use crate::error::{Result, WebTorrentError};
use crate::wire::Wire;
use bytes::Bytes;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;
use url::Url;

const SOCKET_TIMEOUT_MS: u64 = 60000;

/// Web seed connection that converts BitTorrent block requests into HTTP range requests
pub struct WebSeedConn {
    url: String,
    conn_id: String, // Unique id to deduplicate web seeds
    wire: Arc<Wire>,
    client: Client,
    destroyed: Arc<RwLock<bool>>,
    // Store torrent metadata needed for requests
    piece_length: u64,
    files: Vec<WebSeedFile>,
}

#[derive(Clone)]
struct WebSeedFile {
    path: String,
    length: u64,
    offset: u64,
}

impl WebSeedConn {
    /// Create a new web seed connection
    pub fn new(
        url: String,
        piece_length: u64,
        files: Vec<(String, u64, u64)>, // (path, length, offset)
    ) -> Result<Self> {
        // Validate URL
        let parsed_url = Url::parse(&url).map_err(|e| {
            WebTorrentError::InvalidTorrent(format!("Invalid web seed URL: {}", e))
        })?;

        if !matches!(parsed_url.scheme(), "http" | "https") {
            return Err(WebTorrentError::InvalidTorrent(
                "Web seed URL must be http or https".to_string(),
            ));
        }

        let conn_id = url.clone();
        let wire = Arc::new(Wire::new("webSeed".to_string()));

        // Create HTTP client with timeout
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(SOCKET_TIMEOUT_MS))
            .build()
            .map_err(|e| {
                WebTorrentError::Network(format!("Failed to create HTTP client: {}", e))
            })?;

        let web_seed_files = files
            .into_iter()
            .map(|(path, length, offset)| WebSeedFile {
                path,
                length,
                offset,
            })
            .collect();

        Ok(Self {
            url,
            conn_id,
            wire,
            client,
            destroyed: Arc::new(RwLock::new(false)),
            piece_length,
            files: web_seed_files,
        })
    }

    /// Get the connection ID (URL)
    pub fn conn_id(&self) -> &str {
        &self.conn_id
    }

    /// Get the URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the wire
    pub fn wire(&self) -> Arc<Wire> {
        Arc::clone(&self.wire)
    }

    /// Initialize the web seed connection
    pub async fn init(&self, num_pieces: usize) -> Result<()> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Network("Web seed destroyed".to_string()));
        }

        // Set keep-alive
        // In JS version, this is handled by the Wire protocol

        // Web seeds always have all pieces - set bitfield
        let mut bitfield = bitvec::prelude::BitVec::new();
        bitfield.resize(num_pieces, true);
        self.wire.set_peer_pieces(bitfield).await;

        Ok(())
    }

    /// Handle a piece request by making an HTTP range request
    pub async fn http_request(
        &self,
        piece_index: usize,
        offset: usize,
        length: usize,
    ) -> Result<Bytes> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Network("Web seed destroyed".to_string()));
        }

        // Calculate piece offset
        let piece_offset = piece_index as u64 * self.piece_length;
        let range_start = piece_offset + offset as u64;
        let range_end = range_start + length as u64 - 1;

        let requests = if self.files.len() <= 1 {
            // Single-file torrent: make HTTP range request directly to the web seed URL
            vec![HttpRangeRequest {
                url: self.url.clone(),
                start: range_start,
                end: range_end,
                file_offset_in_range: 0,
            }]
        } else {
            // Multi-file torrent: add torrent folder and file name to the URL
            let mut requests = Vec::new();
            let range_end_u64 = range_end;

            for file in &self.files {
                let file_end = file.offset + file.length - 1;
                if file.offset <= range_end_u64 && file_end >= range_start {
                    let file_url = if self.url.ends_with('/') {
                        format!("{}{}", self.url, file.path)
                    } else {
                        format!("{}/{}", self.url, file.path)
                    };

                let file_start = range_start.saturating_sub(file.offset);
                let file_end_in_range = (file_end - file.offset).min(range_end_u64 - file.offset);

                    requests.push(HttpRangeRequest {
                        url: file_url,
                        start: file_start,
                        end: file_end_in_range,
                        file_offset_in_range: file.offset.saturating_sub(range_start),
                    });
                }
            }

            if requests.is_empty() {
                return Err(WebTorrentError::Network(
                    "Could not find file corresponding to web seed range request".to_string(),
                ));
            }

            requests
        };

        // Make HTTP range requests
        let mut chunks = Vec::new();
        for req in requests {
            debug!(
                "Requesting web seed url={} piece_index={} offset={} length={} start={} end={}",
                req.url, piece_index, offset, length, req.start, req.end
            );

            let response = self
                .client
                .get(&req.url)
                .header("Range", format!("bytes={}-{}", req.start, req.end))
                .header("Cache-Control", "no-store")
                .header(
                    "User-Agent",
                    format!("WebTorrent/{} (https://webtorrent.io)", crate::VERSION),
                )
                .send()
                .await
                .map_err(|e| {
                    WebTorrentError::Network(format!("Web seed HTTP request failed: {}", e))
                })?;

            if !response.status().is_success() {
                return Err(WebTorrentError::Network(format!(
                    "Web seed HTTP request failed with status: {}",
                    response.status()
                )));
            }

            let data = response
                .bytes()
                .await
                .map_err(|e| {
                    WebTorrentError::Network(format!("Failed to read web seed response: {}", e))
                })?;

            debug!("Got web seed data of length {}", data.len());

            // If this is part of a multi-file request, we need to handle the offset
            if req.file_offset_in_range > 0 {
                // This shouldn't happen in our current implementation, but handle it
                chunks.push((req.file_offset_in_range, data));
            } else {
                chunks.push((0, data));
            }
        }

        // Sort chunks by offset and concatenate
        chunks.sort_by_key(|(offset, _)| *offset);
        let mut result = BytesMut::with_capacity(length);
        for (_, chunk) in chunks {
            result.extend_from_slice(&chunk);
        }

        Ok(result.freeze())
    }

    /// Destroy the web seed connection
    pub async fn destroy(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Ok(());
        }

        *self.destroyed.write().await = true;
        self.wire.destroy().await?;

        Ok(())
    }
}

struct HttpRangeRequest {
    url: String,
    start: u64,
    end: u64,
    file_offset_in_range: u64,
}

use bytes::BytesMut;

