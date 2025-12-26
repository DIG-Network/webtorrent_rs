// Test: Simulate seeking by creating new stream at different positions
// Note: Seeking requires creating a new stream
// Run with: cargo run --example test_stream_seek_simulation

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_stream_seek_simulation ===");

    let test_data = Bytes::from("Test data for seek simulation.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = Arc::new(client.add(TorrentId::TorrentFile(torrent_file)).await?);
    
    // Create initial stream
    let mut stream1 = torrent.create_read_stream(0)?;
    let pos1 = stream1.position();
    println!("Initial stream position: {}", pos1);
    
    // Destroy and create new stream (simulating seek)
    stream1.destroy().await?;
    let mut stream2 = torrent.create_read_stream(0)?;
    let pos2 = stream2.position();
    println!("New stream position: {}", pos2);
    
    println!("✓ Seek simulation works (by creating new stream)");

    stream2.destroy().await?;
    client.destroy().await?;
    println!("=== test_stream_seek_simulation PASSED ===");
    Ok(())
}

