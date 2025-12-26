// Test: Selection on single-file torrent (edge case)
// Run with: cargo run --example test_file_selection_single_file_torrent

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_file_selection_single_file_torrent ===");

    let test_data = Bytes::from("Test data for single file.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("single_file.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    let files = torrent.files();
    assert_eq!(files.len(), 1, "Should have exactly one file");
    println!("✓ Single-file torrent created");

    // Try to select the single file
    torrent.select(0, 1).await?;
    assert!(torrent.is_file_selected(0).await, "File should be selected");
    println!("✓ Single file can be selected");

    client.destroy().await?;
    println!("=== test_file_selection_single_file_torrent PASSED ===");
    Ok(())
}

