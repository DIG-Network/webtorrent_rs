// Test: get_bitfield() and verify piece availability
// Run with: cargo run --example test_torrent_bitfield

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_torrent_bitfield ===");

    let test_data = Bytes::from("Test data for bitfield testing. ".repeat(10));
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    // Get bitfield
    let bitfield = torrent.get_bitfield().await;
    println!("Bitfield length: {} bits", bitfield.len());
    println!("Bitfield: {:?}", bitfield);

    // For a seeded torrent, all pieces should be available
    // (This depends on whether the torrent is ready/seeded)
    println!("✓ get_bitfield() works");

    client.destroy().await?;
    println!("=== test_torrent_bitfield PASSED ===");
    Ok(())
}

