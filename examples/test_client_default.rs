// Test: Default WebTorrentOptions
// Run with: cargo run --example test_client_default

use webtorrent::{WebTorrent, WebTorrentOptions};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_client_default ===");

    // Test: Initialize WebTorrent with default options
    let options = WebTorrentOptions::default();
    println!("Creating client with default options...");
    let client = WebTorrent::new(options).await?;
    println!("✓ Client created successfully");

    // Verify default options
    let peer_id = client.peer_id();
    println!("  Peer ID: {}", hex::encode(peer_id));
    
    let address = client.address().await;
    match address {
        Some((ip, port)) => println!("  Listening on {}:{}", ip, port),
        None => println!("  Not listening yet"),
    }

    // Clean up
    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_client_default PASSED ===");
    Ok(())
}

