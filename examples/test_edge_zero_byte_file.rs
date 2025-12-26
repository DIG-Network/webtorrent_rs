// Test: Zero-byte file torrent
// Run with: cargo run --example test_edge_zero_byte_file

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_edge_zero_byte_file ===");

    // Create torrent with zero-byte file
    let empty_data = Bytes::new();
    let creator = TorrentCreator::new();
    let (torrent_file, info_hash) = creator.create_from_data("empty.txt".to_string(), empty_data).await?;

    println!("Created zero-byte torrent");
    println!("  Info hash: {}", hex::encode(info_hash));

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;

    let length = torrent.length().await;
    assert_eq!(length, 0, "Length should be 0");
    println!("✓ Zero-byte file handled correctly");

    let progress = torrent.progress().await;
    assert_eq!(progress, 1.0, "Progress should be 100% for zero-byte file");
    println!("✓ Progress is 100% for zero-byte file");

    client.destroy().await?;
    println!("=== test_edge_zero_byte_file PASSED ===");
    Ok(())
}

