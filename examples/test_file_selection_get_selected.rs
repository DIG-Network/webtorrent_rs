// Test: get_selected_files() verification
// Run with: cargo run --example test_file_selection_get_selected

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_file_selection_get_selected ===");

    let test_data = Bytes::from("Test data.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    let files = torrent.files();
    if files.len() > 0 {
        // Initially no files selected
        let selected = torrent.get_selected_files().await;
        assert_eq!(selected.len(), 0, "No files should be selected initially");
        println!("✓ No files selected initially");

        // Select file
        torrent.select(0, 1).await?;
        let selected = torrent.get_selected_files().await;
        assert_eq!(selected.len(), 1, "One file should be selected");
        assert_eq!(selected[0], 0, "File index 0 should be selected");
        println!("✓ File 0 is in selected files list");
    }

    client.destroy().await?;
    println!("=== test_file_selection_get_selected PASSED ===");
    Ok(())
}

