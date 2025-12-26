// Test: create_from_path with existing file
// Note: TorrentCreator doesn't have create_from_path, so this test documents the limitation
// Run with: cargo run --example test_creator_from_path

use webtorrent::TorrentCreator;
use bytes::Bytes;
use tracing_subscriber;
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_creator_from_path ===");

    // Note: TorrentCreator doesn't have create_from_path method
    // This test demonstrates the workaround: read file and use create_from_data
    
    println!("[Note] TorrentCreator doesn't have create_from_path method");
    println!("Workaround: Read file and use create_from_data");
    
    // Create a temporary test file
    use std::env;
    let temp_dir = env::temp_dir();
    let test_file_path = temp_dir.join("test_creator_file.txt");
    let test_content = "This is a test file for create_from_path testing.";
    fs::write(&test_file_path, test_content)?;
    let test_file_path_str = test_file_path.to_string_lossy().to_string();
    println!("✓ Created test file: {}", test_file_path_str);

    // Read the file
    let file_data = fs::read(&test_file_path)?;
    let file_bytes = Bytes::from(file_data);
    
    // Create torrent from the data
    let creator = TorrentCreator::new();
    let file_name = test_file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("test_file.txt");
    
    let (_torrent_file, info_hash) = creator.create_from_data(file_name.to_string(), file_bytes).await?;
    
    println!("✓ Torrent created from file path (via workaround)");
    println!("  Info hash: {}", hex::encode(info_hash));
    println!("  File name: {}", file_name);

    // Clean up
    fs::remove_file(&test_file_path)?;
    println!("✓ Test file cleaned up");

    println!("=== test_creator_from_path PASSED ===");
    Ok(())
}

