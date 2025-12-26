// Test: Various piece_length values
// Run with: cargo run --example test_creator_piece_length

use webtorrent::TorrentCreator;
use bytes::Bytes;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("=== test_creator_piece_length ===");

    let test_data = Bytes::from("This is test data for piece length testing. ".repeat(100));

    // Test various piece lengths
    let piece_lengths = vec![
        (16 * 1024, "16KB"),
        (32 * 1024, "32KB"),
        (64 * 1024, "64KB"),
        (256 * 1024, "256KB"),
        (512 * 1024, "512KB"),
        (1024 * 1024, "1MB"),
        (2 * 1024 * 1024, "2MB"),
    ];

    for (length, name) in piece_lengths {
        println!("Testing piece length: {} ({})", name, length);
        let creator = TorrentCreator::new().with_piece_length(length);
        let (torrent_file, info_hash) = creator.create_from_data("test.txt".to_string(), test_data.clone()).await?;
        println!("  ✓ Created with info hash: {}", hex::encode(info_hash));
    }

    println!("=== test_creator_piece_length PASSED ===");
    Ok(())
}

