// Test: download_limit and upload_limit combinations
// Run with: cargo run --example test_client_throttling

use webtorrent::{WebTorrent, WebTorrentOptions};
use tracing_subscriber;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_client_throttling ===");

    // Test: Download limit only
    println!("[1/3] Testing download limit only...");
    let mut options = WebTorrentOptions::default();
    options.download_limit = Some(1024 * 100); // 100 KB/s
    let client1 = WebTorrent::new(options).await?;
    println!("✓ Client with download limit created");
    client1.destroy().await?;

    // Test: Upload limit only
    println!("[2/3] Testing upload limit only...");
    let mut options = WebTorrentOptions::default();
    options.upload_limit = Some(1024 * 50); // 50 KB/s
    let client2 = WebTorrent::new(options).await?;
    println!("✓ Client with upload limit created");
    client2.destroy().await?;

    // Test: Both limits
    println!("[3/3] Testing both limits...");
    let mut options = WebTorrentOptions::default();
    options.download_limit = Some(1024 * 100); // 100 KB/s
    options.upload_limit = Some(1024 * 50); // 50 KB/s
    let client3 = WebTorrent::new(options).await?;
    println!("✓ Client with both limits created");
    
    // Check speed metrics (should be 0 initially)
    let download_speed = client3.download_speed().await;
    let upload_speed = client3.upload_speed().await;
    println!("  Initial download speed: {} bytes/s", download_speed);
    println!("  Initial upload speed: {} bytes/s", upload_speed);
    
    client3.destroy().await?;

    println!("=== test_client_throttling PASSED ===");
    Ok(())
}

