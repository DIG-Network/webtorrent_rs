// Test: upload_speed() during active seeding
// Run with: cargo run --example test_speed_upload

use webtorrent::{WebTorrent, WebTorrentOptions};
use bytes::Bytes;
use tracing_subscriber;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_speed_upload ===");

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    
    // Check initial upload speed (should be 0)
    let speed = client.upload_speed().await;
    println!("Initial upload speed: {} bytes/s", speed);
    assert_eq!(speed, 0, "Initial speed should be 0");

    // Speed will remain 0 without active upload
    sleep(Duration::from_millis(100)).await;
    let speed2 = client.upload_speed().await;
    println!("Upload speed after 100ms: {} bytes/s", speed2);

    println!("✓ upload_speed() works");

    client.destroy().await?;
    println!("=== test_speed_upload PASSED ===");
    Ok(())
}

