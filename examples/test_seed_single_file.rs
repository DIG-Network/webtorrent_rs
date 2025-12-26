// Test: Seed single file with default options
// Run with: cargo run --example test_seed_single_file

use webtorrent::{WebTorrent, WebTorrentOptions};
use bytes::Bytes;
use tracing_subscriber;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_seed_single_file ===");

    let test_data = Bytes::from("Test data for single file seeding.");
    let test_name = "seed_test.txt";

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    println!("✓ Client created");

    let torrent = client.seed(test_name.to_string(), test_data.clone(), None).await?;
    println!("✓ Torrent seeded");
    println!("  Name: {}", torrent.name());
    println!("  Info hash: {}", hex::encode(torrent.info_hash()));
    println!("  Length: {} bytes", torrent.length().await);

    // Wait a bit for discovery
    sleep(Duration::from_secs(2)).await;

    let peers = torrent.num_peers().await;
    println!("  Peers: {}", peers);

    client.destroy().await?;
    println!("=== test_seed_single_file PASSED ===");
    Ok(())
}

