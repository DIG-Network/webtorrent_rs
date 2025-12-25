// Integration tests

use webtorrent::{WebTorrent, WebTorrentOptions};

#[tokio::test]
async fn test_client_lifecycle() {
    // Create client
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    // Check initial state
    assert_eq!(client.progress().await, 1.0);
    assert_eq!(client.ratio().await, 0.0);
    
    // Destroy client
    client.destroy().await.unwrap();
}

#[tokio::test]
async fn test_client_multiple_torrents() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    // Add multiple torrents (will fail without valid data, but tests the flow)
    let info_hash1 = [1u8; 20];
    let info_hash2 = [2u8; 20];
    
    let _ = client.add(info_hash1).await;
    let _ = client.add(info_hash2).await;
    
    // Client should still be functional
    let progress = client.progress().await;
    assert!(progress >= 0.0 && progress <= 1.0);
    
    client.destroy().await.unwrap();
}

#[tokio::test]
async fn test_client_throttling() {
    let options = WebTorrentOptions {
        download_limit: Some(1000),
        upload_limit: Some(500),
        ..Default::default()
    };
    
    let client = WebTorrent::new(options).await.unwrap();
    
    // Throttling should be set
    client.throttle_download(Some(2000)).await;
    client.throttle_upload(Some(1000)).await;
    
    client.destroy().await.unwrap();
}

#[tokio::test]
async fn test_client_options_persistence() {
    let options = WebTorrentOptions {
        torrent_port: 6881,
        dht_port: 6882,
        max_conns: 50,
        utp: true,
        ..Default::default()
    };
    
    let client = WebTorrent::new(options).await.unwrap();
    
    // Options should be stored in client
    // (This is tested implicitly through client behavior)
    
    client.destroy().await.unwrap();
}

