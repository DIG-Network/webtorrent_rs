use crate::error::Result;
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const BLOCK_LENGTH: usize = 16 * 1024; // 16 KB

/// Represents a piece of a torrent
pub struct Piece {
    length: u64,
    blocks: Arc<RwLock<Vec<Option<Bytes>>>>,
    missing: Arc<RwLock<usize>>,
}

impl Piece {
    pub fn new(length: u64) -> Self {
        let num_blocks = ((length as usize + BLOCK_LENGTH - 1) / BLOCK_LENGTH) as usize;
        let blocks = vec![None; num_blocks];
        let missing = num_blocks;

        Self {
            length,
            blocks: Arc::new(RwLock::new(blocks)),
            missing: Arc::new(RwLock::new(missing)),
        }
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    pub async fn missing(&self) -> usize {
        *self.missing.read().await
    }

    /// Reserve a block for downloading
    pub async fn reserve(&self) -> Option<usize> {
        let mut blocks = self.blocks.write().await;
        for (i, block) in blocks.iter_mut().enumerate() {
            if block.is_none() {
                return Some(i);
            }
        }
        None
    }

    /// Set a block's data
    pub async fn set(&self, block_index: usize, data: Bytes) -> Result<bool> {
        let mut blocks = self.blocks.write().await;
        if block_index >= blocks.len() {
            return Err(crate::error::WebTorrentError::InvalidPieceIndex(block_index));
        }

        if blocks[block_index].is_some() {
            return Ok(false); // Already set
        }

        blocks[block_index] = Some(data);
        let mut missing = self.missing.write().await;
        *missing -= 1;

        Ok(*missing == 0)
    }

    /// Cancel a block reservation
    pub async fn cancel(&self, block_index: usize) {
        let blocks = self.blocks.write().await;
        if block_index < blocks.len() && blocks[block_index].is_none() {
            // Block was reserved but not set, so it's still missing
            // No need to update missing count
        }
    }

    /// Flush all blocks into a single buffer
    pub async fn flush(&self) -> Bytes {
        let blocks = self.blocks.read().await;
        let mut buffer = Vec::with_capacity(self.length as usize);
        for block in blocks.iter() {
            if let Some(ref data) = block {
                buffer.extend_from_slice(data);
            }
        }
        // Truncate to exact length (last piece may be shorter)
        buffer.truncate(self.length as usize);
        buffer.into()
    }

    /// Get chunk offset for a block
    pub fn chunk_offset(&self, block_index: usize) -> u64 {
        (block_index * BLOCK_LENGTH) as u64
    }

    /// Get chunk length for a block
    pub async fn chunk_length(&self, block_index: usize) -> usize {
        let blocks_len = self.blocks.read().await.len();
        if block_index == blocks_len - 1 {
            // Last block may be shorter
            let total_blocks = (self.length as usize + BLOCK_LENGTH - 1) / BLOCK_LENGTH;
            if total_blocks > 0 {
                let last_block_size = self.length as usize - (total_blocks - 1) * BLOCK_LENGTH;
                last_block_size
            } else {
                BLOCK_LENGTH
            }
        } else {
            BLOCK_LENGTH
        }
    }
}

