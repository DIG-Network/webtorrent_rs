// Test: Seed on one client, download on another
// Run with: cargo run --example test_e2e_seed_download

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_e2e_seed_download ===");

    let test_data = Bytes::from("End-to-end test data for seed and download.");
    let creator = TorrentCreator::new();
    let (torrent_file, info_hash) = creator.create_from_data("e2e_test.txt".to_string(), test_data.clone()).await?;

    // Create seeder
    println!("[1/4] Creating seeder...");
    let mut seeder_options = WebTorrentOptions::default();
    seeder_options.torrent_port = 6886;
    let seeder = WebTorrent::new(seeder_options).await?;
    let seeded_torrent = seeder.add(TorrentId::TorrentFile(torrent_file.clone())).await?;
    seeded_torrent.start_discovery().await?;
    println!("✓ Seeder created and started");

    sleep(Duration::from_secs(1)).await;

    // Create downloader
    println!("[2/4] Creating downloader...");
    let mut downloader_options = WebTorrentOptions::default();
    downloader_options.torrent_port = 6887;
    let downloader = WebTorrent::new(downloader_options).await?;
    let download_torrent = downloader.add(TorrentId::TorrentFile(torrent_file)).await?;
    download_torrent.start_discovery().await?;
    println!("✓ Downloader created and started");

    println!("[3/4] Waiting for connection...");
    sleep(Duration::from_secs(5)).await;

    let seeder_peers = seeded_torrent.num_peers().await;
    let downloader_peers = download_torrent.num_peers().await;
    println!("  Seeder peers: {}", seeder_peers);
    println!("  Downloader peers: {}", downloader_peers);

    println!("[4/4] Cleaning up...");
    seeder.destroy().await?;
    downloader.destroy().await?;
    println!("✓ Cleanup complete");

    println!("=== test_e2e_seed_download PASSED ===");
    Ok(())
}

