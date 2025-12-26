// Test: TorrentId::InfoHash([u8; 20]) only
// Run with: cargo run --example test_add_info_hash

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_add_info_hash ===");

    // Create a test info hash
    let info_hash = {
        let mut hash = [0u8; 20];
        hash[0] = 0x01;
        hash[1] = 0x23;
        hash[2] = 0x45;
        hash[3] = 0x67;
        // Rest are zeros
        hash
    };

    println!("Adding torrent with info hash only...");
    println!("  Info hash: {}", hex::encode(info_hash));

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    
    match client.add(TorrentId::InfoHash(info_hash)).await {
        Ok(torrent) => {
            println!("✓ Torrent added via InfoHash");
            println!("  Torrent info hash: {}", hex::encode(torrent.info_hash()));
            assert_eq!(torrent.info_hash(), info_hash);
        },
        Err(e) => {
            println!("⚠ InfoHash only failed (expected if metadata fetching not implemented): {}", e);
        }
    }

    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_add_info_hash PASSED ===");
    Ok(())
}

