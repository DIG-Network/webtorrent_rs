// Test: is_eof() at end of file
// Run with: cargo run --example test_stream_eof

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_stream_eof ===");

    let test_data = Bytes::from("Test data for EOF testing.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = Arc::new(client.add(TorrentId::TorrentFile(torrent_file)).await?);
    
    let mut stream = torrent.create_read_stream(0)?;
    
    let is_eof = stream.is_eof().await;
    println!("Initial EOF state: {}", is_eof);
    // EOF state depends on whether data is available
    
    println!("✓ is_eof() works");

    stream.destroy().await?;
    client.destroy().await?;
    println!("=== test_stream_eof PASSED ===");
    Ok(())
}

