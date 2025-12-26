use webtorrent::webrtc::{WebRtcManager, WebRtcConnection};
use webtorrent::{WebTorrent, WebTorrentOptions};
use std::sync::Arc;

#[tokio::test]
async fn test_webrtc_manager_new() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    let manager = WebRtcManager::new(Arc::new(client));
    
    // Manager should be created
    assert!(true);
}

#[tokio::test]
async fn test_webrtc_create_offer() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    let manager = WebRtcManager::new(Arc::new(client));
    
    let info_hash = [0u8; 20];
    let offer = manager.create_offer("test_peer", info_hash).await;
    
    // Offer should be generated (even if placeholder)
    assert!(offer.is_ok());
    assert!(!offer.unwrap().is_empty());
}

#[tokio::test]
async fn test_webrtc_handle_offer() {
    let options = WebTorrentOptions::default();
    let client = WebTorrent::new(options).await.unwrap();
    let manager = WebRtcManager::new(Arc::new(client));
    
    let info_hash = [0u8; 20];
    let offer = manager.create_offer("test_peer", info_hash).await.unwrap();
    
    let answer = manager.handle_offer(&offer, "test_peer", info_hash).await;
    
    // Answer should be generated
    assert!(answer.is_ok());
}

#[tokio::test]
async fn test_webrtc_connection_structure() {
    // Test WebRtcConnection structure
    let connection = WebRtcConnection {
        peer_id: "test".to_string(),
        connection_id: "test_conn".to_string(),
        offer: None,
        answer: None,
        ice_candidates: Vec::new(),
        connected: Arc::new(tokio::sync::RwLock::new(false)),
        destroyed: Arc::new(tokio::sync::RwLock::new(false)),
    };
    
    assert_eq!(connection.peer_id, "test");
    assert_eq!(connection.connection_id, "test_conn");
}

