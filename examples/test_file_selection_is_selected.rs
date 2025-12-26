// Test: is_file_selected() verification
// Run with: cargo run --example test_file_selection_is_selected

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_file_selection_is_selected ===");

    let test_data = Bytes::from("Test data.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    let files = torrent.files();
    if files.len() > 0 {
        // Initially not selected
        assert!(!torrent.is_file_selected(0).await, "File should not be selected initially");
        println!("✓ File not selected initially");

        // Select file
        torrent.select(0, 1).await?;
        assert!(torrent.is_file_selected(0).await, "File should be selected");
        println!("✓ File is selected after select()");
    }

    client.destroy().await?;
    println!("=== test_file_selection_is_selected PASSED ===");
    Ok(())
}

