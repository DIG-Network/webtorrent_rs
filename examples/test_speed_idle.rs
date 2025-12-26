// Test: Speed metrics when idle (should be 0)
// Run with: cargo run --example test_speed_idle

use webtorrent::{WebTorrent, WebTorrentOptions};
use tracing_subscriber;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_speed_idle ===");

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    
    // Check speeds multiple times while idle
    for i in 0..5 {
        let download_speed = client.download_speed().await;
        let upload_speed = client.upload_speed().await;
        
        println!("Check {}: download={} bytes/s, upload={} bytes/s", 
                 i+1, download_speed, upload_speed);
        
        assert_eq!(download_speed, 0, "Download speed should be 0 when idle");
        assert_eq!(upload_speed, 0, "Upload speed should be 0 when idle");
        
        sleep(Duration::from_millis(200)).await;
    }

    println!("✓ Speed metrics correctly show 0 when idle");

    client.destroy().await?;
    println!("=== test_speed_idle PASSED ===");
    Ok(())
}

