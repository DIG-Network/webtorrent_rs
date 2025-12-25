use crate::error::Result;
use bytes::Bytes;
use sha1::{Sha1, Digest};
use std::collections::HashMap;
use crate::bencode_parser::BencodeValue;

/// Create a torrent file from input data
pub struct TorrentCreator {
    piece_length: u64,
    announce: Vec<String>,
    created_by: String,
    comment: Option<String>,
}

impl TorrentCreator {
    pub fn new() -> Self {
        Self {
            piece_length: 16384, // 16 KB default
            announce: Vec::new(),
            created_by: format!("WebTorrent/{}", env!("CARGO_PKG_VERSION")),
            comment: None,
        }
    }

    pub fn with_piece_length(mut self, length: u64) -> Self {
        self.piece_length = length;
        self
    }

    pub fn with_announce(mut self, announce: Vec<String>) -> Self {
        self.announce = announce;
        self
    }

    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    /// Create a torrent from file data
    pub async fn create_from_data(
        &self,
        name: String,
        data: Bytes,
    ) -> Result<(Bytes, [u8; 20])> {
        // Calculate pieces
        let num_pieces = ((data.len() as u64 + self.piece_length - 1) / self.piece_length) as usize;
        let mut piece_hashes = Vec::new();

        for i in 0..num_pieces {
            let start = (i as u64 * self.piece_length) as usize;
            let end = ((start as u64 + self.piece_length) as usize).min(data.len());
            let piece_data = &data[start..end];

            let mut hasher = Sha1::new();
            hasher.update(piece_data);
            let hash: [u8; 20] = hasher.finalize().into();
            piece_hashes.extend_from_slice(&hash);
        }

        // Build info dictionary
        let mut info_dict = HashMap::new();
        info_dict.insert(
            Bytes::from("name"),
            BencodeValue::Bytes(Bytes::from(name.clone())),
        );
        info_dict.insert(
            Bytes::from("piece length"),
            BencodeValue::Integer(self.piece_length as i64),
        );
        info_dict.insert(
            Bytes::from("pieces"),
            BencodeValue::Bytes(Bytes::from(piece_hashes)),
        );
        info_dict.insert(
            Bytes::from("length"),
            BencodeValue::Integer(data.len() as i64),
        );

        let info = BencodeValue::Dict(info_dict);
        let info_bytes = info.encode();

        // Calculate info hash
        let mut hasher = Sha1::new();
        hasher.update(&info_bytes);
        let info_hash: [u8; 20] = hasher.finalize().into();

        // Build torrent dictionary
        let mut torrent_dict = HashMap::new();

        if !self.announce.is_empty() {
            torrent_dict.insert(
                Bytes::from("announce"),
                BencodeValue::Bytes(Bytes::from(self.announce[0].clone())),
            );
        }

        if self.announce.len() > 1 {
            let announce_list: Vec<BencodeValue> = self.announce[1..]
                .iter()
                .map(|url| {
                    BencodeValue::List(vec![BencodeValue::Bytes(Bytes::from(url.clone()))])
                })
                .collect();
            torrent_dict.insert(
                Bytes::from("announce-list"),
                BencodeValue::List(announce_list),
            );
        }

        if let Some(ref comment) = self.comment {
            torrent_dict.insert(
                Bytes::from("comment"),
                BencodeValue::Bytes(Bytes::from(comment.clone())),
            );
        }

        torrent_dict.insert(
            Bytes::from("created by"),
            BencodeValue::Bytes(Bytes::from(self.created_by.clone())),
        );

        torrent_dict.insert(Bytes::from("info"), info);

        let torrent = BencodeValue::Dict(torrent_dict);
        let torrent_bytes = torrent.encode();

        Ok((torrent_bytes, info_hash))
    }
}

impl Default for TorrentCreator {
    fn default() -> Self {
        Self::new()
    }
}

