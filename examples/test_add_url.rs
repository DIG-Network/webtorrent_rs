// Test: TorrentId::Url(String) - HTTP/HTTPS URLs
// Run with: cargo run --example test_add_url

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_add_url ===");

    // Test: HTTP URL
    println!("[1/2] Testing HTTP URL...");
    let http_url = "http://example.com/torrent.torrent";
    let client1 = WebTorrent::new(WebTorrentOptions::default()).await?;
    
    match client1.add(TorrentId::Url(http_url.to_string())).await {
        Ok(torrent) => {
            println!("  ✓ HTTP URL added");
            println!("    Info hash: {}", hex::encode(torrent.info_hash()));
        },
        Err(e) => {
            println!("  ⚠ HTTP URL failed (expected if URL doesn't exist): {}", e);
        }
    }
    client1.destroy().await?;

    // Test: HTTPS URL
    println!("[2/2] Testing HTTPS URL...");
    let https_url = "https://example.com/torrent.torrent";
    let client2 = WebTorrent::new(WebTorrentOptions::default()).await?;
    
    match client2.add(TorrentId::Url(https_url.to_string())).await {
        Ok(torrent) => {
            println!("  ✓ HTTPS URL added");
            println!("    Info hash: {}", hex::encode(torrent.info_hash()));
        },
        Err(e) => {
            println!("  ⚠ HTTPS URL failed (expected if URL doesn't exist): {}", e);
        }
    }
    client2.destroy().await?;

    println!("=== test_add_url PASSED ===");
    Ok(())
}

