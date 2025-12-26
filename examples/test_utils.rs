// Test utilities for comprehensive API permutation testing
// This module provides common helpers for creating test data, clients, and torrents

use webtorrent::{WebTorrent, WebTorrentOptions, TorrentCreator, TorrentId};
use bytes::Bytes;
use std::sync::Arc;
use tokio::time::Duration;

/// Create a test WebTorrent client with default options
pub async fn create_test_client() -> Result<WebTorrent, Box<dyn std::error::Error>> {
    let options = WebTorrentOptions::default();
    Ok(WebTorrent::new(options).await?)
}

/// Create a test WebTorrent client with custom options
pub async fn create_test_client_with_options(options: WebTorrentOptions) -> Result<WebTorrent, Box<dyn std::error::Error>> {
    Ok(WebTorrent::new(options).await?)
}

/// Create test data bytes
pub fn create_test_data(size: usize) -> Bytes {
    let pattern = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        data.push(pattern[i % pattern.len()]);
    }
    Bytes::from(data)
}

/// Create a small test file (smaller than default piece length)
pub fn create_small_test_data() -> Bytes {
    Bytes::from("This is a small test file for testing purposes.")
}

/// Create a large test file (multiple pieces)
pub fn create_large_test_data() -> Bytes {
    create_test_data(1024 * 100) // 100 KB
}

/// Create an empty test file
pub fn create_empty_test_data() -> Bytes {
    Bytes::new()
}

/// Create a test torrent from data
pub async fn create_test_torrent(
    name: &str,
    data: Bytes,
    announce: Option<Vec<String>>,
) -> Result<(Bytes, [u8; 20]), Box<dyn std::error::Error>> {
    let mut creator = TorrentCreator::new();
    if let Some(announce_list) = announce {
        creator = creator.with_announce(announce_list);
    }
    let (torrent_file, info_hash) = creator.create_from_data(name.to_string(), data).await?;
    Ok((torrent_file, info_hash))
}

/// Create a multi-file torrent
pub async fn create_multi_file_torrent(
    files: Vec<(&str, Bytes)>,
    announce: Option<Vec<String>>,
) -> Result<(Bytes, [u8; 20]), Box<dyn std::error::Error>> {
    // For now, we'll create a single-file torrent and simulate multi-file
    // In a real implementation, TorrentCreator would support multi-file creation
    let first_file = files.first().ok_or("No files provided")?;
    create_test_torrent(first_file.0, first_file.1.clone(), announce).await
}

/// Wait for a torrent to become ready
pub async fn wait_for_torrent_ready(torrent: &Arc<webtorrent::Torrent>, timeout_secs: u64) -> bool {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    
    while start.elapsed() < timeout {
        if torrent.ready().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Wait for a condition to become true
pub async fn wait_for_condition<F>(condition: F, timeout_secs: u64) -> bool
where
    F: Fn() -> bool,
{
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    
    while start.elapsed() < timeout {
        if condition() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Wait for an async condition to become true
pub async fn wait_for_async_condition<F, Fut>(condition: F, timeout_secs: u64) -> bool
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    
    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Get a test tracker URL
pub fn get_test_tracker_url() -> String {
    "http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/announce".to_string()
}

/// Create a test info hash
pub fn create_test_info_hash() -> [u8; 20] {
    let mut hash = [0u8; 20];
    hash[0] = 0x01;
    hash[1] = 0x23;
    hash[2] = 0x45;
    hash[3] = 0x67;
    // Rest are zeros for testing
    hash
}

/// Create a test peer ID
pub fn create_test_peer_id() -> [u8; 20] {
    let mut id = [0u8; 20];
    id[0..3].copy_from_slice(b"-WW");
    id[3..7].copy_from_slice(b"0100");
    id[7] = b'-';
    // Rest are random
    id
}

/// Format bytes as hex string
pub fn hex_string(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

/// Initialize tracing for test output
pub fn init_test_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("webtorrent=debug")
        .try_init();
}

