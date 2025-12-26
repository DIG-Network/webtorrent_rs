// Test: Multiple streams from same torrent
// Run with: cargo run --example test_stream_concurrent

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_stream_concurrent ===");

    let test_data = Bytes::from("Test data.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = Arc::new(client.add(TorrentId::TorrentFile(torrent_file)).await?);
    
    // Create multiple streams
    let mut stream1 = torrent.create_read_stream(0)?;
    println!("✓ Stream 1 created");
    
    let mut stream2 = torrent.create_read_stream(0)?;
    println!("✓ Stream 2 created");

    // Both streams should work independently
    let pos1 = stream1.position();
    let pos2 = stream2.position();
    println!("  Stream 1 position: {}", pos1);
    println!("  Stream 2 position: {}", pos2);

    stream1.destroy().await?;
    stream2.destroy().await?;
    println!("✓ Both streams destroyed");

    client.destroy().await?;
    println!("=== test_stream_concurrent PASSED ===");
    Ok(())
}

