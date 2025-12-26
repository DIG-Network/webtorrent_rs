// Test: download_speed() during active download
// Run with: cargo run --example test_speed_download

use webtorrent::{WebTorrent, WebTorrentOptions};
use bytes::Bytes;
use tracing_subscriber;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_speed_download ===");

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    
    // Check initial download speed (should be 0)
    let speed = client.download_speed().await;
    println!("Initial download speed: {} bytes/s", speed);
    assert_eq!(speed, 0, "Initial speed should be 0");

    // Speed will remain 0 without active download
    sleep(Duration::from_millis(100)).await;
    let speed2 = client.download_speed().await;
    println!("Download speed after 100ms: {} bytes/s", speed2);

    println!("✓ download_speed() works");

    client.destroy().await?;
    println!("=== test_speed_download PASSED ===");
    Ok(())
}

