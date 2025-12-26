// Test: Empty file (0 bytes)
// Run with: cargo run --example test_creator_empty_file

use webtorrent::TorrentCreator;
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_creator_empty_file ===");

    // Create an empty file
    let empty_data = Bytes::new();
    println!("Creating torrent for empty file (0 bytes)...");

    let creator = TorrentCreator::new();
    let (torrent_file, info_hash) = creator.create_from_data("empty_file.txt".to_string(), empty_data).await?;
    
    println!("✓ Torrent created for empty file");
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  Torrent file size: {} bytes", torrent_file.len());

    println!("=== test_creator_empty_file PASSED ===");
    Ok(())
}

