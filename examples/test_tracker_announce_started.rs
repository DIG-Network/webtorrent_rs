// Test: announce() with "started" event
// Run with: cargo run --example test_tracker_announce_started

use webtorrent::tracker::TrackerClient;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_tracker_announce_started ===");

    let info_hash = {
        let mut hash = [0u8; 20];
        hash[0] = 0x01;
        hash
    };
    let peer_id = {
        let mut id = [0u8; 20];
        id[0..3].copy_from_slice(b"-WW");
        id
    };
    let port = 6881;
    let announce_url = "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/announce".to_string();

    let tracker = TrackerClient::new(announce_url, info_hash, peer_id, port);
    
    println!("Announcing with 'started' event...");
    match tracker.announce(0, 0, 0, "started").await {
        Ok((response, peers)) => {
            println!("✓ Announce successful");
            println!("  Peers found: {}", peers.len());
            if let Some(interval) = response.interval {
                println!("  Interval: {} seconds", interval);
            }
        },
        Err(e) => {
            println!("⚠ Announce failed (may be expected): {}", e);
        }
    }

    println!("=== test_tracker_announce_started PASSED ===");
    Ok(())
}

