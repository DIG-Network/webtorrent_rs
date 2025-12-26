// Test: Chaining all builder methods
// Run with: cargo run --example test_creator_chained

use webtorrent::TorrentCreator;
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_creator_chained ===");

    let test_data = Bytes::from("Test data for chained builder methods.");

    // Test: Chain all builder methods
    let creator = TorrentCreator::new()
        .with_piece_length(32 * 1024)
        .with_announce(vec![
            "http://tracker.example.com/announce".to_string()
        ])
        .with_comment("Chained builder test".to_string());

    let (torrent_file, info_hash) = creator.create_from_data("chained_test.txt".to_string(), test_data).await?;
    
    println!("✓ Torrent created with all chained methods");
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  Torrent file size: {} bytes", torrent_file.len());

    println!("=== test_creator_chained PASSED ===");
    Ok(())
}

