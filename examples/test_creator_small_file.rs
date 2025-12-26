// Test: File smaller than piece_length
// Run with: cargo run --example test_creator_small_file

use webtorrent::TorrentCreator;
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_creator_small_file ===");

    // Create a file smaller than default piece length (16KB)
    let small_data = Bytes::from("Small file content");
    println!("Creating torrent for small file ({} bytes, piece length: 16KB)...", small_data.len());

    let creator = TorrentCreator::new(); // Default piece length is 16KB
    let (torrent_file, info_hash) = creator.create_from_data("small_file.txt".to_string(), small_data.clone()).await?;
    
    println!("✓ Torrent created for small file");
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  File size: {} bytes", small_data.len());
    println!("  Torrent file size: {} bytes", torrent_file.len());

    println!("=== test_creator_small_file PASSED ===");
    Ok(())
}

