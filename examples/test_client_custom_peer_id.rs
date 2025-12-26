// Test: Custom peer_id and node_id
// Run with: cargo run --example test_client_custom_peer_id

use webtorrent::{WebTorrent, WebTorrentOptions};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_client_custom_peer_id ===");

    // Test: Custom peer_id and node_id
    let mut options = WebTorrentOptions::default();
    let custom_peer_id = {
        let mut id = [0u8; 20];
        id[0..3].copy_from_slice(b"-WT");
        id[3..7].copy_from_slice(b"TEST");
        id[7] = b'-';
        // Rest are zeros for testing
        id
    };
    let custom_node_id = {
        let mut id = [0u8; 20];
        id[0..4].copy_from_slice(b"NODE");
        // Rest are zeros
        id
    };
    
    options.peer_id = Some(custom_peer_id);
    options.node_id = Some(custom_node_id);
    
    println!("Creating client with custom peer_id and node_id...");
    let client = WebTorrent::new(options).await?;
    println!("✓ Client created successfully");

    let peer_id = client.peer_id();
    println!("  Peer ID: {}", hex::encode(peer_id));
    assert_eq!(peer_id, custom_peer_id, "Peer ID should match custom value");
    println!("✓ Peer ID matches custom value");

    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_client_custom_peer_id PASSED ===");
    Ok(())
}

