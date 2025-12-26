// Test: File that fits in single piece
// Run with: cargo run --example test_edge_single_piece

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_edge_single_piece ===");

    // Create file smaller than piece length (16KB default)
    let small_data = Bytes::from("Small file that fits in one piece.");
    let creator = TorrentCreator::new(); // Default piece length is 16KB
    let (torrent_file, info_hash) = creator.create_from_data("small.txt".to_string(), small_data.clone()).await?;

    println!("Created single-piece torrent");
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  File size: {} bytes", small_data.len());

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;

    let piece_hashes = torrent.piece_hashes();
    assert_eq!(piece_hashes.len(), 1, "Should have exactly one piece");
    println!("✓ Torrent has exactly one piece");

    client.destroy().await?;
    println!("=== test_edge_single_piece PASSED ===");
    Ok(())
}

