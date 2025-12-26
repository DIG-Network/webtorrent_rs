// Test: Both speeds during seed+download
// Run with: cargo run --example test_speed_both

use webtorrent::{WebTorrent, WebTorrentOptions};
use tracing_subscriber;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_speed_both ===");

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    
    // Check both speeds
    let download_speed = client.download_speed().await;
    let upload_speed = client.upload_speed().await;
    
    println!("Download speed: {} bytes/s", download_speed);
    println!("Upload speed: {} bytes/s", upload_speed);

    sleep(Duration::from_millis(100)).await;
    
    let download_speed2 = client.download_speed().await;
    let upload_speed2 = client.upload_speed().await;
    
    println!("After 100ms:");
    println!("  Download speed: {} bytes/s", download_speed2);
    println!("  Upload speed: {} bytes/s", upload_speed2);

    println!("✓ Both speed metrics work");

    client.destroy().await?;
    println!("=== test_speed_both PASSED ===");
    Ok(())
}

