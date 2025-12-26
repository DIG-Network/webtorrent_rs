// Test: deselect() files
// Run with: cargo run --example test_file_selection_deselect

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_file_selection_deselect ===");

    let test_data = Bytes::from("Test data.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    let files = torrent.files();
    if files.len() > 0 {
        // Select file
        torrent.select(0, 1).await?;
        assert!(torrent.is_file_selected(0).await, "File should be selected");
        println!("✓ File selected");

        // Deselect file
        torrent.deselect(0).await?;
        assert!(!torrent.is_file_selected(0).await, "File should be deselected");
        println!("✓ File deselected");
    }

    client.destroy().await?;
    println!("=== test_file_selection_deselect PASSED ===");
    Ok(())
}

