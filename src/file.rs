// File struct - no unused imports
use std::sync::Arc;
use tokio::sync::RwLock;

/// Represents a file within a torrent
pub struct File {
    name: String,
    path: String,
    length: u64,
    offset: u64,
    done: Arc<RwLock<bool>>,
}

impl File {
    pub fn new(path: String, length: u64, offset: u64) -> Self {
        let name = path.split('/').last().unwrap_or(&path).to_string();
        Self {
            name,
            path,
            length,
            offset,
            done: Arc::new(RwLock::new(false)),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub async fn done(&self) -> bool {
        *self.done.read().await
    }

    pub async fn set_done(&self, done: bool) {
        *self.done.write().await = done;
    }

    /// Get the start piece index for this file
    pub fn start_piece(&self, piece_length: u64) -> usize {
        (self.offset / piece_length) as usize
    }

    /// Get the end piece index for this file
    pub fn end_piece(&self, piece_length: u64) -> usize {
        ((self.offset + self.length - 1) / piece_length) as usize
    }

    /// Check if this file includes the given piece index
    pub fn includes_piece(&self, piece_index: usize, piece_length: u64) -> bool {
        let start = self.start_piece(piece_length);
        let end = self.end_piece(piece_length);
        piece_index >= start && piece_index <= end
    }
}

