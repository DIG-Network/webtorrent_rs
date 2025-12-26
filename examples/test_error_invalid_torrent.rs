// Test: Invalid torrent file handling
// Run with: cargo run --example test_error_invalid_torrent

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_error_invalid_torrent ===");

    // Invalid torrent data (not valid bencode)
    let invalid_data = Bytes::from("This is not a valid torrent file!");

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;

    match client.add(TorrentId::TorrentFile(invalid_data)).await {
        Ok(_) => {
            println!("⚠ WARNING: Invalid torrent was accepted (should have been rejected)");
        },
        Err(e) => {
            println!("✓ Invalid torrent correctly rejected");
            println!("  Error type: {:?}", e);
        }
    }

    client.destroy().await?;
    println!("=== test_error_invalid_torrent PASSED ===");
    Ok(())
}

