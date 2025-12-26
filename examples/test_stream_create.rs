// Test: create_read_stream() for single-file torrent
// Run with: cargo run --example test_stream_create

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_stream_create ===");

    let test_data = Bytes::from("Test data for stream creation.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = Arc::new(client.add(TorrentId::TorrentFile(torrent_file)).await?);
    
    println!("Creating read stream for file index 0...");
    let mut stream = torrent.create_read_stream(0)?;
    println!("✓ Read stream created");

    // Clean up
    stream.destroy().await?;
    client.destroy().await?;
    println!("=== test_stream_create PASSED ===");
    Ok(())
}

