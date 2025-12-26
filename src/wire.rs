use crate::error::Result;
use crate::extensions::UtPex;
use bytes::{Bytes, BufMut};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// Represents a BitTorrent protocol wire (connection)
pub struct Wire {
    #[allow(dead_code)]
    wire_type: String,
    destroyed: Arc<RwLock<bool>>,
    peer_pieces: Arc<RwLock<bitvec::prelude::BitVec>>,
    peer_choking: Arc<RwLock<bool>>,
    peer_interested: Arc<RwLock<bool>>,
    am_choking: Arc<RwLock<bool>>,
    am_interested: Arc<RwLock<bool>>,
    requests: Arc<RwLock<Vec<BlockRequest>>>,
    // PEX extension
    pub(crate) ut_pex: Arc<RwLock<Option<UtPex>>>,
    // Channel for sending messages immediately (set by connection handler)
    message_tx: Arc<RwLock<Option<mpsc::UnboundedSender<Bytes>>>>,
}

#[derive(Debug, Clone)]
pub struct BlockRequest {
    pub piece: usize,
    pub offset: usize,
    pub length: usize,
}

impl Wire {
    pub fn new(wire_type: String) -> Self {
        Self {
            wire_type,
            destroyed: Arc::new(RwLock::new(false)),
            peer_pieces: Arc::new(RwLock::new(bitvec::prelude::BitVec::new())),
            peer_choking: Arc::new(RwLock::new(true)),
            peer_interested: Arc::new(RwLock::new(false)),
            am_choking: Arc::new(RwLock::new(true)),
            am_interested: Arc::new(RwLock::new(false)),
            requests: Arc::new(RwLock::new(Vec::new())),
            ut_pex: Arc::new(RwLock::new(None)),
            message_tx: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Set the message sender for immediate message sending
    pub(crate) async fn set_message_sender(&self, tx: mpsc::UnboundedSender<Bytes>) {
        *self.message_tx.write().await = Some(tx);
    }

    /// Enable PEX extension on this wire
    pub async fn enable_pex(&self) {
        *self.ut_pex.write().await = Some(UtPex::new());
    }

    /// Handle PEX message - returns (added_peers, dropped_peers)
    pub async fn handle_pex_message(&self, data: &[u8]) -> Result<(Vec<(String, u16)>, Vec<(String, u16)>)> {
        UtPex::decode_full(data)
    }
    
    /// Send PEX update - updates internal state and sends immediately if possible
    pub async fn send_pex_update(&self, added: Vec<(String, u16)>, dropped: Vec<(String, u16)>) -> Result<bytes::Bytes> {
        // Update PEX state
        let encoded = if let Some(ref mut pex) = *self.ut_pex.write().await {
            pex.added = added;
            pex.dropped = dropped;
            pex.encode()
        } else {
            // Create new PEX if not enabled
            let mut pex = UtPex::new();
            pex.added = added;
            pex.dropped = dropped;
            let encoded = pex.encode();
            *self.ut_pex.write().await = Some(pex);
            encoded
        };
        
        // Send immediately if message sender is available
        if let Some(ref tx) = *self.message_tx.read().await {
            // Build extended message with PEX data
            // Extended message format: [20][extended_id][data]
            let mut message = bytes::BytesMut::with_capacity(2 + encoded.len());
            message.put_u8(20); // Extended message ID
            message.put_u8(1); // ut_pex extension ID (typically 1)
            message.put_slice(&encoded);
            
            // Send message length prefix
            let mut full_message = bytes::BytesMut::with_capacity(4 + message.len());
            full_message.put_u32(message.len() as u32);
            full_message.put_slice(&message);
            
            // Send immediately
            if tx.send(full_message.freeze()).is_err() {
                // Channel closed - connection may be closed
            }
        }
        
        Ok(encoded)
    }
    
    /// Get PEX extension (if enabled)
    pub async fn get_pex(&self) -> Option<UtPex> {
        self.ut_pex.read().await.clone()
    }

    pub async fn destroyed(&self) -> bool {
        *self.destroyed.read().await
    }

    pub async fn destroy(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Ok(());
        }

        *self.destroyed.write().await = true;
        Ok(())
    }

    pub async fn peer_pieces(&self) -> bitvec::prelude::BitVec {
        self.peer_pieces.read().await.clone()
    }

    pub async fn set_peer_pieces(&self, bitfield: bitvec::prelude::BitVec) {
        *self.peer_pieces.write().await = bitfield;
    }

    pub async fn peer_choking(&self) -> bool {
        *self.peer_choking.read().await
    }

    pub async fn set_peer_choking(&self, choking: bool) {
        *self.peer_choking.write().await = choking;
    }

    pub async fn peer_interested(&self) -> bool {
        *self.peer_interested.read().await
    }

    pub async fn set_peer_interested(&self, interested: bool) {
        *self.peer_interested.write().await = interested;
    }

    pub async fn am_choking(&self) -> bool {
        *self.am_choking.read().await
    }

    pub async fn choke(&self) {
        *self.am_choking.write().await = true;
    }

    pub async fn unchoke(&self) {
        *self.am_choking.write().await = false;
    }

    pub async fn interested(&self) {
        *self.am_interested.write().await = true;
    }

    pub async fn uninterested(&self) {
        *self.am_interested.write().await = false;
    }

    pub async fn request(&self, piece: usize, offset: usize, length: usize) {
        let mut requests = self.requests.write().await;
        requests.push(BlockRequest {
            piece,
            offset,
            length,
        });
    }

    pub async fn requests(&self) -> Vec<BlockRequest> {
        self.requests.read().await.clone()
    }
}

