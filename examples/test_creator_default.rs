// Test: TorrentCreator::new() with defaults
// Run with: cargo run --example test_creator_default

use webtorrent::TorrentCreator;
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_creator_default ===");

    // Test: Default TorrentCreator
    let creator = TorrentCreator::new();
    println!("✓ TorrentCreator created with defaults");

    // Create a test torrent
    let test_data = Bytes::from("This is a test file for default creator.");
    let (torrent_file, info_hash) = creator.create_from_data("test_file.txt".to_string(), test_data.clone()).await?;
    
    println!("✓ Torrent created");
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  Torrent file size: {} bytes", torrent_file.len());
    println!("  Data size: {} bytes", test_data.len());

    println!("=== test_creator_default PASSED ===");
    Ok(())
}

