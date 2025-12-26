// Test: blocklist option with various formats
// Run with: cargo run --example test_client_blocklist

use webtorrent::{WebTorrent, WebTorrentOptions, Blocklist};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_client_blocklist ===");

    // Test: Blocklist from string
    println!("[1/2] Testing blocklist from string...");
    let blocklist_data = "192.168.1.1\n10.0.0.1\n";
    let mut options = WebTorrentOptions::default();
    options.blocklist = Some(blocklist_data.to_string());
    
    let client1 = WebTorrent::new(options).await?;
    println!("✓ Client with blocklist string created");
    client1.destroy().await?;

    // Test: Blocklist from file path (if file exists)
    println!("[2/2] Testing blocklist from file path...");
    // Note: This would require an actual blocklist file
    // For now, we'll just test that the option is accepted
    let mut options = WebTorrentOptions::default();
    options.blocklist = Some("/tmp/test_blocklist.txt".to_string());
    
    let client2 = WebTorrent::new(options).await?;
    println!("✓ Client with blocklist file path created");
    client2.destroy().await?;

    println!("=== test_client_blocklist PASSED ===");
    Ok(())
}

