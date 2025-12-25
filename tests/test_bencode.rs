use webtorrent::bencode_parser::{parse_bencode, BencodeValue};
use bytes::Bytes;

#[tokio::test]
async fn test_parse_integer() {
    let data = b"i42e";
    let (value, _) = parse_bencode(data).unwrap();
    
    assert_eq!(value.as_integer(), Some(42));
}

#[tokio::test]
async fn test_parse_negative_integer() {
    let data = b"i-42e";
    let (value, _) = parse_bencode(data).unwrap();
    
    assert_eq!(value.as_integer(), Some(-42));
}

#[tokio::test]
async fn test_parse_bytes() {
    let data = b"4:test";
    let (value, _) = parse_bencode(data).unwrap();
    
    assert_eq!(value.as_bytes(), Some(&Bytes::from("test")));
    assert_eq!(value.as_string(), Some("test".to_string()));
}

#[tokio::test]
async fn test_parse_list() {
    let data = b"li42e4:teste";
    let (value, _) = parse_bencode(data).unwrap();
    
    let list = value.as_list().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].as_integer(), Some(42));
    assert_eq!(list[1].as_string(), Some("test".to_string()));
}

#[tokio::test]
async fn test_parse_dict() {
    let data = b"d4:name4:test3:agei25ee";
    let (value, _) = parse_bencode(data).unwrap();
    
    let dict = value.as_dict().unwrap();
    assert_eq!(dict.get("name".as_bytes()).unwrap().as_string(), Some("test".to_string()));
    assert_eq!(dict.get("age".as_bytes()).unwrap().as_integer(), Some(25));
}

#[tokio::test]
async fn test_encode_integer() {
    let value = BencodeValue::Integer(42);
    let encoded = value.encode();
    assert_eq!(encoded, Bytes::from("i42e"));
}

#[tokio::test]
async fn test_encode_bytes() {
    let value = BencodeValue::Bytes(Bytes::from("test"));
    let encoded = value.encode();
    assert_eq!(encoded, Bytes::from("4:test"));
}

#[tokio::test]
async fn test_encode_list() {
    let value = BencodeValue::List(vec![
        BencodeValue::Integer(42),
        BencodeValue::Bytes(Bytes::from("test")),
    ]);
    let encoded = value.encode();
    // Order may vary, so we parse it back
    let (parsed, _) = parse_bencode(&encoded).unwrap();
    let list = parsed.as_list().unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn test_encode_dict() {
    let mut dict = std::collections::HashMap::new();
    dict.insert(Bytes::from("name"), BencodeValue::Bytes(Bytes::from("test")));
    dict.insert(Bytes::from("age"), BencodeValue::Integer(25));
    
    let value = BencodeValue::Dict(dict);
    let encoded = value.encode();
    
    // Parse it back to verify
    let (parsed, _) = parse_bencode(&encoded).unwrap();
    let parsed_dict = parsed.as_dict().unwrap();
    assert_eq!(parsed_dict.get("name".as_bytes()).unwrap().as_string(), Some("test".to_string()));
    assert_eq!(parsed_dict.get("age".as_bytes()).unwrap().as_integer(), Some(25));
}

#[tokio::test]
async fn test_round_trip() {
    let original = BencodeValue::Dict({
        let mut dict = std::collections::HashMap::new();
        dict.insert(Bytes::from("name"), BencodeValue::Bytes(Bytes::from("test")));
        dict.insert(Bytes::from("numbers"), BencodeValue::List(vec![
            BencodeValue::Integer(1),
            BencodeValue::Integer(2),
            BencodeValue::Integer(3),
        ]));
        dict
    });
    
    let encoded = original.encode();
    let (decoded, _) = parse_bencode(&encoded).unwrap();
    
    let original_dict = original.as_dict().unwrap();
    let decoded_dict = decoded.as_dict().unwrap();
    
    assert_eq!(
        original_dict.get("name".as_bytes()).unwrap().as_string(),
        decoded_dict.get("name".as_bytes()).unwrap().as_string()
    );
}

