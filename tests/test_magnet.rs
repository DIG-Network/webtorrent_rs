use webtorrent::magnet::MagnetUri;

#[tokio::test]
async fn test_parse_simple_magnet() {
    let uri = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567";
    let magnet = MagnetUri::parse(uri).unwrap();
    
    assert_eq!(magnet.info_hash[0], 0x01);
    assert_eq!(magnet.info_hash[1], 0x23);
}

#[tokio::test]
async fn test_parse_magnet_with_display_name() {
    let uri = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=test+file";
    let magnet = MagnetUri::parse(uri).unwrap();
    
    assert_eq!(magnet.display_name, Some("test file".to_string()));
}

#[tokio::test]
async fn test_parse_magnet_with_trackers() {
    let uri = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&tr=http://tracker1.com&tr=http://tracker2.com";
    let magnet = MagnetUri::parse(uri).unwrap();
    
    assert_eq!(magnet.trackers.len(), 2);
    assert!(magnet.trackers.contains(&"http://tracker1.com".to_string()));
    assert!(magnet.trackers.contains(&"http://tracker2.com".to_string()));
}

#[tokio::test]
async fn test_parse_magnet_with_exact_length() {
    let uri = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&xl=1048576";
    let magnet = MagnetUri::parse(uri).unwrap();
    
    assert_eq!(magnet.exact_length, Some(1048576));
}

#[tokio::test]
async fn test_parse_invalid_magnet() {
    let uri = "not a magnet uri";
    assert!(MagnetUri::parse(uri).is_err());
}

#[tokio::test]
async fn test_parse_magnet_missing_hash() {
    let uri = "magnet:?dn=test";
    assert!(MagnetUri::parse(uri).is_err());
}

#[tokio::test]
async fn test_magnet_to_string() {
    let magnet = MagnetUri {
        info_hash: [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23,
                    0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67],
        display_name: Some("test file".to_string()),
        trackers: vec!["http://tracker.com".to_string()],
        exact_length: Some(1024),
        exact_source: None,
        keywords: vec![],
        acceptable_source: vec![],
    };
    
    let uri = magnet.to_string();
    assert!(uri.starts_with("magnet:?"));
    assert!(uri.contains("xt=urn:btih:"));
    assert!(uri.contains("dn="));
    assert!(uri.contains("tr="));
    assert!(uri.contains("xl=1024"));
}

