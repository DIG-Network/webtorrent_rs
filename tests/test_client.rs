use webtorrent::{WebTorrent, WebTorrentOptions};

#[tokio::test]
async fn test_client_creation() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await;
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_client_with_custom_options() {
    let options = WebTorrentOptions {
        torrent_port: 6881,
        dht_port: 6882,
        max_conns: 100,
        utp: true,
        nat_upnp: true,
        nat_pmp: true,
        ..Default::default()
    };
    
    let client = WebTorrent::new(options).await;
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_client_destroy() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    let result = client.destroy().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_client_destroy_twice() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    let result1 = client.destroy().await;
    assert!(result1.is_ok());
    
    // Second destroy should return error
    let result2 = client.destroy().await;
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_client_progress_empty() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    let progress = client.progress().await;
    assert_eq!(progress, 1.0); // No torrents = 100% complete
}

#[tokio::test]
async fn test_client_ratio_empty() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    let ratio = client.ratio().await;
    assert_eq!(ratio, 0.0); // No downloads = 0 ratio
}

#[tokio::test]
async fn test_client_address_not_listening() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    let address = client.address().await;
    assert!(address.is_none());
}

#[tokio::test]
async fn test_client_download_speed() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    // Initially should be 0
    let speed = client.download_speed().await;
    assert_eq!(speed, 0);
    
    // Speed tracking is tested through the public API
    // The actual recording happens internally when data is transferred
}

#[tokio::test]
async fn test_client_upload_speed() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    // Initially should be 0
    let speed = client.upload_speed().await;
    assert_eq!(speed, 0);
    
    // Speed tracking is tested through the public API
    // The actual recording happens internally when data is transferred
}

#[tokio::test]
async fn test_client_throttle_download() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    // Set download throttle
    client.throttle_download(Some(1000)).await; // 1000 bytes/sec
    
    // Throttling is applied internally - test that it doesn't error
    // The actual throttling happens when data is transferred
}

#[tokio::test]
async fn test_client_throttle_upload() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    // Set upload throttle
    client.throttle_upload(Some(2000)).await; // 2000 bytes/sec
    
    // Throttling is applied internally - test that it doesn't error
    // The actual throttling happens when data is transferred
}

#[tokio::test]
async fn test_client_throttle_unlimited() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    // Set unlimited throttle (None)
    client.throttle_download(None).await;
    client.throttle_upload(None).await;
    
    // Should not error when setting to unlimited
}

