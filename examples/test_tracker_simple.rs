// Simple test: Direct tracker announcement test
// Run with: cargo run --example test_tracker_simple

use hex;
use webtorrent::tracker::TrackerClient;

const TRACKER_URL: &str =
    "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/announce";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple Tracker Connection Test ===");
    println!("Tracker URL: {}", TRACKER_URL);
    println!();
    
    // Create a test info hash (20 bytes)
    let mut info_hash = [0u8; 20];
    info_hash[0] = 0x01;
    info_hash[1] = 0x23;
    // Rest are zeros for testing
    
    // Create a test peer ID (20 bytes) - WebTorrent format
    let mut peer_id = [0u8; 20];
    peer_id[0..4].copy_from_slice(b"-WW");
    peer_id[4..8].copy_from_slice(b"0100");
    peer_id[8] = b'-';
    // Rest are random
    
    let port = 6881;
    
    println!("Test parameters:");
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  Peer ID: {}", hex::encode(peer_id));
    println!("  Port: {}", port);
    println!();
    
    let tracker = TrackerClient::new(
        TRACKER_URL.to_string(),
        info_hash,
        peer_id,
        port,
    );
    
    println!("Announcing to tracker...");
    println!("  Full URL will be constructed with query parameters");
    println!();
    
    match tracker.announce(0, 0, 0, "started").await {
        Ok((response, peers)) => {
            println!("  Peers found: {}", peers.len());
            for (ip, port) in peers.iter().take(5) {
                println!("    - {}:{}", ip, port);
            }
            println!("✓ SUCCESS! Tracker responded!");
            println!();
            println!("Response details:");
            if let Some(interval) = response.interval {
                println!("  Interval: {} seconds", interval);
            }
            if let Some(min_interval) = response.min_interval {
                println!("  Min interval: {} seconds", min_interval);
            }
            if let Some(ref tracker_id) = response.tracker_id {
                println!("  Tracker ID: {}", tracker_id);
            }
            if let Some(complete) = response.complete {
                println!("  Complete (seeders): {}", complete);
            }
            if let Some(incomplete) = response.incomplete {
                println!("  Incomplete (leechers): {}", incomplete);
            }
            match &response.peers {
                webtorrent::tracker::TrackerPeers::String(s) => {
                    if !s.is_empty() {
                        println!("  Peers (compact): {} bytes", s.len());
                    } else {
                        println!("  Peers: None");
                    }
                }
                webtorrent::tracker::TrackerPeers::List(peers) => {
                    println!("  Peers (list): {} peers", peers.len());
                    for peer in peers.iter().take(5) {
                        println!("    - {}:{}", peer.ip, peer.port);
                    }
                }
            }
            if let Some(ref failure) = response.failure_reason {
                eprintln!("  ⚠ Failure reason: {}", failure);
            }
            if let Some(ref warning) = response.warning_message {
                eprintln!("  ⚠ Warning: {}", warning);
            }
            println!();
            println!("✓ Check the stats page: http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/stats");
        }
        Err(e) => {
            eprintln!("✗ FAILED to connect to tracker");
            eprintln!("Error: {}", e);
            eprintln!("Error details: {:?}", e);
            println!();
            eprintln!("Possible issues:");
            eprintln!("  1. Tracker server is down or unreachable");
            eprintln!("  2. Network connectivity issues");
            eprintln!("  3. Firewall blocking the connection");
            eprintln!("  4. Invalid tracker URL format");
            return Err(e.into());
        }
    }
    
    println!("✓ Test completed successfully!");
    Ok(())
}

