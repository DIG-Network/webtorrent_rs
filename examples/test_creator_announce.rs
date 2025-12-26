// Test: Single and multiple announce URLs
// Run with: cargo run --example test_creator_announce

use webtorrent::TorrentCreator;
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_creator_announce ===");

    let test_data = Bytes::from("Test data for announce testing.");

    // Test: Single announce URL
    println!("[1/2] Testing single announce URL...");
    let creator1 = TorrentCreator::new().with_announce(vec![
        "http://tracker.example.com/announce".to_string()
    ]);
    let (torrent_file1, info_hash1) = creator1.create_from_data("test1.txt".to_string(), test_data.clone()).await?;
    println!("  ✓ Created with single announce");
    println!("    Info hash: {}", hex::encode(info_hash1));

    // Test: Multiple announce URLs
    println!("[2/2] Testing multiple announce URLs...");
    let creator2 = TorrentCreator::new().with_announce(vec![
        "http://tracker1.example.com/announce".to_string(),
        "http://tracker2.example.com/announce".to_string(),
        "udp://tracker3.example.com:1337/announce".to_string(),
    ]);
    let (torrent_file2, info_hash2) = creator2.create_from_data("test2.txt".to_string(), test_data.clone()).await?;
    println!("  ✓ Created with multiple announce URLs");
    println!("    Info hash: {}", hex::encode(info_hash2));

    println!("=== test_creator_announce PASSED ===");
    Ok(())
}

