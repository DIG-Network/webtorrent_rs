// Example: Seed a file to the dig-relay tracker
// Run with: cargo run --example seed_to_dig_relay

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

    println!("WebTorrent Rust - Seed to dig-relay tracker example");
    println!("Tracker: {}", TRACKER_URL);
    println!("Stats page: {}", TRACKER_STATS_URL);

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

    let client = WebTorrent::new(options).await?;
    println!("Client created");

    // Create test file data
    let test_data = Bytes::from(
        "Hello, World! This is a test file seeded to the dig-relay tracker from WebTorrent Rust.",
    );
    let test_name = "test_file.txt";

    println!("Creating torrent...");

    // Create torrent with dig-relay tracker
    let announce_url = format!("{}/announce", TRACKER_URL);
    let creator = TorrentCreator::new()
        .with_announce(vec![announce_url.clone()])
        .with_comment("Test file from WebTorrent Rust".to_string());

    let (torrent_file, info_hash) = creator
        .create_from_data(test_name.to_string(), test_data.clone())
        .await?;

    println!("Torrent created!");
    println!("Info hash: {}", hex::encode(info_hash));
    println!("Torrent file size: {} bytes", torrent_file.len());

    // Add torrent to client
    println!("Adding torrent to client...");
    let torrent = client.add(torrent_file).await?;
    println!("Torrent added: {}", torrent.name());

    // Wait for torrent to initialize
    sleep(Duration::from_secs(2)).await;

    // Announce to tracker
    use webtorrent::tracker::TrackerClient;
    let port = client.address().await.map(|(_ip, port)| port).unwrap_or(0);
    let tracker = TrackerClient::new(announce_url, info_hash, client.peer_id(), port);

    println!("Announcing to tracker...");
    match tracker
        .announce(0, 0, test_data.len() as u64, "started")
        .await
    {
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
            println!("\nCheck the stats page: {}", TRACKER_STATS_URL);
        }
        Err(e) => {
            eprintln!("✗ Failed to announce: {}", e);
            return Err(e.into());
        }
    }

    // Keep seeding
    println!("\nSeeding... Press Ctrl+C to stop");

    // Periodically re-announce
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;

        match tracker.announce(0, 0, 0, "update").await {
            Ok((response, _peers)) => {
                println!(
                    "Re-announced: {} seeders, {} leechers",
                    response.complete.unwrap_or(0),
                    response.incomplete.unwrap_or(0)
                );
            }
            Err(e) => {
                eprintln!("Re-announce failed: {}", e);
            }
        }
    }
}

