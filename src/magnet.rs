use crate::error::{Result, WebTorrentError};
use url::Url;

/// Parsed magnet URI
#[derive(Debug, Clone)]
pub struct MagnetUri {
    pub info_hash: [u8; 20],
    pub display_name: Option<String>,
    pub trackers: Vec<String>,
    pub exact_length: Option<u64>,
    pub exact_source: Option<String>,
    pub keywords: Vec<String>,
    pub acceptable_source: Vec<String>,
}

impl MagnetUri {
    /// Parse a magnet URI string
    pub fn parse(uri: &str) -> Result<Self> {
        if !uri.starts_with("magnet:?") {
            return Err(WebTorrentError::InvalidTorrent(
                "Invalid magnet URI format".to_string(),
            ));
        }

        let url = Url::parse(uri)?;
        let mut info_hash = None;
        let mut display_name = None;
        let mut trackers = Vec::new();
        let mut exact_length = None;
        let mut exact_source = None;
        let mut keywords = Vec::new();
        let mut acceptable_source = Vec::new();

        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "xt" => {
                    // Extract info hash from urn:btih:...
                    if value.starts_with("urn:btih:") {
                        let hash_str = value.trim_start_matches("urn:btih:");
                        let hash_bytes = if hash_str.len() == 40 {
                            // Hex encoding
                            hex::decode(hash_str)
                                .map_err(|e| WebTorrentError::InvalidTorrent(format!("Invalid hex hash: {}", e)))?
                        } else if hash_str.len() == 32 {
                            // Base32 encoding
                            base32::decode(base32::Alphabet::RFC4648 { padding: false }, hash_str)
                                .ok_or_else(|| WebTorrentError::InvalidTorrent("Invalid base32 hash".to_string()))?
                        } else {
                            return Err(WebTorrentError::InvalidTorrent(
                                "Invalid hash length".to_string(),
                            ));
                        };

                        if hash_bytes.len() == 20 {
                            let mut hash = [0u8; 20];
                            hash.copy_from_slice(&hash_bytes);
                            info_hash = Some(hash);
                        }
                    }
                }
                "dn" => {
                    // Display name
                    display_name = Some(value.to_string());
                }
                "tr" => {
                    // Tracker
                    trackers.push(value.to_string());
                }
                "xl" => {
                    // Exact length
                    exact_length = value.parse().ok();
                }
                "xs" => {
                    // Exact source
                    exact_source = Some(value.to_string());
                }
                "kt" => {
                    // Keywords
                    keywords.extend(value.split('+').map(|s| s.to_string()));
                }
                "as" => {
                    // Acceptable source
                    acceptable_source.push(value.to_string());
                }
                _ => {}
            }
        }

        let info_hash = info_hash.ok_or_else(|| {
            WebTorrentError::InvalidTorrent("Magnet URI missing info hash (xt)".to_string())
        })?;

        Ok(Self {
            info_hash,
            display_name,
            trackers,
            exact_length,
            exact_source,
            keywords,
            acceptable_source,
        })
    }

    /// Convert to magnet URI string
    pub fn to_string(&self) -> String {
        let mut parts = Vec::new();
        parts.push(format!("magnet:?xt=urn:btih:{}", hex::encode(self.info_hash)));

        if let Some(ref name) = self.display_name {
            parts.push(format!("dn={}", urlencoding::encode(name)));
        }

        for tracker in &self.trackers {
            parts.push(format!("tr={}", urlencoding::encode(tracker)));
        }

        if let Some(length) = self.exact_length {
            parts.push(format!("xl={}", length));
        }

        if let Some(ref source) = self.exact_source {
            parts.push(format!("xs={}", urlencoding::encode(source)));
        }

        if !self.keywords.is_empty() {
            parts.push(format!("kt={}", self.keywords.join("+")));
        }

        for source in &self.acceptable_source {
            parts.push(format!("as={}", urlencoding::encode(source)));
        }

        parts.join("&")
    }
}

