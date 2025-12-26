// Test: All features disabled
// Run with: cargo run --example test_client_no_features

use webtorrent::{WebTorrent, WebTorrentOptions};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_client_no_features ===");

    // Test: All features disabled
    let mut options = WebTorrentOptions::default();
    options.utp = false;
    options.nat_upnp = false;
    options.nat_pmp = false;
    options.lsd = false;
    options.ut_pex = false;
    options.web_seeds = false;
    
    println!("Creating client with all features disabled...");
    let client = WebTorrent::new(options).await?;
    println!("✓ Client created successfully");

    let peer_id = client.peer_id();
    println!("  Peer ID: {}", hex::encode(peer_id));
    
    let address = client.address().await;
    match address {
        Some((ip, port)) => println!("  Listening on {}:{}", ip, port),
        None => println!("  Not listening yet"),
    }

    println!("✓ Client works with all features disabled");

    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_client_no_features PASSED ===");
    Ok(())
}

