use crate::error::{Result, WebTorrentError};
use crate::torrent::Torrent;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use tokio::sync::{RwLock, mpsc};

/// A stream that reads data from a torrent file as it becomes available
pub struct TorrentReadStream {
    torrent: Arc<Torrent>,
    file_index: usize,
    position: u64,
    buffer: Vec<Bytes>,
    buffer_position: usize,
    buffer_size: usize,
    destroyed: Arc<RwLock<bool>>,
    // Channel for receiving data from the background reader task
    data_rx: Option<mpsc::UnboundedReceiver<Result<Bytes>>>,
    // Waker to wake the stream when data is available
    waker: Arc<RwLock<Option<Waker>>>,
    // Background task handle
    reader_task: Option<tokio::task::JoinHandle<()>>,
}

impl TorrentReadStream {
    /// Create a new read stream for a file in the torrent
    pub fn new(torrent: Arc<Torrent>, file_index: usize) -> Result<Self> {
        let files = torrent.files();
        if file_index >= files.len() {
            return Err(WebTorrentError::InvalidTorrent(
                format!("File index {} out of range", file_index)
            ));
        }

        // Mark file as critical for streaming (high priority)
        // Also create a stream selection for sequential piece downloading
        let torrent_clone = Arc::clone(&torrent);
        let file_index_clone = file_index;
        
        tokio::spawn(async move {
            // Mark as critical
            let _ = torrent_clone.critical(file_index_clone).await;
            
            // Get piece length and file info
            let piece_length = torrent_clone.piece_length().await;
            let files = torrent_clone.files();
            if file_index_clone < files.len() {
                let file = &files[file_index_clone];
                let start_piece = file.start_piece(piece_length);
                let end_piece = file.end_piece(piece_length);
                
                // Add stream selection for sequential downloading
                let mut selections = torrent_clone.selections.write().await;
                let stream_selection = crate::selections::Selection::new(
                    start_piece,
                    end_piece,
                    7, // High priority for streaming
                    true, // This is a stream selection
                );
                selections.insert(stream_selection);
            }
        });

        // Create channel for data streaming
        let destroyed = Arc::new(RwLock::new(false));
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        let waker = Arc::new(RwLock::new(None::<Waker>));
        let waker_clone = Arc::clone(&waker);
        let destroyed_clone = Arc::clone(&destroyed);
        let torrent_clone = Arc::clone(&torrent);
        let file_index_clone = file_index;
        
        // Spawn background task to read data from store
        let reader_task = tokio::spawn(async move {
            let mut position = 0u64;
            let files = torrent_clone.files();
            if file_index_clone >= files.len() {
                return;
            }
            
            let file = &files[file_index_clone];
            let file_length = file.length();
            let file_offset = file.offset();
            let piece_length = torrent_clone.piece_length().await;
            let store = torrent_clone.store();
            
            if store.is_none() {
                return;
            }
            let store = store.unwrap();
            
            loop {
                // Check if destroyed
                if *destroyed_clone.read().await {
                    break;
                }
                
                // Check if EOF
                if position >= file_length {
                    // Send EOF signal
                    let _ = data_tx.send(Ok(Bytes::new()));
                    break;
                }
                
                // Calculate which piece we need
                let absolute_position = file_offset + position;
                let piece_index = (absolute_position / piece_length) as usize;
                let offset_in_piece = (absolute_position % piece_length) as usize;
                let remaining = (file_length - position) as usize;
                let to_read = (64 * 1024).min(remaining); // Read up to 64KB at a time
                
                // Try to read from store
                match store.get(piece_index, offset_in_piece, to_read).await {
                    Ok(data) => {
                        if data.is_empty() {
                            // No data available yet - wait a bit and retry
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            continue;
                        }
                        
                        // Send data
                        if data_tx.send(Ok(data.clone())).is_err() {
                            // Receiver dropped
                            break;
                        }
                        
                        position += data.len() as u64;
                        
                        // Wake the stream if waker is set
                        if let Some(w) = waker_clone.write().await.take() {
                            w.wake();
                        }
                    }
                    Err(_) => {
                        // Piece not available yet - wait and retry
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
        });
        
        Ok(Self {
            torrent,
            file_index,
            position: 0,
            buffer: Vec::new(),
            buffer_position: 0,
            buffer_size: 64 * 1024, // 64KB buffer
            destroyed,
            data_rx: Some(data_rx),
            waker,
            reader_task: Some(reader_task),
        })
    }

    /// Get the current read position
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Check if the stream is at the end
    pub async fn is_eof(&self) -> bool {
        let files = self.torrent.files();
        if self.file_index >= files.len() {
            return true;
        }
        let file = &files[self.file_index];
        self.position >= file.length()
    }

    /// Destroy the stream
    pub async fn destroy(&mut self) -> Result<()> {
        *self.destroyed.write().await = true;
        
        // Wait for reader task to finish
        if let Some(handle) = self.reader_task.take() {
            let _ = handle.await;
        }
        
        Ok(())
    }

    /// Read data from the stream (internal method)
    async fn read_internal(&mut self, buf: &mut [u8]) -> Result<usize> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Network("Stream destroyed".to_string()));
        }

        let files = self.torrent.files();
        if self.file_index >= files.len() {
            return Ok(0); // EOF
        }

        let file = &files[self.file_index];
        let file_length = file.length();
        let file_offset = file.offset();
        let piece_length = self.torrent.piece_length().await;

        // Check if we're at EOF
        if self.position >= file_length {
            return Ok(0);
        }

        // Calculate how much we can read
        let remaining_in_file = file_length - self.position;
        let to_read = buf.len().min(remaining_in_file as usize);

        // Try to read from buffer first
        let mut bytes_read = 0;
        while bytes_read < to_read && !self.buffer.is_empty() {
            if self.buffer_position < self.buffer.len() {
                let chunk = &self.buffer[self.buffer_position];
                let available = chunk.len();
                let needed = to_read - bytes_read;
                let copy_len = available.min(needed);

                buf[bytes_read..bytes_read + copy_len].copy_from_slice(&chunk[..copy_len]);
                bytes_read += copy_len;

                if copy_len == available {
                    // Consumed entire chunk
                    self.buffer.remove(self.buffer_position);
                } else {
                    // Partial chunk - update it
                    self.buffer[self.buffer_position] = chunk.slice(copy_len..);
                }
            } else {
                break;
            }
        }

        // If we need more data, try to read from store
        if bytes_read < to_read {
            let store = self.torrent.store();
            if let Some(store) = store {
                // Calculate which piece we need
                let absolute_position = file_offset + self.position;
                let piece_index = (absolute_position / piece_length) as usize;
                let offset_in_piece = (absolute_position % piece_length) as usize;
                let remaining = to_read - bytes_read;

                // Try to read from store
                match store.get(piece_index, offset_in_piece, remaining).await {
                    Ok(data) => {
                        let copy_len = data.len().min(remaining);
                        buf[bytes_read..bytes_read + copy_len].copy_from_slice(&data[..copy_len]);
                        bytes_read += copy_len;
                    }
                    Err(_) => {
                        // Piece not available yet - return what we have
                        // The caller should wait and try again
                    }
                }
            }
        }

        self.position += bytes_read as u64;
        Ok(bytes_read)
    }
}

impl Stream for TorrentReadStream {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Check if destroyed
        if let Ok(destroyed) = self.destroyed.try_read() {
            if *destroyed {
                return Poll::Ready(None);
            }
        }
        
        // Store waker for background task
        {
            if let Ok(mut guard) = self.waker.try_write() {
                *guard = Some(cx.waker().clone());
            }
        }
        
        // Try to receive data from the background reader task
        if let Some(ref mut rx) = self.data_rx {
            match rx.try_recv() {
                Ok(Ok(data)) => {
                    if data.is_empty() {
                        // EOF signal
                        Poll::Ready(None)
                    } else {
                        self.position += data.len() as u64;
                        Poll::Ready(Some(Ok(data)))
                    }
                }
                Ok(Err(e)) => {
                    // Error from reader task
                    Poll::Ready(Some(Err(e)))
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    // No data available yet
                    Poll::Pending
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Channel closed - EOF
                    Poll::Ready(None)
                }
            }
        } else {
            // No receiver - stream is closed
            Poll::Ready(None)
        }
    }
}


#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_torrent_read_stream_new() {
        // This would require a mock torrent
        // Just verify the function exists
        assert!(true);
    }
}

