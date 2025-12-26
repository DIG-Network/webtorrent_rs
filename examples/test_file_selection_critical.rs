// Test: critical() for streaming priority
// Run with: cargo run --example test_file_selection_critical

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_file_selection_critical ===");

    let test_data = Bytes::from("Test data.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    let files = torrent.files();
    if files.len() > 0 {
        // Mark file as critical
        torrent.critical(0).await?;
        println!("✓ File marked as critical");

        // Critical files should have priority 7
        let priority = torrent.get_file_priority(0).await;
        assert_eq!(priority, Some(7), "Critical file should have priority 7");
        println!("✓ Priority verified: {:?}", priority);
    }

    client.destroy().await?;
    println!("=== test_file_selection_critical PASSED ===");
    Ok(())
}

