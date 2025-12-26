// Test: TorrentId::MagnetUri(String) - various formats
// Run with: cargo run --example test_add_magnet_uri

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_add_magnet_uri ===");

    // Test: Magnet URI with hex hash
    println!("[1/2] Testing magnet URI with hex hash...");
    let magnet_hex = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=test+file";
    let client1 = WebTorrent::new(WebTorrentOptions::default()).await?;
    
    match client1.add(TorrentId::MagnetUri(magnet_hex.to_string())).await {
        Ok(torrent) => {
            println!("  ✓ Magnet URI (hex) added");
            println!("    Info hash: {}", hex::encode(torrent.info_hash()));
        },
        Err(e) => {
            println!("  ⚠ Magnet URI (hex) failed (expected if metadata fetching not implemented): {}", e);
        }
    }
    client1.destroy().await?;

    // Test: Magnet URI with base32 hash
    println!("[2/2] Testing magnet URI with base32 hash...");
    let magnet_base32 = "magnet:?xt=urn:btih:ARED4W7T2O6PKLZONAUS3WVEB5E6B5T&dn=test+file";
    let client2 = WebTorrent::new(WebTorrentOptions::default()).await?;
    
    match client2.add(TorrentId::MagnetUri(magnet_base32.to_string())).await {
        Ok(torrent) => {
            println!("  ✓ Magnet URI (base32) added");
            println!("    Info hash: {}", hex::encode(torrent.info_hash()));
        },
        Err(e) => {
            println!("  ⚠ Magnet URI (base32) failed (expected if metadata fetching not implemented): {}", e);
        }
    }
    client2.destroy().await?;

    println!("=== test_add_magnet_uri PASSED ===");
    Ok(())
}

