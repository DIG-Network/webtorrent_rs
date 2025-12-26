// Test: TorrentId::TorrentFile(Bytes)
// Run with: cargo run --example test_add_torrent_file

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_add_torrent_file ===");

    // Create a test torrent file
    let test_data = Bytes::from("This is test data for adding torrent file.");
    let creator = TorrentCreator::new();
    let (torrent_file, info_hash) = creator.create_from_data("test_file.txt".to_string(), test_data).await?;
    
    println!("Created test torrent file");
    println!("  Info hash: {}", hex::encode(info_hash));

    // Create client and add torrent
    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    println!("✓ Client created");

    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    println!("✓ Torrent added via TorrentFile");
    println!("  Torrent name: {}", torrent.name());
    println!("  Torrent info hash: {}", hex::encode(torrent.info_hash()));

    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_add_torrent_file PASSED ===");
    Ok(())
}

