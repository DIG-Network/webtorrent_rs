use crate::error::{Result, WebTorrentError};
use std::collections::HashMap;
use url::Url;
use serde::Deserialize;

/// Tracker response
#[derive(Debug, Clone, Deserialize)]
pub struct TrackerResponse {
    #[serde(rename = "interval")]
    pub interval: Option<u64>,
    #[serde(rename = "min interval")]
    pub min_interval: Option<u64>,
    #[serde(rename = "tracker id")]
    pub tracker_id: Option<String>,
    #[serde(rename = "complete")]
    pub complete: Option<u64>,
    #[serde(rename = "incomplete")]
    pub incomplete: Option<u64>,
    #[serde(rename = "peers")]
    pub peers: TrackerPeers,
    #[serde(rename = "failure reason")]
    pub failure_reason: Option<String>,
    #[serde(rename = "warning message")]
    pub warning_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TrackerPeers {
    String(String), // Compact format (binary)
    List(Vec<TrackerPeer>), // Dictionary format
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrackerPeer {
    pub peer_id: Option<String>,
    pub ip: String,
    pub port: u16,
}

/// Tracker client
pub struct TrackerClient {
    announce_url: String,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
}

impl TrackerClient {
    pub fn new(announce_url: String, info_hash: [u8; 20], peer_id: [u8; 20], port: u16) -> Self {
        Self {
            announce_url,
            info_hash,
            peer_id,
            port,
        }
    }

    /// Announce to tracker and return peers
    pub async fn announce(
        &self,
        uploaded: u64,
        downloaded: u64,
        left: u64,
        event: &str,
    ) -> Result<(TrackerResponse, Vec<(String, u16)>)> {
        // Ensure we have /announce endpoint
        // Handle various URL formats:
        // - http://tracker.com/announce (already correct)
        // - http://tracker.com/ (needs /announce)
        // - http://tracker.com (needs /announce)
        // - http://tracker.com/stats (needs /announce)
        let announce_url = if self.announce_url.ends_with("/announce") {
            self.announce_url.clone()
        } else if self.announce_url.ends_with("/stats") {
            self.announce_url.replace("/stats", "/announce")
        } else if self.announce_url.ends_with("/") {
            format!("{}announce", self.announce_url)
        } else {
            format!("{}/announce", self.announce_url)
        };

        // For BitTorrent, we need to manually percent-encode the info_hash and peer_id
        // Each byte becomes %XX where XX is the hex representation
        let mut info_hash_encoded = String::with_capacity(60); // 20 bytes * 3 chars
        for &byte in &self.info_hash {
            info_hash_encoded.push_str(&format!("%{:02X}", byte));
        }
        
        let mut peer_id_encoded = String::with_capacity(60);
        for &byte in &self.peer_id {
            peer_id_encoded.push_str(&format!("%{:02X}", byte));
        }
        
        // Build query string manually to avoid double-encoding
        let query = format!(
            "info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact=1&event={}",
            info_hash_encoded, peer_id_encoded, self.port, uploaded, downloaded, left, event
        );
        
        let full_url = if announce_url.contains('?') {
            format!("{}&{}", announce_url, query)
        } else {
            format!("{}?{}", announce_url, query)
        };
        
        let url = Url::parse(&full_url)?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        let response = client.get(url.as_str()).send().await?;
        let data = response.bytes().await?;

        // Parse bencoded response
        use crate::bencode_parser::parse_bencode;
        let (bencode, _) = parse_bencode(&data)?;

        // Check for failure
        if let Some(failure) = bencode.get(b"failure reason") {
            if let Some(reason) = failure.as_string() {
                return Err(WebTorrentError::Discovery(format!("Tracker error: {}", reason)));
            }
        }

        // Parse peers (compact format)
        let peers = bencode.get(b"peers")
            .and_then(|p| {
                // Try compact format first (bytes)
                if let Some(bytes) = p.as_bytes() {
                    if bytes.len() % 6 == 0 {
                        Some(bytes.chunks_exact(6)
                            .map(|chunk| {
                                let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
                                let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                                (ip, port)
                            })
                            .collect::<Vec<_>>())
                    } else {
                        None
                    }
                } else if let Some(list) = p.as_list() {
                    // Try dictionary format (list of dicts)
                    Some(list.iter()
                        .filter_map(|peer_dict| {
                            let ip = peer_dict.get(b"ip").and_then(|i| i.as_string())?;
                            let port = peer_dict.get(b"port").and_then(|p| p.as_integer())? as u16;
                            Some((ip, port))
                        })
                        .collect::<Vec<_>>())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        
        // Debug: log what we found
        if !peers.is_empty() {
            tracing::info!("Tracker returned {} peers", peers.len());
            for (ip, port) in &peers {
                tracing::debug!("  Peer: {}:{}", ip, port);
            }
        } else {
            tracing::debug!("Tracker returned 0 peers (response may be empty or in unexpected format)");
        }

        let interval = bencode.get(b"interval")
            .and_then(|i| i.as_integer())
            .map(|i| i as u64)
            .unwrap_or(1800);

        // Convert peers to TrackerPeers format
        let peers_str = if peers.is_empty() {
            String::new()
        } else {
            // For now, just use empty string - peers are available via the response
            String::new()
        };

        let response = TrackerResponse {
            interval: Some(interval),
            min_interval: bencode.get(b"min interval")
                .and_then(|i| i.as_integer())
                .map(|i| i as u64),
            tracker_id: bencode.get(b"tracker id")
                .and_then(|t| t.as_string()),
            complete: bencode.get(b"complete")
                .and_then(|c| c.as_integer())
                .map(|c| c as u64),
            incomplete: bencode.get(b"incomplete")
                .and_then(|i| i.as_integer())
                .map(|i| i as u64),
            peers: TrackerPeers::String(peers_str),
            failure_reason: bencode.get(b"failure reason")
                .and_then(|f| f.as_string()),
            warning_message: bencode.get(b"warning message")
                .and_then(|w| w.as_string()),
        };
        
        Ok((response, peers))
    }

    /// Scrape tracker
    pub async fn scrape(&self) -> Result<HashMap<String, (u64, u64, u64)>> {
        // Convert announce URL to scrape URL
        let scrape_url = self.announce_url
            .replace("/announce", "/scrape")
            .replace("announce", "scrape");

        let mut url = Url::parse(&scrape_url)?;
        url.query_pairs_mut()
            .append_pair("info_hash", &urlencoding::encode_binary(&self.info_hash));

        let response = reqwest::get(url.as_str()).await?;
        let data = response.bytes().await?;

        // Parse bencoded response
        use crate::bencode_parser::parse_bencode;
        use bytes::Bytes;
        let (bencode, _) = parse_bencode(&data)?;

        let mut result = HashMap::new();
        let info_hash_str = hex::encode(self.info_hash);

        if let Some(files) = bencode.get(b"files") {
            if let Some(files_dict) = files.as_dict() {
                let key_bytes = Bytes::copy_from_slice(info_hash_str.as_bytes());
                if let Some(file_info) = files_dict.get(&key_bytes) {
                    let complete = file_info.get(b"complete")
                        .and_then(|c| c.as_integer())
                        .map(|c| c as u64)
                        .unwrap_or(0);
                    let downloaded = file_info.get(b"downloaded")
                        .and_then(|d| d.as_integer())
                        .map(|d| d as u64)
                        .unwrap_or(0);
                    let incomplete = file_info.get(b"incomplete")
                        .and_then(|i| i.as_integer())
                        .map(|i| i as u64)
                        .unwrap_or(0);
                    result.insert(info_hash_str, (complete, downloaded, incomplete));
                }
            }
        }

        Ok(result)
    }
}

