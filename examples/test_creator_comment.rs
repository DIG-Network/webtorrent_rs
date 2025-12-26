// Test: With and without comment
// Run with: cargo run --example test_creator_comment

use webtorrent::TorrentCreator;
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_creator_comment ===");

    let test_data = Bytes::from("Test data for comment testing.");

    // Test: Without comment
    println!("[1/2] Testing without comment...");
    let creator1 = TorrentCreator::new();
    let (torrent_file1, info_hash1) = creator1.create_from_data("test1.txt".to_string(), test_data.clone()).await?;
    println!("  ✓ Created without comment");
    println!("    Info hash: {}", hex::encode(info_hash1));

    // Test: With comment
    println!("[2/2] Testing with comment...");
    let creator2 = TorrentCreator::new().with_comment("This is a test torrent comment".to_string());
    let (torrent_file2, info_hash2) = creator2.create_from_data("test2.txt".to_string(), test_data.clone()).await?;
    println!("  ✓ Created with comment");
    println!("    Info hash: {}", hex::encode(info_hash2));

    println!("=== test_creator_comment PASSED ===");
    Ok(())
}

