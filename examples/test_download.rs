// Test downloading a torrent from the tracker
// Run with: cargo run --example test_download
// Make sure the seeder is running first: cargo run --example test_seeder_quick

use std::time::Duration;
use tokio::time::sleep;
use tracing_subscriber;
use webtorrent::{WebTorrent, WebTorrentOptions};

const TRACKER_URL: &str = "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000";
// Info hash from the seeder test - update this if you change the test data
const TEST_INFO_HASH: &str = "6ae911a1ea51a7ce62d817804287da0ef4e338f9";
const EXPECTED_DATA: &str =
    "Hello, World! This is a test file seeded to the dig-relay tracker from WebTorrent Rust.";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== WebTorrent Rust - Download Test ===");
    println!("Tracker: {}", TRACKER_URL);
    println!("Info Hash: {}", TEST_INFO_HASH);
    println!();

    // Parse info hash
    let info_hash_bytes = hex::decode(TEST_INFO_HASH)?;
    if info_hash_bytes.len() != 20 {
        return Err("Invalid info hash length".into());
    }
    let mut info_hash = [0u8; 20];
    info_hash.copy_from_slice(&info_hash_bytes);

    // Create download client (use different port to avoid conflicts)
    println!("[1/5] Creating download client...");
    let options = WebTorrentOptions {
        torrent_port: 6882, // Different port from seeder (6881)
        dht_port: 0,
        max_conns: 10,
        utp: false,
        nat_upnp: false,
        nat_pmp: false,
        ..Default::default()
    };

    let client = WebTorrent::new(options).await?;
    println!("✓ Download client created");
    println!("  Peer ID: {}", hex::encode(client.peer_id()));
    println!();

    // Add torrent for download using info hash
    // Note: For a full download test, we would need the torrent file or
    // implement ut_metadata extension. For now, we'll use the info hash directly.
    println!("[2/5] Adding torrent for download using info hash...");
    let torrent = client.add(info_hash).await?;
    println!("✓ Torrent added: {}", torrent.name());
    println!("  Info hash: {}", hex::encode(torrent.info_hash()));
    let length = torrent.length().await;
    println!("  Length: {} bytes", length);
    println!();

    // Wait for peers and download
    println!("[3/5] Waiting for peers and download...");
    let mut attempts = 0;
    let max_attempts = 30; // 30 seconds

    loop {
        attempts += 1;
        sleep(Duration::from_secs(1)).await;

        // Access ready state through the torrent's internal structure
        // Note: This is a simplified check - in production, you'd use proper accessors
        let progress = torrent.progress().await;
        let downloaded = torrent.downloaded().await;
        let num_peers = torrent.num_peers().await;

        println!(
            "  Attempt {}/{}: Progress={:.1}%, Downloaded={} bytes, Peers={}",
            attempts,
            max_attempts,
            progress * 100.0,
            downloaded,
            num_peers
        );

        if progress >= 1.0 {
            println!("✓ Download complete!");
            break;
        }

        if attempts >= max_attempts {
            println!("⚠ Timeout waiting for download to complete");
            println!("  Final progress: {:.1}%", progress * 100.0);
            println!("  Peers found: {}", num_peers);
            if num_peers == 0 {
                println!("  ⚠ No peers found. Make sure the seeder is running!");
                println!("  Run: cargo run --example test_seeder_quick");
            }
            break;
        }
    }
    println!();

    // Verify downloaded data
    println!("[4/5] Verifying downloaded data...");
    let progress = torrent.progress().await;

    if progress >= 1.0 {
        // Get the downloaded data from the store
        // Note: This is a simplified check - in a real implementation,
        // you would read the data from the store
        println!("✓ Download completed successfully!");
        println!("  Progress: {:.1}%", progress * 100.0);
        println!("  Downloaded: {} bytes", torrent.downloaded().await);
        println!("  Expected: {} bytes", EXPECTED_DATA.len());

        // Check if we have the data
        // In a full implementation, we would read from the store here
        println!();
        println!("⚠ Note: Data verification requires reading from the store.");
        println!("  The torrent download logic is working if progress reached 100%.");
    } else {
        println!("✗ Download incomplete");
        println!("  Progress: {:.1}%", progress * 100.0);
        let downloaded = torrent.downloaded().await;
        let total_length = torrent.length().await;
        println!(
            "  Downloaded: {} bytes / {} bytes",
            downloaded, total_length
        );

        if torrent.num_peers().await == 0 {
            println!();
            println!("⚠ No peers found. Possible issues:");
            println!("  1. Seeder is not running");
            println!("  2. Tracker is not returning peers");
            println!("  3. Network/firewall issues");
        }
    }

    println!();
    println!("=== Test Complete ===");

    // Clean up
    client.destroy().await?;

    Ok(())
}
