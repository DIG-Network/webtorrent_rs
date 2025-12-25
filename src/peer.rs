use crate::error::Result;
use crate::wire::Wire;
use std::sync::Arc;
use tokio::sync::RwLock;
// Peer struct - debug not currently used

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerType {
    TcpIncoming,
    TcpOutgoing,
    UtpIncoming,
    UtpOutgoing,
    WebRtc,
    WebSeed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerSource {
    Manual,
    Tracker,
    Dht,
    Lsd,
    UtPex,
}

/// Represents a peer connection
pub struct Peer {
    id: String,
    peer_type: PeerType,
    #[allow(dead_code)]
    source: Option<PeerSource>,
    #[allow(dead_code)]
    addr: Option<String>,
    wire: Option<Arc<Wire>>,
    connected: Arc<RwLock<bool>>,
    destroyed: Arc<RwLock<bool>>,
}

impl Peer {
    pub fn new(id: String, peer_type: PeerType) -> Self {
        Self {
            id,
            peer_type,
            source: None,
            addr: None,
            wire: None,
            connected: Arc::new(RwLock::new(false)),
            destroyed: Arc::new(RwLock::new(false)),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn peer_type(&self) -> PeerType {
        self.peer_type
    }

    pub async fn connected(&self) -> bool {
        *self.connected.read().await
    }

    pub async fn destroy(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Ok(());
        }

        *self.destroyed.write().await = true;
        *self.connected.write().await = false;

        if let Some(wire) = &self.wire {
            wire.destroy().await?;
        }

        Ok(())
    }
}

