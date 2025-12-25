use webtorrent::protocol::{Handshake, MessageType};

#[tokio::test]
async fn test_handshake_encode() {
    let info_hash = [0u8; 20];
    let peer_id = [1u8; 20];
    
    let handshake = Handshake::new(info_hash, peer_id);
    let encoded = handshake.encode();
    
    // Should start with protocol length (19 = "BitTorrent protocol".len())
    assert_eq!(encoded[0], 19);
    assert!(encoded.len() >= 68); // Minimum handshake size
}

#[tokio::test]
async fn test_handshake_decode() {
    let info_hash = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a,
                     0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14];
    let peer_id = [0x20; 20];
    
    let handshake = Handshake::new(info_hash, peer_id);
    let encoded = handshake.encode();
    
    let decoded = Handshake::decode(&encoded).unwrap();
    
    assert_eq!(decoded.info_hash, info_hash);
    assert_eq!(decoded.peer_id, peer_id);
}

#[tokio::test]
async fn test_handshake_round_trip() {
    let info_hash = [0x42; 20];
    let peer_id = [0x99; 20];
    
    let original = Handshake::new(info_hash, peer_id);
    let encoded = original.encode();
    let decoded = Handshake::decode(&encoded).unwrap();
    
    assert_eq!(decoded.info_hash, info_hash);
    assert_eq!(decoded.peer_id, peer_id);
    assert_eq!(decoded.protocol, "BitTorrent protocol");
}

#[tokio::test]
async fn test_handshake_invalid_short() {
    let data = b"i19e";
    let result = Handshake::decode(data);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_message_types() {
    assert_eq!(MessageType::Choke as u8, 0);
    assert_eq!(MessageType::Unchoke as u8, 1);
    assert_eq!(MessageType::Interested as u8, 2);
    assert_eq!(MessageType::NotInterested as u8, 3);
    assert_eq!(MessageType::Have as u8, 4);
    assert_eq!(MessageType::Bitfield as u8, 5);
    assert_eq!(MessageType::Request as u8, 6);
    assert_eq!(MessageType::Piece as u8, 7);
    assert_eq!(MessageType::Cancel as u8, 8);
    assert_eq!(MessageType::Port as u8, 9);
    assert_eq!(MessageType::Extended as u8, 20);
}

