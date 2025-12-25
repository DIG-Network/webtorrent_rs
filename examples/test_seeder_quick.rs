// Quick test: Seed to tracker for 10 seconds then exit
// Run with: cargo run --example test_seeder_quick

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentCreator};
use bytes::Bytes;
use std::time::Duration;
use tokio::time::sleep;
use tracing_subscriber;

const TRACKER_URL: &str = "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000";
const TRACKER_STATS_URL: &str = "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/stats";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== WebTorrent Rust - Quick Seeder Test ===");
    println!("Tracker: {}", TRACKER_URL);
    println!("Stats: {}", TRACKER_STATS_URL);
    println!();

    // Create client
    println!("[1/6] Creating client...");
    let options = WebTorrentOptions {
        torrent_port: 6881,
        dht_port: 0,
        max_conns: 10,
        utp: false,
        nat_upnp: false,
        nat_pmp: false,
        ..Default::default()
    };

    let client = WebTorrent::new(options).await?;
    println!("✓ Client created");
    
    // Check listening status
    let address = client.address().await;
    match address {
        Some((ip, port)) => {
            println!("✓ Client address: {}:{}", ip, port);
        }
        None => {
            println!("⚠ Client not listening yet (will start when torrent is added)");
        }
    }
    println!("  Peer ID: {}", hex::encode(client.peer_id()));
    println!();

    // Create test data
    println!("[2/6] Creating test torrent file...");
    let test_data = Bytes::from("Hello, World! This is a test file seeded to the dig-relay tracker from WebTorrent Rust.");
    let test_name = "test_file.txt";
    let announce_url = format!("{}/announce", TRACKER_URL);
    
    let creator = TorrentCreator::new()
        .with_announce(vec![announce_url.clone()])
        .with_comment("Test file from WebTorrent Rust".to_string());

    let (torrent_file, info_hash) = creator
        .create_from_data(test_name.to_string(), test_data.clone())
        .await?;

    println!("✓ Torrent created");
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  File size: {} bytes", test_data.len());
    println!();

    // Add torrent
    println!("[3/6] Adding torrent to client...");
    let torrent = client.add(torrent_file).await?;
    println!("✓ Torrent added: {}", torrent.name());
    
    // Check if client is now listening
    let address_after = client.address().await;
    match address_after {
        Some((ip, port)) => {
            println!("✓ Client is now listening on {}:{}", ip, port);
        }
        None => {
            eprintln!("✗ ERROR: Client is still not listening!");
            return Err("Client not listening".into());
        }
    }
    println!();

    // Wait for discovery to start
    println!("[4/6] Waiting for discovery to start and announce to tracker...");
    sleep(Duration::from_secs(3)).await;
    println!("✓ Discovery should have started");
    println!();

    // Manually announce to verify
    println!("[5/6] Manually announcing to tracker...");
    use webtorrent::tracker::TrackerClient;
    let port = client.address().await
        .map(|(_ip, port)| port)
        .unwrap_or(0);
    
    if port == 0 {
        eprintln!("✗ ERROR: Port is 0! Cannot announce.");
        return Err("Port is 0".into());
    }
    
    let tracker = TrackerClient::new(
        announce_url.clone(),
        info_hash,
        client.peer_id(),
        port,
    );

    match tracker.announce(0, 0, test_data.len() as u64, "started").await {
        Ok((response, _peers)) => {
            println!("✓ Successfully announced to tracker!");
            if let Some(interval) = response.interval {
                println!("  Interval: {} seconds", interval);
            }
            if let Some(complete) = response.complete {
                println!("  Seeders: {}", complete);
            }
            if let Some(incomplete) = response.incomplete {
                println!("  Leechers: {}", incomplete);
            }
            if let Some(ref failure) = response.failure_reason {
                eprintln!("  ⚠ Failure: {}", failure);
            }
            if let Some(ref warning) = response.warning_message {
                eprintln!("  ⚠ Warning: {}", warning);
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to announce: {}", e);
            return Err(e.into());
        }
    }
    println!();

    // Keep seeding for a short time
    println!("[6/6] Seeding for 10 seconds...");
    println!("  Check stats: {}", TRACKER_STATS_URL);
    println!();
    
    // Re-announce once more
    sleep(Duration::from_secs(5)).await;
    println!("Re-announcing...");
    match tracker.announce(0, 0, 0, "update").await {
        Ok((response, _peers)) => {
            println!("✓ Re-announced: {} seeders, {} leechers",
                response.complete.unwrap_or(0),
                response.incomplete.unwrap_or(0)
            );
        }
        Err(e) => {
            eprintln!("✗ Re-announce failed: {}", e);
        }
    }
    
    sleep(Duration::from_secs(5)).await;
    
    println!();
    println!("=== Test Complete ===");
    println!("Check the stats page to see if the torrent appears:");
    println!("  {}", TRACKER_STATS_URL);
    println!();
    println!("If the tracker shows 0 torrents, check:");
    println!("  1. Network connectivity to the tracker");
    println!("  2. Firewall allowing outbound connections on port 6881");
    println!("  3. Tracker server is running and accessible");
    
    Ok(())
}

