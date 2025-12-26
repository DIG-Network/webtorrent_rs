// Test: position() tracking during read
// Run with: cargo run --example test_stream_position

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_stream_position ===");

    let test_data = Bytes::from("Test data for position tracking.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = Arc::new(client.add(TorrentId::TorrentFile(torrent_file)).await?);
    
    let mut stream = torrent.create_read_stream(0)?;
    
    let initial_position = stream.position();
    println!("Initial position: {}", initial_position);
    assert_eq!(initial_position, 0, "Initial position should be 0");
    
    println!("✓ position() tracking works");

    stream.destroy().await?;
    client.destroy().await?;
    println!("=== test_stream_position PASSED ===");
    Ok(())
}

