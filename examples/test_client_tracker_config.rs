// Test: Custom TrackerConfig with announce URLs
// Run with: cargo run --example test_client_tracker_config

use webtorrent::{WebTorrent, WebTorrentOptions, TrackerConfig};
use tracing_subscriber;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_client_tracker_config ===");

    // Test: Custom tracker config
    let mut options = WebTorrentOptions::default();
    options.tracker = Some(TrackerConfig {
        announce: vec![
            "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/announce".to_string(),
            "udp://tracker.example.com:1337/announce".to_string(),
        ],
        get_announce_opts: None,
    });
    
    println!("Creating client with custom tracker config...");
    let client = WebTorrent::new(options).await?;
    println!("✓ Client created with custom tracker config");
    
    let peer_id = client.peer_id();
    println!("  Peer ID: {}", hex::encode(peer_id));
    
    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_client_tracker_config PASSED ===");
    Ok(())
}

