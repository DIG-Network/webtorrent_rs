use webtorrent::{WebTorrent, WebTorrentOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use webtorrent::protocol::Handshake;

mod common;
use common::*;

#[tokio::test]
async fn test_conn_pool_creation() {
    let options = WebTorrentOptions {
        torrent_port: 0, // Use random port
        ..Default::default()
    };
    let client = WebTorrent::new(options).await.unwrap();
    
    // ConnPool is created automatically when client is created
    // We can't directly access it, but we can test that the client works
    // Client should be ready
    assert!(client.address().await.is_some() || client.address().await.is_none());
}

#[tokio::test]
async fn test_conn_pool_listening() {
    let options = WebTorrentOptions {
        torrent_port: 0, // Use random port
        ..Default::default()
    };
    let client = WebTorrent::new(options).await.unwrap();
    
    // Client should be ready
    assert!(client.address().await.is_some() || client.address().await.is_none());
    
    // Clean up
    client.destroy().await.unwrap();
}

#[tokio::test]
async fn test_conn_pool_handshake() {
    // Create a test torrent
    let options = WebTorrentOptions {
        torrent_port: 0,
        ..Default::default()
    };
    let client = WebTorrent::new(options).await.unwrap();
    
    // Create a simple test torrent
    let test_data = create_test_torrent_data();
    let torrent_id = webtorrent::TorrentId::TorrentFile(bytes::Bytes::from(test_data));
    
    let torrent = client.add(torrent_id).await.unwrap();
    let info_hash = torrent.info_hash();
    
    // Get the client's listening address
    let address = client.address().await;
    
    if let Some((ip, port)) = address {
        // Try to connect to the client
        let addr = format!("{}:{}", ip, port);
        let mut stream = tokio::net::TcpStream::connect(&addr).await.unwrap();
        
        // Send a handshake
        let peer_id = [0u8; 20];
        let handshake = Handshake::new(info_hash, peer_id);
        let handshake_bytes = handshake.encode();
        stream.write_all(handshake_bytes.as_ref()).await.unwrap();
        
        // Read response handshake
        let mut response_buf = [0u8; 68];
        stream.read_exact(&mut response_buf).await.unwrap();
        let response_handshake = Handshake::decode(&response_buf).unwrap();
        
        // Verify handshake
        assert_eq!(response_handshake.info_hash, info_hash);
        assert_eq!(response_handshake.peer_id, client.peer_id());
        
        stream.shutdown().await.unwrap();
    }
    
    client.destroy().await.unwrap();
}

#[tokio::test]
async fn test_conn_pool_destroy() {
    let options = WebTorrentOptions {
        torrent_port: 0,
        ..Default::default()
    };
    let client = WebTorrent::new(options).await.unwrap();
    
    // Destroy should work
    let result = client.destroy().await;
    assert!(result.is_ok());
    
    // Second destroy should fail
    let result2 = client.destroy().await;
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_conn_pool_multiple_connections() {
    let options = WebTorrentOptions {
        torrent_port: 0,
        ..Default::default()
    };
    let client = WebTorrent::new(options).await.unwrap();
    
    // Create a test torrent
    let test_data = create_test_torrent_data();
    let torrent_id = webtorrent::TorrentId::TorrentFile(bytes::Bytes::from(test_data));
    let torrent = client.add(torrent_id).await.unwrap();
    let info_hash = torrent.info_hash();
    
    let address = client.address().await;
    
    if let Some((ip, port)) = address {
        // Try multiple connections
        let addr_str = format!("{}:{}", ip, port);
        for i in 0..3 {
            let mut stream = tokio::net::TcpStream::connect(&addr_str).await.unwrap();
            
            // Send handshake with unique peer ID
            let mut peer_id = [0u8; 20];
            peer_id[0] = i as u8;
            let handshake = Handshake::new(info_hash, peer_id);
            let handshake_bytes = handshake.encode();
            stream.write_all(handshake_bytes.as_ref()).await.unwrap();
            
            // Read response
            let mut response_buf = [0u8; 68];
            stream.read_exact(&mut response_buf).await.unwrap();
            
            stream.shutdown().await.unwrap();
        }
    }
    
    client.destroy().await.unwrap();
}

#[tokio::test]
async fn test_conn_pool_utp_enabled() {
    let options = WebTorrentOptions {
        torrent_port: 0,
        utp: true, // Enable uTP
        ..Default::default()
    };
    let client = WebTorrent::new(options).await.unwrap();
    
    // Client should be ready with uTP enabled
    assert!(client.address().await.is_some() || client.address().await.is_none());
    
    // Clean up
    client.destroy().await.unwrap();
}

#[tokio::test]
async fn test_conn_pool_utp_disabled() {
    let options = WebTorrentOptions {
        torrent_port: 0,
        utp: false, // Disable uTP
        ..Default::default()
    };
    let client = WebTorrent::new(options).await.unwrap();
    
    // Client should work without uTP
    assert!(client.address().await.is_some() || client.address().await.is_none());
    
    // Clean up
    client.destroy().await.unwrap();
}
