// Test: get_file_priority() verification
// Run with: cargo run --example test_file_selection_get_priority

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_file_selection_get_priority ===");

    let test_data = Bytes::from("Test data.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    let files = torrent.files();
    if files.len() > 0 {
        // Initially no priority
        let priority = torrent.get_file_priority(0).await;
        assert_eq!(priority, None, "Priority should be None initially");
        println!("✓ Priority is None initially");

        // Set priority
        torrent.select(0, 5).await?;
        let priority = torrent.get_file_priority(0).await;
        assert_eq!(priority, Some(5), "Priority should be 5");
        println!("✓ Priority is 5 after select()");
    }

    client.destroy().await?;
    println!("=== test_file_selection_get_priority PASSED ===");
    Ok(())
}

