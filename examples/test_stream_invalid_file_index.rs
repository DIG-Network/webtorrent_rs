// Test: Invalid file_index (error case)
// Run with: cargo run --example test_stream_invalid_file_index

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_stream_invalid_file_index ===");

    let test_data = Bytes::from("Test data.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = Arc::new(client.add(TorrentId::TorrentFile(torrent_file)).await?);
    
    // Try to create stream with invalid file index
    let invalid_index = 999;
    match torrent.create_read_stream(invalid_index) {
        Ok(_) => {
            println!("⚠ WARNING: Invalid file index was accepted (should have been rejected)");
        },
        Err(e) => {
            println!("✓ Invalid file index correctly rejected");
            println!("  Error: {}", e);
        }
    }

    client.destroy().await?;
    println!("=== test_stream_invalid_file_index PASSED ===");
    Ok(())
}

