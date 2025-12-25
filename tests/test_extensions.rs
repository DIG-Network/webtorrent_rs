use webtorrent::extensions::{UtMetadata, UtPex, ExtensionProtocol};

#[tokio::test]
async fn test_ut_metadata_new() {
    let ut_metadata = UtMetadata::new();
    
    assert!(ut_metadata.get_metadata().is_none());
}

#[tokio::test]
async fn test_ut_metadata_set_get() {
    let mut ut_metadata = UtMetadata::new();
    
    let metadata = bytes::Bytes::from(vec![1, 2, 3, 4, 5]);
    ut_metadata.set_metadata(metadata.clone());
    
    assert_eq!(ut_metadata.get_metadata(), Some(&metadata));
}

#[tokio::test]
async fn test_ut_metadata_handle_request() {
    let mut ut_metadata = UtMetadata::new();
    
    let metadata = bytes::Bytes::from(vec![0u8; 16384 * 2]); // 2 pieces
    ut_metadata.set_metadata(metadata);
    
    let piece0 = ut_metadata.handle_request(0).unwrap();
    assert!(piece0.is_some());
    assert_eq!(piece0.unwrap().len(), 16384);
    
    let piece1 = ut_metadata.handle_request(1).unwrap();
    assert!(piece1.is_some());
    assert_eq!(piece1.unwrap().len(), 16384);
}

#[tokio::test]
async fn test_ut_pex_new() {
    let ut_pex = UtPex::new();
    
    assert_eq!(ut_pex.get_added().len(), 0);
    assert_eq!(ut_pex.get_dropped().len(), 0);
}

#[tokio::test]
async fn test_ut_pex_add_peer() {
    let mut ut_pex = UtPex::new();
    
    ut_pex.add_peer("192.168.1.1".to_string(), 6881);
    ut_pex.add_peer("192.168.1.2".to_string(), 6882);
    
    assert_eq!(ut_pex.get_added().len(), 2);
}

#[tokio::test]
async fn test_ut_pex_drop_peer() {
    let mut ut_pex = UtPex::new();
    
    ut_pex.add_peer("192.168.1.1".to_string(), 6881);
    ut_pex.drop_peer("192.168.1.1".to_string(), 6881);
    
    assert_eq!(ut_pex.get_added().len(), 1);
    assert_eq!(ut_pex.get_dropped().len(), 1);
}

#[tokio::test]
async fn test_ut_pex_encode_decode() {
    let mut ut_pex = UtPex::new();
    
    ut_pex.add_peer("192.168.1.1".to_string(), 6881);
    ut_pex.add_peer("192.168.1.2".to_string(), 6882);
    
    let encoded = ut_pex.encode();
    let decoded = UtPex::decode(&encoded).unwrap();
    
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0].0, "192.168.1.1");
    assert_eq!(decoded[0].1, 6881);
}

#[tokio::test]
async fn test_ut_pex_reset() {
    let mut ut_pex = UtPex::new();
    
    ut_pex.add_peer("192.168.1.1".to_string(), 6881);
    ut_pex.reset();
    
    assert_eq!(ut_pex.get_added().len(), 0);
    assert_eq!(ut_pex.get_dropped().len(), 0);
}

#[tokio::test]
async fn test_extension_protocol_register() {
    let mut ext = ExtensionProtocol::new();
    
    ext.register_extension(1, "ut_metadata".to_string());
    ext.register_extension(2, "ut_pex".to_string());
    
    assert_eq!(ext.get_extension_id("ut_metadata"), Some(1));
    assert_eq!(ext.get_extension_id("ut_pex"), Some(2));
    assert_eq!(ext.get_extension_id("nonexistent"), None);
}

