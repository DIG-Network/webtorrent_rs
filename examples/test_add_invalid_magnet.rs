// Test: Invalid magnet URI (error case)
// Run with: cargo run --example test_add_invalid_magnet

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_add_invalid_magnet ===");

    // Test: Invalid magnet URI (not starting with magnet:)
    let invalid_magnet = "not-a-magnet-uri";
    
    println!("Attempting to add invalid magnet URI...");
    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    println!("✓ Client created");

    match client.add(TorrentId::MagnetUri(invalid_magnet.to_string())).await {
        Ok(_) => {
            println!("⚠ WARNING: Invalid magnet URI was accepted (should have been rejected)");
        },
        Err(e) => {
            println!("✓ Invalid magnet URI correctly rejected");
            println!("  Error: {}", e);
        }
    }

    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_add_invalid_magnet PASSED ===");
    Ok(())
}

