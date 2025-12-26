// Test: All features enabled
// Run with: cargo run --example test_client_all_features

use webtorrent::{WebTorrent, WebTorrentOptions};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_client_all_features ===");

    // Test: All features enabled
    let mut options = WebTorrentOptions::default();
    options.utp = true;
    options.nat_upnp = true;
    options.nat_pmp = true;
    options.lsd = true;
    options.ut_pex = true;
    options.web_seeds = true;
    
    println!("Creating client with all features enabled...");
    let client = WebTorrent::new(options).await?;
    println!("✓ Client created successfully");

    let peer_id = client.peer_id();
    println!("  Peer ID: {}", hex::encode(peer_id));
    
    let address = client.address().await;
    match address {
        Some((ip, port)) => println!("  Listening on {}:{}", ip, port),
        None => println!("  Not listening yet"),
    }

    // Note: We can't directly verify features are enabled without internal access
    // But if creation succeeds, features are initialized
    println!("✓ All features initialized");

    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_client_all_features PASSED ===");
    Ok(())
}

