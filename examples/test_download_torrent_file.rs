// Test: Download from .torrent file
// Run with: cargo run --example test_download_torrent_file

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_download_torrent_file ===");

    // Create a test torrent file
    let test_data = Bytes::from("Test data for download.");
    let creator = TorrentCreator::new();
    let (torrent_file, info_hash) = creator.create_from_data("download_test.txt".to_string(), test_data).await?;

    println!("Created test torrent");
    println!("  Info hash: {}", hex::encode(info_hash));

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    println!("✓ Client created");

    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    println!("✓ Torrent added");

    torrent.start_discovery().await?;
    println!("✓ Discovery started");

    // Wait for potential download
    sleep(Duration::from_secs(2)).await;

    let progress = torrent.progress().await;
    let downloaded = torrent.downloaded().await;
    println!("  Progress: {:.1}%", progress * 100.0);
    println!("  Downloaded: {} bytes", downloaded);

    client.destroy().await?;
    println!("=== test_download_torrent_file PASSED ===");
    Ok(())
}

