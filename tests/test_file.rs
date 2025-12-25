use webtorrent::file::File;

#[tokio::test]
async fn test_file_creation() {
    let file = File::new("test.txt".to_string(), 1024, 0);
    
    assert_eq!(file.name(), "test.txt");
    assert_eq!(file.path(), "test.txt");
    assert_eq!(file.length(), 1024);
    assert_eq!(file.offset(), 0);
}

#[tokio::test]
async fn test_file_with_path() {
    let file = File::new("path/to/file.txt".to_string(), 2048, 1024);
    
    assert_eq!(file.name(), "file.txt");
    assert_eq!(file.path(), "path/to/file.txt");
    assert_eq!(file.length(), 2048);
    assert_eq!(file.offset(), 1024);
}

#[tokio::test]
async fn test_file_start_piece() {
    let file = File::new("test.txt".to_string(), 16384, 0);
    
    assert_eq!(file.start_piece(16384), 0);
    assert_eq!(file.start_piece(8192), 0);
}

#[tokio::test]
async fn test_file_end_piece() {
    let file = File::new("test.txt".to_string(), 16384, 0);
    
    assert_eq!(file.end_piece(16384), 0);
    assert_eq!(file.end_piece(8192), 1);
}

#[tokio::test]
async fn test_file_includes_piece() {
    let file = File::new("test.txt".to_string(), 32768, 0);
    
    assert!(file.includes_piece(0, 16384));
    assert!(file.includes_piece(1, 16384));
    assert!(!file.includes_piece(2, 16384));
}

#[tokio::test]
async fn test_file_done() {
    let file = File::new("test.txt".to_string(), 1024, 0);
    
    assert!(!file.done().await);
    
    file.set_done(true).await;
    assert!(file.done().await);
    
    file.set_done(false).await;
    assert!(!file.done().await);
}

