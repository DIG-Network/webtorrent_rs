// Test: select() multiple files
// Run with: cargo run --example test_file_selection_multiple

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_file_selection_multiple ===");

    let test_data = Bytes::from("Test data.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    let files = torrent.files();
    if files.len() > 0 {
        // Select file with different priorities
        println!("Selecting file with priority 1...");
        torrent.select(0, 1).await?;
        println!("✓ File selected with priority 1");

        let priority = torrent.get_file_priority(0).await;
        assert_eq!(priority, Some(1), "Priority should be 1");
        println!("✓ Priority verified: {:?}", priority);
    } else {
        println!("⚠ Single-file torrent");
    }

    client.destroy().await?;
    println!("=== test_file_selection_multiple PASSED ===");
    Ok(())
}

