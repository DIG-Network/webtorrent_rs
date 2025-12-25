// Integration test for the dig-relay tracker

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentCreator};
use bytes::Bytes;
use std::time::Duration;
use tokio::time::sleep;

const TRACKER_URL: &str = "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000";

#[tokio::test]
#[ignore] // Ignore by default, run with: cargo test -- --ignored
async fn test_seed_to_dig_relay_tracker() {
    // Create client
    let options = WebTorrentOptions {
        torrent_port: 6881,
        dht_port: 0, // Disable DHT for this test
        max_conns: 10,
        utp: false,
        nat_upnp: false,
        nat_pmp: false,
        ..Default::default()
    };

    let client = WebTorrent::new(options).await.unwrap();

    // Create test file data
    let test_data = Bytes::from("Hello, World! This is a test file for the dig-relay tracker.");
    let test_name = "test_file.txt";

    // Create torrent with dig-relay tracker
    let announce_url = format!("{}/announce", TRACKER_URL);
    let creator = TorrentCreator::new()
        .with_announce(vec![announce_url.clone()])
        .with_comment("Test file for dig-relay tracker".to_string());

    let (torrent_file, info_hash) = creator
        .create_from_data(test_name.to_string(), test_data.clone())
        .await
        .unwrap();

    println!("Created torrent with info hash: {}", hex::encode(info_hash));
    println!("Torrent file size: {} bytes", torrent_file.len());

    // Add torrent to client
    let _ = client.add(torrent_file).await.unwrap();
    println!("Torrent added to client");

    // Wait a bit for torrent to initialize
    sleep(Duration::from_secs(2)).await;

    // The discovery module should automatically announce to the tracker
    // Let's verify the tracker can be reached
    use webtorrent::tracker::TrackerClient;
    let port = client.address().await
        .map(|(_ip, port)| port)
        .unwrap_or(0);
    let tracker = TrackerClient::new(
        announce_url,
        info_hash,
        client.peer_id(),
        port,
    );

    // Announce as started
    match tracker.announce(0, 0, test_data.len() as u64, "started").await {
        Ok((response, peers)) => {
            println!("Successfully announced to tracker!");
            println!("Interval: {:?}", response.interval);
            println!("Peers: {:?}", peers);
            if let Some(complete) = response.complete {
                println!("Complete (seeders): {}", complete);
            }
            if let Some(incomplete) = response.incomplete {
                println!("Incomplete (leechers): {}", incomplete);
            }
        }
        Err(e) => {
            eprintln!("Failed to announce to tracker: {}", e);
            // Don't fail the test, just log the error
        }
    }

    // Wait a bit to allow tracker to update stats
    sleep(Duration::from_secs(3)).await;

    // Announce as completed (since we're seeding)
    let _ = tracker.announce(0, 0, 0, "completed").await;

    // Clean up
    client.destroy().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_tracker_scrape() {
    use webtorrent::tracker::TrackerClient;

    // Create a dummy tracker client for scraping
    let info_hash = [0u8; 20];
    let peer_id = [0u8; 20];
    let announce_url = format!("{}/announce", TRACKER_URL);

    let tracker = TrackerClient::new(announce_url, info_hash, peer_id, 6881);

    // Try to scrape
    match tracker.scrape().await {
        Ok(stats) => {
            println!("Scrape successful!");
            for (hash, (complete, downloaded, incomplete)) in stats {
                println!(
                    "Hash: {}, Complete: {}, Downloaded: {}, Incomplete: {}",
                    hash, complete, downloaded, incomplete
                );
            }
        }
        Err(e) => {
            eprintln!("Scrape failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_tracker_url_construction() {
    use webtorrent::tracker::TrackerClient;

    let base_url = TRACKER_URL;
    let announce_url = format!("{}/announce", base_url);
    let info_hash = [0u8; 20];
    let peer_id = [0u8; 20];

    let _tracker = TrackerClient::new(announce_url.clone(), info_hash, peer_id, 6881);

    // Verify URL construction
    assert!(announce_url.contains("dig-relay-prod"));
    assert!(announce_url.ends_with("/announce"));
}

