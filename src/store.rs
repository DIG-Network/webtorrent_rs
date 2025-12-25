use crate::error::Result;
use bytes::Bytes;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Chunk store trait for storing torrent pieces
#[async_trait]
pub trait ChunkStore: Send + Sync {
    /// Get a piece from the store
    async fn get(&self, index: usize, offset: usize, length: usize) -> Result<Bytes>;

    /// Put a piece into the store
    async fn put(&self, index: usize, data: Bytes) -> Result<()>;

    /// Close the store
    async fn close(&self) -> Result<()>;

    /// Destroy the store (delete all data)
    async fn destroy(&self) -> Result<()>;
}

/// Memory-based chunk store
pub struct MemoryChunkStore {
    pieces: Arc<RwLock<HashMap<usize, Bytes>>>,
}

impl MemoryChunkStore {
    pub fn new() -> Self {
        Self {
            pieces: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ChunkStore for MemoryChunkStore {
    async fn get(&self, index: usize, offset: usize, length: usize) -> Result<Bytes> {
        let pieces = self.pieces.read().await;
        if let Some(piece) = pieces.get(&index) {
            let end = (offset + length).min(piece.len());
            Ok(piece.slice(offset..end))
        } else {
            Err(crate::error::WebTorrentError::Store(
                format!("Piece {} not found", index),
            ))
        }
    }

    async fn put(&self, index: usize, data: Bytes) -> Result<()> {
        let mut pieces = self.pieces.write().await;
        pieces.insert(index, data);
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }

    async fn destroy(&self) -> Result<()> {
        let mut pieces = self.pieces.write().await;
        pieces.clear();
        Ok(())
    }
}

