// Test: is_blocked() verification
// Run with: cargo run --example test_blocklist_is_blocked

use webtorrent::Blocklist;
use tracing_subscriber;
use std::net::IpAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_blocklist_is_blocked ===");

    let blocklist = Blocklist::new();
    
    let test_ip: IpAddr = "10.0.0.1".parse()?;
    
    // Initially not blocked
    assert!(!blocklist.is_blocked(test_ip).await, "IP should not be blocked");
    println!("✓ IP not blocked initially");

    // Add and check
    blocklist.add_ip(test_ip).await;
    assert!(blocklist.is_blocked(test_ip).await, "IP should be blocked");
    println!("✓ IP is blocked after adding");

    // Remove and check
    blocklist.remove_ip(test_ip).await;
    assert!(!blocklist.is_blocked(test_ip).await, "IP should not be blocked after removal");
    println!("✓ IP not blocked after removal");

    println!("=== test_blocklist_is_blocked PASSED ===");
    Ok(())
}

