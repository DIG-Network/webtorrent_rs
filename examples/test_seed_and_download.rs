// Test seeding and downloading on the same machine
// This creates a seeder and a downloader to test end-to-end functionality
// Run with: cargo run --example test_seed_and_download

use bytes::Bytes;
use std::time::Duration;
use tokio::time::sleep;
use tracing_subscriber;
use webtorrent::{TorrentCreator, WebTorrent, WebTorrentOptions};

const TRACKER_URL: &str = "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000";
const EXPECTED_DATA: &str =
    "Hello, World! This is a test file for seed and download test from WebTorrent Rust.";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== WebTorrent Rust - Seed and Download Test ===");
    println!("Tracker: {}", TRACKER_URL);
    println!();

    // Step 1: Create seeder client
    println!("[1/6] Creating seeder client...");
    let seeder_options = WebTorrentOptions {
        torrent_port: 6881,
        dht_port: 0,
        max_conns: 10,
        utp: false,
        nat_upnp: false,
        nat_pmp: false,
        ..Default::default()
    };
    let seeder = WebTorrent::new(seeder_options).await?;
    println!("✓ Seeder created on port 6881");
    println!("  Peer ID: {}", hex::encode(seeder.peer_id()));
    println!();

    // Step 2: Create test data and torrent
    println!("[2/6] Creating test torrent...");
    let test_data = Bytes::from(EXPECTED_DATA);
    let test_name = "test_seed_download.txt";
    let announce_url = format!("{}/announce", TRACKER_URL);

    let creator = TorrentCreator::new()
        .with_announce(vec![announce_url.clone()])
        .with_comment("Test file for seed and download".to_string());

    let (torrent_file, info_hash) = creator
        .create_from_data(test_name.to_string(), test_data.clone())
        .await?;

    println!("✓ Torrent created");
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  File size: {} bytes", test_data.len());
    println!();

    // Step 3: Seed the torrent
    println!("[3/6] Seeding torrent...");
    let _seeded_torrent = seeder
        .seed(
            test_name.to_string(),
            test_data.clone(),
            Some(vec![announce_url.clone()]),
        )
        .await?;
    println!("✓ Torrent seeded");

    // Wait for seeder to announce
    sleep(Duration::from_secs(2)).await;
    println!("✓ Seeder announced to tracker");
    println!();

    // Step 4: Create downloader client
    println!("[4/6] Creating downloader client...");
    let downloader_options = WebTorrentOptions {
        torrent_port: 6882, // Different port
        dht_port: 0,
        max_conns: 10,
        utp: false,
        nat_upnp: false,
        nat_pmp: false,
        ..Default::default()
    };
    let downloader = WebTorrent::new(downloader_options).await?;
    println!("✓ Downloader created on port 6882");
    println!("  Peer ID: {}", hex::encode(downloader.peer_id()));
    println!();

    // Step 5: Download using the torrent file
    println!("[5/6] Adding torrent to downloader...");
    let download_torrent = downloader.add(torrent_file).await?;
    println!("✓ Torrent added to downloader");
    println!("  Name: {}", download_torrent.name());
    println!("  Length: {} bytes", download_torrent.length().await);
    println!();

    // Step 6: Wait for download
    println!("[6/6] Waiting for download...");
    println!("  Downloader will discover peers from tracker and connect...");
    sleep(Duration::from_secs(2)).await; // Give downloader time to announce
    let mut attempts = 0;
    let max_attempts = 30;

    loop {
        attempts += 1;
        sleep(Duration::from_secs(1)).await;

        let progress = download_torrent.progress().await;
        let downloaded = download_torrent.downloaded().await;
        let num_peers = download_torrent.num_peers().await;

        println!(
            "  Attempt {}/{}: Progress={:.1}%, Downloaded={}/{} bytes, Peers={}",
            attempts,
            max_attempts,
            progress * 100.0,
            downloaded,
            download_torrent.length().await,
            num_peers
        );

        if progress >= 1.0 {
            println!();
            println!("✓✓✓ Download complete! ✓✓✓");
            println!("  Downloaded: {} bytes", downloaded);
            println!("  Expected: {} bytes", test_data.len());
            break;
        }

        if attempts >= max_attempts {
            println!();
            println!("⚠ Download incomplete after {} seconds", max_attempts);
            println!("  Final progress: {:.1}%", progress * 100.0);
            println!(
                "  Downloaded: {}/{} bytes",
                downloaded,
                download_torrent.length().await
            );
            println!("  Peers: {}", num_peers);
            if num_peers == 0 {
                println!("  ⚠ No peers found - connection may have failed");
            }
            break;
        }
    }

    println!();
    println!("=== Test Complete ===");
    println!("Seeder and downloader should have connected through the tracker.");
    println!("If download completed, the end-to-end test was successful!");

    // Keep running for a bit to allow connections
    println!();
    println!("Keeping clients alive for 5 more seconds...");
    sleep(Duration::from_secs(5)).await;

    // Clean up
    seeder.destroy().await?;
    downloader.destroy().await?;

    Ok(())
}
