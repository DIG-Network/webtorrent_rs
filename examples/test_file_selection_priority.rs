// Test: Various priority levels
// Run with: cargo run --example test_file_selection_priority

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_file_selection_priority ===");

    let test_data = Bytes::from("Test data.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    let files = torrent.files();
    if files.len() > 0 {
        // Test various priority levels
        let priorities = vec![1, 3, 5, 7];
        for priority in priorities {
            torrent.select(0, priority).await?;
            let actual_priority = torrent.get_file_priority(0).await;
            assert_eq!(actual_priority, Some(priority), "Priority should match");
            println!("✓ Priority {} set and verified", priority);
        }
    }

    client.destroy().await?;
    println!("=== test_file_selection_priority PASSED ===");
    Ok(())
}

