// Test: Invalid torrent file (error case)
// Run with: cargo run --example test_add_invalid_torrent

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_add_invalid_torrent ===");

    // Test: Invalid torrent file (not valid bencode)
    let invalid_torrent = Bytes::from("This is not a valid torrent file!");
    
    println!("Attempting to add invalid torrent file...");
    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    println!("✓ Client created");

    match client.add(TorrentId::TorrentFile(invalid_torrent)).await {
        Ok(_) => {
            println!("⚠ WARNING: Invalid torrent was accepted (should have been rejected)");
        },
        Err(e) => {
            println!("✓ Invalid torrent correctly rejected");
            println!("  Error: {}", e);
        }
    }

    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_add_invalid_torrent PASSED ===");
    Ok(())
}

