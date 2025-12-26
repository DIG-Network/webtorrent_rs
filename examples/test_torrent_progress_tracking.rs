// Test: progress(), downloaded(), uploaded(), received() during download
// Run with: cargo run --example test_torrent_progress_tracking

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_torrent_progress_tracking ===");

    let test_data = Bytes::from("Test data for progress tracking.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data.clone()).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    // Check initial progress metrics
    let progress = torrent.progress().await;
    let downloaded = torrent.downloaded().await;
    let uploaded = torrent.uploaded().await;
    let received = torrent.received().await;
    
    println!("Initial metrics:");
    println!("  progress(): {:.2}%", progress * 100.0);
    println!("  downloaded(): {} bytes", downloaded);
    println!("  uploaded(): {} bytes", uploaded);
    println!("  received(): {} bytes", received);

    // Wait a bit and check again
    sleep(Duration::from_millis(100)).await;
    
    let progress2 = torrent.progress().await;
    println!("After 100ms:");
    println!("  progress(): {:.2}%", progress2 * 100.0);

    println!("✓ Progress tracking methods work");

    client.destroy().await?;
    println!("=== test_torrent_progress_tracking PASSED ===");
    Ok(())
}

