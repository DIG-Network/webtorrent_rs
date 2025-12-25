// Example: Seed a file to the dig-relay tracker (with debug output)
// Run with: cargo run --example seed_to_dig_relay_debug

use bytes::Bytes;
use std::time::Duration;
use tokio::time::sleep;
use tracing_subscriber;
use webtorrent::{TorrentCreator, WebTorrent, WebTorrentOptions};

const TRACKER_URL: &str = "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000";
const TRACKER_STATS_URL: &str =
    "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/stats";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== WebTorrent Rust - Seed to dig-relay tracker (DEBUG) ===");
    println!("Tracker: {}", TRACKER_URL);
    println!("Stats page: {}", TRACKER_STATS_URL);
    println!();

    // Create client
    let options = WebTorrentOptions {
        torrent_port: 6881,
        dht_port: 0, // Disable DHT for this example
        max_conns: 10,
        utp: false,
        nat_upnp: false,
        nat_pmp: false,
        ..Default::default()
    };

    println!("Creating client with port 6881...");
    let client = WebTorrent::new(options).await?;
    println!("✓ Client created");
    
    // Check if client is listening
    let address = client.address().await;
    match address {
        Some((ip, port)) => {
            println!("✓ Client listening on {}:{}", ip, port);
            if port == 0 {
                eprintln!("⚠ WARNING: Port is 0! Tracker won't be able to connect back.");
            }
        }
        None => {
            eprintln!("⚠ WARNING: Client address is None! Tracker won't be able to connect back.");
        }
    }
    
    println!("Peer ID: {}", hex::encode(client.peer_id()));
    println!();

    // Create test file data
    let test_data = Bytes::from("Hello, World! This is a test file seeded to the dig-relay tracker from WebTorrent Rust.");
    let test_name = "test_file.txt";

    println!("Creating torrent...");
    
    // Create torrent with dig-relay tracker
    let announce_url = format!("{}/announce", TRACKER_URL);
    println!("Announce URL: {}", announce_url);
    
    let creator = TorrentCreator::new()
        .with_announce(vec![announce_url.clone()])
        .with_comment("Test file from WebTorrent Rust".to_string());

    let (torrent_file, info_hash) = creator
        .create_from_data(test_name.to_string(), test_data.clone())
        .await?;

    println!("✓ Torrent created!");
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  Torrent file size: {} bytes", torrent_file.len());
    println!();

    // Add torrent to client
    println!("Adding torrent to client...");
    let torrent = client.add(torrent_file).await?;
    println!("✓ Torrent added: {}", torrent.name());
    println!("  Torrent announce URLs: {:?}", torrent.name()); // We'll check this differently
    
    // Wait for torrent to initialize and discovery to start
    println!("Waiting for torrent to initialize and discovery to start...");
    sleep(Duration::from_secs(3)).await;
    println!();

    // Get the actual port (might have changed if 0 was specified)
    let port = client.address().await
        .map(|(_ip, port)| port)
        .unwrap_or(0);
    
    if port == 0 {
        eprintln!("✗ ERROR: Port is still 0! Cannot announce to tracker.");
        eprintln!("  The client must be listening on a valid port for the tracker to work.");
        return Err("Port is 0".into());
    }
    
    println!("Using port: {}", port);
    println!();

    // Announce to tracker manually (in addition to automatic discovery)
    use webtorrent::tracker::TrackerClient;
    let tracker = TrackerClient::new(
        announce_url.clone(),
        info_hash,
        client.peer_id(),
        port,
    );

    println!("Announcing to tracker...");
    println!("  URL: {}", announce_url);
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  Peer ID: {}", hex::encode(client.peer_id()));
    println!("  Port: {}", port);
    println!();
    
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
                eprintln!("  ⚠ Tracker warning: {}", failure);
            }
            if let Some(ref warning) = response.warning_message {
                eprintln!("  ⚠ Tracker warning: {}", warning);
            }
            println!("\n✓ Check the stats page: {}", TRACKER_STATS_URL);
        }
        Err(e) => {
            eprintln!("✗ Failed to announce: {}", e);
            eprintln!("Error details: {:?}", e);
            return Err(e.into());
        }
    }

    // Keep seeding
    println!("\n=== Seeding... Press Ctrl+C to stop ===");
    println!("Re-announcing every 30 seconds...\n");
    
    // Periodically re-announce
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    let mut count = 0;
    loop {
        interval.tick().await;
        count += 1;
        
        println!("[Re-announce #{}]", count);
        match tracker.announce(0, 0, 0, "update").await {
            Ok((response, _peers)) => {
                println!("  ✓ Re-announced: {} seeders, {} leechers",
                    response.complete.unwrap_or(0),
                    response.incomplete.unwrap_or(0)
                );
            }
            Err(e) => {
                eprintln!("  ✗ Re-announce failed: {}", e);
            }
        }
        println!();
    }
}
