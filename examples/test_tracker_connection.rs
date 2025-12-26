// Simple test: Connect to dig-relay tracker
// Run with: cargo run --example test_tracker_connection

use webtorrent::tracker::TrackerClient;

const TRACKER_URL: &str =
    "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/announce";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing connection to dig-relay tracker...");
    println!("Tracker URL: {}", TRACKER_URL);
    
    // Create a dummy info hash and peer ID for testing
    let info_hash = [0u8; 20];
    let peer_id = [0u8; 20];
    let port = 6881;
    
    let tracker = TrackerClient::new(
        TRACKER_URL.to_string(),
        info_hash,
        peer_id,
        port,
    );
    
    println!("\nAttempting to announce...");
    match tracker.announce(0, 0, 0, "started").await {
        Ok((response, peers)) => {
            println!("  Peers found: {}", peers.len());
            for (ip, port) in peers.iter().take(5) {
                println!("    - {}:{}", ip, port);
            }
            println!("✓ Successfully connected to tracker!");
            println!("  Interval: {:?}", response.interval);
            println!("  Complete (seeders): {:?}", response.complete);
            println!("  Incomplete (leechers): {:?}", response.incomplete);
            println!("  Peers: {:?}", response.peers);
            if let Some(ref reason) = response.failure_reason {
                println!("  Warning: {}", reason);
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to connect: {}", e);
            eprintln!("Error details: {:?}", e);
            return Err(e.into());
        }
    }
    
    println!("\n✓ Tracker connection test completed successfully!");
    println!("=== test_tracker_connection PASSED ===");
    Ok(())
}

