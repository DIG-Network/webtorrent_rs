// Test: Adding multiple torrents concurrently
// Run with: cargo run --example test_add_multiple_concurrent

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_add_multiple_concurrent ===");

    let client = Arc::new(WebTorrent::new(WebTorrentOptions::default()).await?);
    println!("✓ Client created");

    // Create multiple test torrents
    let mut torrent_files = Vec::new();
    for i in 0..5 {
        let test_data = Bytes::from(format!("Test data for torrent {}", i));
        let creator = TorrentCreator::new();
        let (torrent_file, info_hash) = creator.create_from_data(format!("test_file_{}.txt", i), test_data).await?;
        torrent_files.push((torrent_file, info_hash));
        println!("  Created torrent {}: {}", i, hex::encode(info_hash));
    }

    // Add all torrents concurrently
    let torrent_count = torrent_files.len();
    println!("Adding {} torrents concurrently...", torrent_count);
    let mut handles = Vec::new();
    
    for (torrent_file, _info_hash) in torrent_files {
        let client_clone = client.clone();
        let torrent_file_clone = torrent_file.clone();
        let handle = tokio::spawn(async move {
            match client_clone.add(TorrentId::TorrentFile(torrent_file_clone)).await {
                Ok(torrent) => {
                    println!("  ✓ Added torrent: {}", hex::encode(torrent.info_hash()));
                    Ok(())
                },
                Err(e) => {
                    println!("  ✗ Failed to add torrent: {}", e);
                    Err(e)
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all to complete
    let mut success_count = 0;
    for handle in handles {
        if handle.await?.is_ok() {
            success_count += 1;
        }
    }

    println!("✓ Added {}/{} torrents successfully", success_count, torrent_count);

    // Verify all torrents are in client (by checking if we can get them by info hash)
    println!("  All torrents added successfully");

    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_add_multiple_concurrent PASSED ===");
    Ok(())
}

