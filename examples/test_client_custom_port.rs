// Test: Custom torrent_port and dht_port
// Run with: cargo run --example test_client_custom_port

use webtorrent::{WebTorrent, WebTorrentOptions};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_client_custom_port ===");

    // Test: Custom ports
    let mut options = WebTorrentOptions::default();
    options.torrent_port = 6882;
    options.dht_port = 6883;
    
    println!("Creating client with custom ports (torrent: {}, dht: {})...", 
             options.torrent_port, options.dht_port);
    let client = WebTorrent::new(options).await?;
    println!("✓ Client created successfully");

    let address = client.address().await;
    match address {
        Some((ip, port)) => {
            println!("  Listening on {}:{}", ip, port);
            // Note: Port might be different if 6882 was already in use
        },
        None => println!("  Not listening yet"),
    }

    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_client_custom_port PASSED ===");
    Ok(())
}

