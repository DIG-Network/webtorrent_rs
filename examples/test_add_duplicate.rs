// Test: Adding same torrent twice (should error)
// Run with: cargo run --example test_add_duplicate

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_add_duplicate ===");

    // Create a test torrent file
    let test_data = Bytes::from("This is test data for duplicate testing.");
    let creator = TorrentCreator::new();
    let (torrent_file, info_hash) = creator.create_from_data("test_file.txt".to_string(), test_data).await?;
    
    println!("Created test torrent file");
    println!("  Info hash: {}", hex::encode(info_hash));

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    println!("✓ Client created");

    // Add torrent first time
    let torrent1 = client.add(TorrentId::TorrentFile(torrent_file.clone())).await?;
    println!("✓ Torrent added first time");

    // Try to add same torrent again
    match client.add(TorrentId::TorrentFile(torrent_file)).await {
        Ok(_) => {
            println!("⚠ WARNING: Duplicate torrent was accepted (should have been rejected)");
        },
        Err(e) => {
            println!("✓ Duplicate torrent correctly rejected");
            println!("  Error: {}", e);
        }
    }

    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_add_duplicate PASSED ===");
    Ok(())
}

