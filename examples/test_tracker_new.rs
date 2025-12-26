// Test: TrackerClient::new() initialization
// Run with: cargo run --example test_tracker_new

use webtorrent::tracker::TrackerClient;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_tracker_new ===");

    let info_hash = [0u8; 20];
    let peer_id = [0u8; 20];
    let port = 6881;
    let announce_url = "http://tracker.example.com/announce".to_string();

    let tracker = TrackerClient::new(announce_url.clone(), info_hash, peer_id, port);
    println!("✓ TrackerClient created");
    println!("  Announce URL: {}", announce_url);
    println!("  Port: {}", port);

    println!("=== test_tracker_new PASSED ===");
    Ok(())
}

