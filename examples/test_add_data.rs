// Test: Direct data seeding using seed() method
// Note: TorrentId doesn't have Data variant, so we use seed() method
// Run with: cargo run --example test_add_data

use webtorrent::{WebTorrent, WebTorrentOptions};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_add_data ===");

    // Test: Direct data seeding
    let test_data = Bytes::from("This is test data for direct seeding.");
    let test_name = "direct_seed_test.txt";

    println!("Seeding data directly...");
    println!("  File name: {}", test_name);
    println!("  Data size: {} bytes", test_data.len());

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    println!("✓ Client created");

    let torrent = client.seed(test_name.to_string(), test_data.clone(), None).await?;
    println!("✓ Data seeded directly");
    println!("  Torrent name: {}", torrent.name());
    println!("  Torrent info hash: {}", hex::encode(torrent.info_hash()));
    println!("  Torrent length: {} bytes", torrent.length().await);

    client.destroy().await?;
    println!("✓ Client destroyed");

    println!("=== test_add_data PASSED ===");
    Ok(())
}

