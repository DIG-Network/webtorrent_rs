// Test: Blocklist::new() initialization
// Run with: cargo run --example test_blocklist_new

use webtorrent::Blocklist;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_blocklist_new ===");

    let blocklist = Blocklist::new();
    println!("✓ Blocklist created");

    // Test that no IPs are blocked initially
    use std::net::IpAddr;
    let test_ip: IpAddr = "192.168.1.1".parse()?;
    let is_blocked = blocklist.is_blocked(test_ip).await;
    assert!(!is_blocked, "IP should not be blocked initially");
    println!("✓ No IPs blocked initially");

    println!("=== test_blocklist_new PASSED ===");
    Ok(())
}

