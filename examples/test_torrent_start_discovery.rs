// Test: start_discovery() on new torrent
// Run with: cargo run --example test_torrent_start_discovery

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_torrent_start_discovery ===");

    let test_data = Bytes::from("Test data for discovery.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    println!("Starting discovery for torrent...");
    torrent.start_discovery().await?;
    println!("✓ Discovery started");

    client.destroy().await?;
    println!("=== test_torrent_start_discovery PASSED ===");
    Ok(())
}

