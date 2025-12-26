use crate::error::Result;
use bytes::Bytes;
use std::collections::HashMap;

/// BitTorrent extension protocol handler
pub struct ExtensionProtocol {
    extensions: HashMap<u8, ExtensionInfo>,
    #[allow(dead_code)]
    handshake: Option<Bytes>,
}

#[derive(Debug, Clone)]
pub struct ExtensionInfo {
    pub id: u8,
    pub name: String,
    pub metadata_size: Option<usize>,
}

/// ut_metadata extension
pub struct UtMetadata {
    metadata: Option<Bytes>,
    metadata_size: Option<usize>,
}

impl UtMetadata {
    pub fn new() -> Self {
        Self {
            metadata: None,
            metadata_size: None,
        }
    }

    pub fn set_metadata(&mut self, metadata: Bytes) {
        let len = metadata.len();
        self.metadata = Some(metadata);
        self.metadata_size = Some(len);
    }

    pub fn get_metadata(&self) -> Option<&Bytes> {
        self.metadata.as_ref()
    }

    pub fn fetch(&self) -> Result<()> {
        // Request metadata from peer
        Ok(())
    }

    pub fn handle_request(&self, piece: usize) -> Result<Option<Bytes>> {
        if let Some(ref metadata) = self.metadata {
            let piece_size = 16 * 1024; // 16 KB per piece
            let start = piece * piece_size;
            let end = (start + piece_size).min(metadata.len());
            if start < metadata.len() {
                return Ok(Some(metadata.slice(start..end)));
            }
        }
        Ok(None)
    }
}

/// ut_pex extension (Peer Exchange)
#[derive(Clone)]
pub struct UtPex {
    pub(crate) added: Vec<(String, u16)>,
    pub(crate) dropped: Vec<(String, u16)>,
}

impl UtPex {
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            dropped: Vec::new(),
        }
    }

    pub fn add_peer(&mut self, ip: String, port: u16) {
        self.added.push((ip, port));
    }

    pub fn drop_peer(&mut self, ip: String, port: u16) {
        self.dropped.push((ip, port));
    }

    pub fn get_added(&self) -> &[(String, u16)] {
        &self.added
    }

    pub fn get_dropped(&self) -> &[(String, u16)] {
        &self.dropped
    }

    pub fn reset(&mut self) {
        self.added.clear();
        self.dropped.clear();
    }

    pub fn encode(&self) -> Bytes {
        // Encode peer list in compact format: [added_len][added_peers][dropped_len][dropped_peers]
        let mut buf = Vec::new();
        
        // Encode added peers
        let added_count = self.added.len().min(0xFFFF) as u16;
        buf.extend_from_slice(&added_count.to_be_bytes());
        for (ip, port) in &self.added {
            if let Ok(ip_bytes) = ip.parse::<std::net::Ipv4Addr>() {
                buf.extend_from_slice(&ip_bytes.octets());
                buf.extend_from_slice(&port.to_be_bytes());
            }
        }
        
        // Encode dropped peers
        let dropped_count = self.dropped.len().min(0xFFFF) as u16;
        buf.extend_from_slice(&dropped_count.to_be_bytes());
        for (ip, port) in &self.dropped {
            if let Ok(ip_bytes) = ip.parse::<std::net::Ipv4Addr>() {
                buf.extend_from_slice(&ip_bytes.octets());
                buf.extend_from_slice(&port.to_be_bytes());
            }
        }
        
        Bytes::from(buf)
    }

    pub fn decode(data: &[u8]) -> Result<Vec<(String, u16)>> {
        let mut peers = Vec::new();
        // PEX data format: [added_peers][dropped_peers]
        // Each peer is 6 bytes: 4 bytes IP + 2 bytes port
        // First 2 bytes are length of added peers list
        if data.len() < 2 {
            return Ok(peers);
        }
        
        let added_len = u16::from_be_bytes([data[0], data[1]]) as usize;
        let added_start = 2;
        let added_end = added_start + (added_len * 6);
        
        if added_end <= data.len() {
            for chunk in data[added_start..added_end].chunks_exact(6) {
                let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
                let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                peers.push((ip, port));
            }
        }
        
        // Decode dropped peers if present
        if added_end < data.len() {
            let dropped_start = added_end;
            if dropped_start + 2 <= data.len() {
                let dropped_len = u16::from_be_bytes([data[dropped_start], data[dropped_start + 1]]) as usize;
                let dropped_end = dropped_start + 2 + (dropped_len * 6);
                
                if dropped_end <= data.len() {
                    for chunk in data[dropped_start + 2..dropped_end].chunks_exact(6) {
                        let _ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
                        let _port = u16::from_be_bytes([chunk[4], chunk[5]]);
                        // Store dropped peers separately if needed
                        // For now, we just decode them (dropped peers are handled in decode_full)
                    }
                }
            }
        }
        
        Ok(peers)
    }
    
    /// Decode PEX message with both added and dropped peers
    pub fn decode_full(data: &[u8]) -> Result<(Vec<(String, u16)>, Vec<(String, u16)>)> {
        let mut added = Vec::new();
        let mut dropped = Vec::new();
        
        if data.len() < 2 {
            return Ok((added, dropped));
        }
        
        let added_len = u16::from_be_bytes([data[0], data[1]]) as usize;
        let added_start = 2;
        let added_end = added_start + (added_len * 6);
        
        if added_end <= data.len() {
            for chunk in data[added_start..added_end].chunks_exact(6) {
                let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
                let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                added.push((ip, port));
            }
        }
        
        // Decode dropped peers
        if added_end < data.len() {
            let dropped_start = added_end;
            if dropped_start + 2 <= data.len() {
                let dropped_len = u16::from_be_bytes([data[dropped_start], data[dropped_start + 1]]) as usize;
                let dropped_end = dropped_start + 2 + (dropped_len * 6);
                
                if dropped_end <= data.len() {
                    for chunk in data[dropped_start + 2..dropped_end].chunks_exact(6) {
                        let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
                        let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                        dropped.push((ip, port));
                    }
                }
            }
        }
        
        Ok((added, dropped))
    }
}

impl ExtensionProtocol {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
            handshake: None,
        }
    }

    pub fn register_extension(&mut self, id: u8, name: String) {
        self.extensions.insert(id, ExtensionInfo {
            id,
            name,
            metadata_size: None,
        });
    }

    pub fn get_extension_id(&self, name: &str) -> Option<u8> {
        self.extensions.values()
            .find(|ext| ext.name == name)
            .map(|ext| ext.id)
    }
}

