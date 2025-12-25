use crate::error::{Result, WebTorrentError};
use bytes::Bytes;
use std::collections::HashMap;

/// Simple bencode value
#[derive(Debug, Clone)]
pub enum BencodeValue {
    Integer(i64),
    Bytes(Bytes),
    List(Vec<BencodeValue>),
    Dict(HashMap<Bytes, BencodeValue>),
}

impl BencodeValue {
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            BencodeValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&Bytes> {
        match self {
            BencodeValue::Bytes(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        self.as_bytes()
            .and_then(|b| String::from_utf8(b.to_vec()).ok())
    }

    pub fn as_list(&self) -> Option<&Vec<BencodeValue>> {
        match self {
            BencodeValue::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&HashMap<Bytes, BencodeValue>> {
        match self {
            BencodeValue::Dict(d) => Some(d),
            _ => None,
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&BencodeValue> {
        self.as_dict().and_then(|d| {
            let key_bytes = Bytes::copy_from_slice(key);
            d.get(&key_bytes)
        })
    }

    pub fn encode(&self) -> Bytes {
        let mut result = Vec::new();
        self.encode_into(&mut result);
        Bytes::from(result)
    }

    fn encode_into(&self, buf: &mut Vec<u8>) {
        match self {
            BencodeValue::Integer(i) => {
                buf.push(b'i');
                buf.extend_from_slice(i.to_string().as_bytes());
                buf.push(b'e');
            }
            BencodeValue::Bytes(b) => {
                buf.extend_from_slice(b.len().to_string().as_bytes());
                buf.push(b':');
                buf.extend_from_slice(b);
            }
            BencodeValue::List(l) => {
                buf.push(b'l');
                for item in l {
                    item.encode_into(buf);
                }
                buf.push(b'e');
            }
            BencodeValue::Dict(d) => {
                buf.push(b'd');
                let mut keys: Vec<_> = d.keys().collect();
                keys.sort();
                for key in keys {
                    BencodeValue::Bytes(key.clone()).encode_into(buf);
                    d[key].encode_into(buf);
                }
                buf.push(b'e');
            }
        }
    }
}

/// Parse bencode from bytes
pub fn parse_bencode(data: &[u8]) -> Result<(BencodeValue, usize)> {
    let mut pos = 0;
    let value = parse_value(data, &mut pos)?;
    Ok((value, pos))
}

fn parse_value(data: &[u8], pos: &mut usize) -> Result<BencodeValue> {
    if *pos >= data.len() {
        return Err(WebTorrentError::Bencode(
            "Unexpected end of data".to_string(),
        ));
    }

    match data[*pos] {
        b'i' => parse_integer(data, pos),
        b'l' => parse_list(data, pos),
        b'd' => parse_dict(data, pos),
        b'0'..=b'9' => parse_bytes(data, pos),
        _ => Err(WebTorrentError::Bencode(format!(
            "Unexpected byte: {}",
            data[*pos]
        ))),
    }
}

fn parse_integer(data: &[u8], pos: &mut usize) -> Result<BencodeValue> {
    *pos += 1; // skip 'i'
    let start = *pos;
    while *pos < data.len() && data[*pos] != b'e' {
        *pos += 1;
    }
    if *pos >= data.len() {
        return Err(WebTorrentError::Bencode(
            "Integer not terminated".to_string(),
        ));
    }
    let num_str = String::from_utf8_lossy(&data[start..*pos]);
    let num = num_str
        .parse::<i64>()
        .map_err(|e| WebTorrentError::Bencode(format!("Invalid integer: {}", e)))?;
    *pos += 1; // skip 'e'
    Ok(BencodeValue::Integer(num))
}

fn parse_bytes(data: &[u8], pos: &mut usize) -> Result<BencodeValue> {
    let start = *pos;
    while *pos < data.len() && data[*pos] != b':' {
        *pos += 1;
    }
    if *pos >= data.len() {
        return Err(WebTorrentError::Bencode(
            "Bytes length not terminated".to_string(),
        ));
    }
    let len_str = String::from_utf8_lossy(&data[start..*pos]);
    let len = len_str
        .parse::<usize>()
        .map_err(|e| WebTorrentError::Bencode(format!("Invalid bytes length: {}", e)))?;
    *pos += 1; // skip ':'
    if *pos + len > data.len() {
        return Err(WebTorrentError::Bencode(
            "Bytes length exceeds data".to_string(),
        ));
    }
    let bytes = Bytes::copy_from_slice(&data[*pos..*pos + len]);
    *pos += len;
    Ok(BencodeValue::Bytes(bytes))
}

fn parse_list(data: &[u8], pos: &mut usize) -> Result<BencodeValue> {
    *pos += 1; // skip 'l'
    let mut list = Vec::new();
    while *pos < data.len() && data[*pos] != b'e' {
        list.push(parse_value(data, pos)?);
    }
    if *pos >= data.len() {
        return Err(WebTorrentError::Bencode("List not terminated".to_string()));
    }
    *pos += 1; // skip 'e'
    Ok(BencodeValue::List(list))
}

fn parse_dict(data: &[u8], pos: &mut usize) -> Result<BencodeValue> {
    *pos += 1; // skip 'd'
    let mut dict = HashMap::new();
    while *pos < data.len() && data[*pos] != b'e' {
        let key = match parse_value(data, pos)? {
            BencodeValue::Bytes(b) => b,
            _ => {
                return Err(WebTorrentError::Bencode(
                    "Dictionary key must be bytes".to_string(),
                ))
            }
        };
        let value = parse_value(data, pos)?;
        dict.insert(key, value);
    }
    if *pos >= data.len() {
        return Err(WebTorrentError::Bencode(
            "Dictionary not terminated".to_string(),
        ));
    }
    *pos += 1; // skip 'e'
    Ok(BencodeValue::Dict(dict))
}
