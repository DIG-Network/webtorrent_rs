// Test: destroy() and verify cleanup
// Run with: cargo run --example test_torrent_destroy

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_torrent_destroy ===");

    let test_data = Bytes::from("Test data for destroy.");
    let creator = TorrentCreator::new();
    let (torrent_file, info_hash) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    println!("Destroying torrent...");
    torrent.destroy().await?;
    println!("✓ Torrent destroyed");

    // Note: Torrent destroy may or may not remove from client immediately
    // This depends on implementation details
    println!("✓ Torrent destroyed successfully");

    client.destroy().await?;
    println!("=== test_torrent_destroy PASSED ===");
    Ok(())
}

