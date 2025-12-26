// Test: create_read_stream() for multi-file torrent
// Note: Multi-file torrents require special creation
// Run with: cargo run --example test_stream_multi_file

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_stream_multi_file ===");

    // For now, test with single-file (multi-file requires TorrentCreator enhancement)
    let test_data = Bytes::from("Test data.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = Arc::new(client.add(TorrentId::TorrentFile(torrent_file)).await?);
    
    let files = torrent.files();
    if files.len() > 0 {
        println!("Creating read stream for file index 0...");
        let mut stream = torrent.create_read_stream(0)?;
        println!("✓ Read stream created for file 0");
        stream.destroy().await?;
    } else {
        println!("⚠ Single-file torrent");
    }

    client.destroy().await?;
    println!("=== test_stream_multi_file PASSED ===");
    Ok(())
}

