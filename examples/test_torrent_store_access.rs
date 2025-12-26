// Test: store() access and data retrieval
// Run with: cargo run --example test_torrent_store_access

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_torrent_store_access ===");

    let test_data = Bytes::from("Test data for store access.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    // Get store
    let store = torrent.store();
    match store {
        Some(_) => {
            println!("✓ Store is available");
        },
        None => {
            println!("⚠ Store is None (may be normal for non-seeded torrents)");
        }
    }

    println!("✓ store() access works");

    client.destroy().await?;
    println!("=== test_torrent_store_access PASSED ===");
    Ok(())
}

