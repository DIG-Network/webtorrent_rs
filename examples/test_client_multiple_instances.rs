// Test: Multiple WebTorrent instances on different ports
// Run with: cargo run --example test_client_multiple_instances

use webtorrent::{WebTorrent, WebTorrentOptions};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_client_multiple_instances ===");

    // Test: Multiple instances on different ports
    println!("[1/3] Creating first client on port 6884...");
    let mut options1 = WebTorrentOptions::default();
    options1.torrent_port = 6884;
    let client1 = WebTorrent::new(options1).await?;
    println!("✓ First client created");
    
    let address1 = client1.address().await;
    match address1 {
        Some((ip, port)) => println!("  Client 1 listening on {}:{}", ip, port),
        None => println!("  Client 1 not listening yet"),
    }

    println!("[2/3] Creating second client on port 6885...");
    let mut options2 = WebTorrentOptions::default();
    options2.torrent_port = 6885;
    let client2 = WebTorrent::new(options2).await?;
    println!("✓ Second client created");
    
    let address2 = client2.address().await;
    match address2 {
        Some((ip, port)) => println!("  Client 2 listening on {}:{}", ip, port),
        None => println!("  Client 2 not listening yet"),
    }

    println!("[3/3] Verifying both clients are independent...");
    let peer_id1 = client1.peer_id();
    let peer_id2 = client2.peer_id();
    assert_ne!(peer_id1, peer_id2, "Peer IDs should be different");
    println!("✓ Peer IDs are different");
    
    println!("✓ Both clients are independent");

    client1.destroy().await?;
    client2.destroy().await?;
    println!("✓ Both clients destroyed");

    println!("=== test_client_multiple_instances PASSED ===");
    Ok(())
}

