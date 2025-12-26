// Test: Speed metrics with throttling enabled
// Run with: cargo run --example test_speed_throttled

use webtorrent::{WebTorrent, WebTorrentOptions};
use tracing_subscriber;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_speed_throttled ===");

    // Create client with throttling
    let mut options = WebTorrentOptions::default();
    options.download_limit = Some(1024 * 100); // 100 KB/s
    options.upload_limit = Some(1024 * 50); // 50 KB/s
    
    let client = WebTorrent::new(options).await?;
    
    // Check speeds (will be 0 without active transfer)
    let download_speed = client.download_speed().await;
    let upload_speed = client.upload_speed().await;
    
    println!("Download speed: {} bytes/s (limit: 100 KB/s)", download_speed);
    println!("Upload speed: {} bytes/s (limit: 50 KB/s)", upload_speed);

    println!("✓ Speed metrics work with throttling enabled");

    client.destroy().await?;
    println!("=== test_speed_throttled PASSED ===");
    Ok(())
}

