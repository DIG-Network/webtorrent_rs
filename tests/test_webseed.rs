use webtorrent::webseed::WebSeedConn;

#[tokio::test]
async fn test_webseed_new() {
    let piece_length = 16384;
    let files = vec![
        ("test.txt".to_string(), 1000, 0),
    ];
    
    let webseed = WebSeedConn::new("http://example.com".to_string(), piece_length, files).unwrap();
    assert_eq!(webseed.url(), "http://example.com");
}

#[tokio::test]
async fn test_webseed_invalid_url() {
    let piece_length = 16384;
    let files = vec![
        ("test.txt".to_string(), 1000, 0),
    ];
    
    let result = WebSeedConn::new("not-a-url".to_string(), piece_length, files);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_webseed_init() {
    let piece_length = 16384;
    let files = vec![
        ("test.txt".to_string(), 1000, 0),
    ];
    
    let webseed = WebSeedConn::new("http://example.com".to_string(), piece_length, files).unwrap();
    // Init would make HTTP request - skip for unit test
    // webseed.init().await.unwrap();
}

#[tokio::test]
async fn test_webseed_multiple_files() {
    let piece_length = 16384;
    let files = vec![
        ("file1.txt".to_string(), 1000, 0),
        ("file2.txt".to_string(), 2000, 1000),
    ];
    
    let webseed = WebSeedConn::new("http://example.com".to_string(), piece_length, files).unwrap();
    assert_eq!(webseed.url(), "http://example.com");
}

