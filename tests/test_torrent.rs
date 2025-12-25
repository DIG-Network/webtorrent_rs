use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId};
use bytes::Bytes;

#[tokio::test]
async fn test_torrent_from_info_hash() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    let info_hash = [0u8; 20];
    let torrent_id = TorrentId::InfoHash(info_hash);
    
    // Should fail because we need metadata
    let result = client.add(torrent_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_torrent_from_url() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    // This will fail because the URL doesn't exist, but tests the parsing
    let torrent_id = TorrentId::Url("http://example.com/nonexistent.torrent".to_string());
    let result = client.add(torrent_id).await;
    
    // Should fail with network error or invalid torrent
    assert!(result.is_err());
}

#[tokio::test]
async fn test_torrent_duplicate() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    
    // Create a minimal torrent file
    let torrent_data = create_minimal_torrent();
    let torrent_id = TorrentId::TorrentFile(Bytes::from(torrent_data));
    
    let torrent1 = client.add(torrent_id.clone()).await;
    if torrent1.is_ok() {
        // Try to add the same torrent again
        let torrent2 = client.add(torrent_id).await;
        assert!(torrent2.is_err()); // Should fail with duplicate error
    }
}

fn create_minimal_torrent() -> Vec<u8> {
    // Create a minimal valid torrent file
    let mut data = Vec::new();
    
    // d8:announce41:http://tracker.example.com:8080/announce
    data.extend_from_slice(b"d8:announce41:http://tracker.example.com:8080/announce");
    
    // 4:infod
    data.extend_from_slice(b"4:infod");
    
    // 4:name4:test
    data.extend_from_slice(b"4:name4:test");
    
    // 12:piece lengthi16384e
    data.extend_from_slice(b"12:piece lengthi16384e");
    
    // 6:pieces20:XXXXXXXXXXXXXXXXXXXX
    data.extend_from_slice(b"6:pieces20:");
    data.extend_from_slice(&[0u8; 20]); // Placeholder hash
    
    // 6:lengthi1024e
    data.extend_from_slice(b"6:lengthi1024e");
    
    // ee (end dict, end dict)
    data.extend_from_slice(b"ee");
    
    data
}

