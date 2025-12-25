// Common test utilities
#![allow(dead_code)] // Test utility functions may not be used in all test files

use webtorrent::{WebTorrent, WebTorrentOptions};
use std::sync::Arc;

#[allow(dead_code)]
pub async fn create_test_client() -> Arc<WebTorrent> {
    let options = WebTorrentOptions {
        torrent_port: 0, // Use random port
        dht_port: 0,
        max_conns: 10,
        utp: false, // Disable for testing
        nat_upnp: false, // Disable for testing
        nat_pmp: false, // Disable for testing
        ..Default::default()
    };
    
    Arc::new(WebTorrent::new(options).await.unwrap())
}

pub fn create_test_torrent_data() -> Vec<u8> {
    // Create a minimal valid torrent file for testing
    // This is a single-file torrent with 1 piece
    let mut data = Vec::new();
    
    // d8:announce41:http://tracker.example.com:8080/announce
    data.extend_from_slice(b"d8:announce41:http://tracker.example.com:8080/announce");
    
    // 4:infod
    data.extend_from_slice(b"4:infod");
    
    // 4:name4:test
    data.extend_from_slice(b"4:name4:test");
    
    // 12:piece lengthi16384e
    data.extend_from_slice(b"12:piece lengthi16384e");
    
    // 6:pieces20:XXXXXXXXXXXXXXXXXXXX
    data.extend_from_slice(b"6:pieces20:");
    data.extend_from_slice(&[0u8; 20]); // Placeholder hash
    
    // 6:lengthi1024e
    data.extend_from_slice(b"6:lengthi1024e");
    
    // ee (end dict, end dict)
    data.extend_from_slice(b"ee");
    
    data
}

#[allow(dead_code)]
pub fn create_test_info_hash() -> [u8; 20] {
    let mut hash = [0u8; 20];
    hash[0] = 0x01;
    hash[1] = 0x23;
    hash[2] = 0x45;
    hash[3] = 0x67;
    hash[4] = 0x89;
    hash[5] = 0xab;
    hash[6] = 0xcd;
    hash[7] = 0xef;
    // Rest is zeros for testing
    hash
}

