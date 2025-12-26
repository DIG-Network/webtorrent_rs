// Test: info_hash(), name(), length(), piece_length(), files(), piece_hashes(), is_private()
// Run with: cargo run --example test_torrent_metadata_access

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_torrent_metadata_access ===");

    let test_data = Bytes::from("Test data for metadata access.");
    let creator = TorrentCreator::new();
    let (torrent_file, expected_info_hash) = creator.create_from_data("test.txt".to_string(), test_data.clone()).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    // Test all metadata accessors
    let info_hash = torrent.info_hash();
    assert_eq!(info_hash, expected_info_hash);
    println!("✓ info_hash(): {}", hex::encode(info_hash));

    let name = torrent.name();
    assert_eq!(name, "test.txt");
    println!("✓ name(): {}", name);

    let length = torrent.length().await;
    assert_eq!(length, test_data.len() as u64);
    println!("✓ length(): {} bytes", length);

    let piece_length = torrent.piece_length().await;
    println!("✓ piece_length(): {} bytes", piece_length);

    let files = torrent.files();
    assert_eq!(files.len(), 1);
    println!("✓ files(): {} file(s)", files.len());

    let piece_hashes = torrent.piece_hashes();
    println!("✓ piece_hashes(): {} piece(s)", piece_hashes.len());

    let is_private = torrent.is_private();
    println!("✓ is_private(): {}", is_private);

    client.destroy().await?;
    println!("=== test_torrent_metadata_access PASSED ===");
    Ok(())
}

