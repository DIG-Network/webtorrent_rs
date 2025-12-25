use crate::error::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

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
        }
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

