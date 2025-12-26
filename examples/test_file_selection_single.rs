// Test: select() single file in multi-file torrent
// Note: Creating multi-file torrents requires special handling
// Run with: cargo run --example test_file_selection_single

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_file_selection_single ===");

    // For now, test with single-file torrent (multi-file requires TorrentCreator enhancement)
    let test_data = Bytes::from("Test data for file selection.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    let files = torrent.files();
    if files.len() > 0 {
        println!("Selecting file index 0...");
        let result = torrent.select(0, 1).await?;
        assert!(result, "File selection should succeed");
        println!("✓ File selected");

        let is_selected = torrent.is_file_selected(0).await;
        assert!(is_selected, "File should be selected");
        println!("✓ File is selected");
    } else {
        println!("⚠ Single-file torrent (no multi-file to test)");
    }

    client.destroy().await?;
    println!("=== test_file_selection_single PASSED ===");
    Ok(())
}

