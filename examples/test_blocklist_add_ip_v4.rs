// Test: add_ip() IPv4 addresses
// Run with: cargo run --example test_blocklist_add_ip_v4

use webtorrent::Blocklist;
use tracing_subscriber;
use std::net::IpAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_blocklist_add_ip_v4 ===");

    let blocklist = Blocklist::new();
    
    let test_ip: IpAddr = "192.168.1.1".parse()?;
    blocklist.add_ip(test_ip).await;
    println!("✓ IPv4 address added to blocklist");

    let is_blocked = blocklist.is_blocked(test_ip).await;
    assert!(is_blocked, "IP should be blocked");
    println!("✓ IP is blocked");

    println!("=== test_blocklist_add_ip_v4 PASSED ===");
    Ok(())
}

