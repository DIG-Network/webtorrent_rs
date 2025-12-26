// Test: Large file (multiple pieces)
// Run with: cargo run --example test_creator_large_file

use webtorrent::TorrentCreator;
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_creator_large_file ===");

    // Create a large file (100KB, will create multiple pieces with 16KB piece length)
    let large_data: Vec<u8> = (0..1024 * 100).map(|i| (i % 256) as u8).collect();
    let large_bytes = Bytes::from(large_data);
    
    println!("Creating torrent for large file ({} bytes, piece length: 16KB)...", large_bytes.len());

    let creator = TorrentCreator::new(); // Default piece length is 16KB
    let (torrent_file, info_hash) = creator.create_from_data("large_file.bin".to_string(), large_bytes.clone()).await?;
    
    let num_pieces = (large_bytes.len() as u64 + 16384 - 1) / 16384;
    println!("✓ Torrent created for large file");
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  File size: {} bytes", large_bytes.len());
    println!("  Number of pieces: {}", num_pieces);
    println!("  Torrent file size: {} bytes", torrent_file.len());

    println!("=== test_creator_large_file PASSED ===");
    Ok(())
}

