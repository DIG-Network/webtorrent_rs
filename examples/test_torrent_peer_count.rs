// Test: num_peers() with various peer sources
// Run with: cargo run --example test_torrent_peer_count

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentId, TorrentCreator};
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_torrent_peer_count ===");

    let test_data = Bytes::from("Test data for peer count.");
    let creator = TorrentCreator::new();
    let (torrent_file, _) = creator.create_from_data("test.txt".to_string(), test_data).await?;

    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    let torrent = client.add(TorrentId::TorrentFile(torrent_file)).await?;
    
    // Check initial peer count
    let num_peers = torrent.num_peers().await;
    println!("Initial peer count: {}", num_peers);
    assert_eq!(num_peers, 0, "Should start with 0 peers");

    // Start discovery to potentially find peers
    torrent.start_discovery().await?;
    
    // Check peer count after discovery
    let num_peers2 = torrent.num_peers().await;
    println!("Peer count after discovery: {}", num_peers2);

    println!("✓ num_peers() works");

    client.destroy().await?;
    println!("=== test_torrent_peer_count PASSED ===");
    Ok(())
}

