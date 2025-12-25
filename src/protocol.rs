// BitTorrent protocol implementation
// This module will handle the BitTorrent protocol messages and handshakes

use crate::error::Result;
use bytes::{Bytes, BytesMut, BufMut};

/// BitTorrent protocol message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
    Port = 9,
    Extended = 20,
}

/// BitTorrent handshake
pub struct Handshake {
    pub protocol: String,
    pub reserved: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl Handshake {
    pub fn new(info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
        Self {
            protocol: "BitTorrent protocol".to_string(),
            reserved: [0u8; 8],
            info_hash,
            peer_id,
        }
    }

    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(68);
        buf.put_u8(self.protocol.len() as u8);
        buf.put_slice(self.protocol.as_bytes());
        buf.put_slice(&self.reserved);
        buf.put_slice(&self.info_hash);
        buf.put_slice(&self.peer_id);
        buf.freeze()
    }

    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 68 {
            return Err(crate::error::WebTorrentError::Protocol(
                "Handshake too short".to_string(),
            ));
        }

        let protocol_len = data[0] as usize;
        if protocol_len == 0 || protocol_len > 20 {
            return Err(crate::error::WebTorrentError::Protocol(
                "Invalid protocol length".to_string(),
            ));
        }

        let protocol = String::from_utf8_lossy(&data[1..=protocol_len]).to_string();
        let reserved = data[protocol_len + 1..protocol_len + 9]
            .try_into()
            .map_err(|_| crate::error::WebTorrentError::Protocol(
                "Invalid reserved bytes length".to_string()
            ))?;
        let info_hash = data[protocol_len + 9..protocol_len + 29]
            .try_into()
            .map_err(|_| crate::error::WebTorrentError::Protocol(
                "Invalid info hash length".to_string()
            ))?;
        let peer_id = data[protocol_len + 29..protocol_len + 49]
            .try_into()
            .map_err(|_| crate::error::WebTorrentError::Protocol(
                "Invalid peer ID length".to_string()
            ))?;

        Ok(Self {
            protocol,
            reserved,
            info_hash,
            peer_id,
        })
    }
}

