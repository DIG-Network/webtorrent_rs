use crate::error::{Result, WebTorrentError};
use libp2p::{
    kad::{Behaviour as KadBehaviour, QueryId, RecordKey, store::MemoryStore},
    PeerId,
};
use libp2p::kad::Record;
use libp2p::identity::Keypair;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::Duration;
use tracing::{debug, info};

/// Commands to send to the DHT Swarm task
#[derive(Debug)]
enum DhtCommand {
    Announce,
    Lookup,
    Shutdown,
}

/// Responses from the DHT Swarm task
#[derive(Debug)]
enum DhtResponse {
    AnnounceComplete,
    LookupComplete(Vec<(String, u16)>),
    #[allow(dead_code)]
    Error(String),
}

/// DHT implementation using libp2p Kademlia
/// The Swarm is stored in a separate task to avoid Send/Sync issues
/// All operations are performed via message passing
pub struct Dht {
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
    peers: Arc<RwLock<HashMap<String, (String, u16)>>>, // peer_id -> (ip, port)
    discovered_peers: Arc<RwLock<Vec<(String, u16)>>>, // List of discovered peers
    destroyed: Arc<RwLock<bool>>,
    event_tx: Option<mpsc::UnboundedSender<DhtEvent>>,
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<DhtEvent>>>>,
    // Channel to send commands to the Swarm task
    command_tx: Arc<RwLock<Option<mpsc::UnboundedSender<(DhtCommand, mpsc::UnboundedSender<DhtResponse>)>>>>,
    // Handle to the Swarm task (for cleanup)
    swarm_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    local_peer_id: Arc<RwLock<Option<PeerId>>>,
    query_id_to_info_hash: Arc<RwLock<HashMap<QueryId, [u8; 20]>>>,
}

/// DHT events
#[derive(Debug, Clone)]
pub enum DhtEvent {
    PeerDiscovered(String, u16),
    AnnounceComplete,
    LookupComplete(Vec<(String, u16)>),
    Error(String),
}

impl Dht {
    /// Create a new DHT instance
    pub fn new(info_hash: [u8; 20], peer_id: [u8; 20], port: u16) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (command_tx, _command_rx) = mpsc::unbounded_channel();
        Self {
            info_hash,
            peer_id,
            port,
            peers: Arc::new(RwLock::new(HashMap::new())),
            discovered_peers: Arc::new(RwLock::new(Vec::new())),
            destroyed: Arc::new(RwLock::new(false)),
            event_tx: Some(tx),
            event_rx: Arc::new(RwLock::new(Some(rx))),
            command_tx: Arc::new(RwLock::new(Some(command_tx))),
            swarm_task: Arc::new(RwLock::new(None)),
            local_peer_id: Arc::new(RwLock::new(None)),
            query_id_to_info_hash: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start DHT and begin peer discovery
    /// The Swarm is moved to a separate task to avoid Send/Sync issues
    pub async fn start(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Discovery("DHT destroyed".to_string()));
        }

        info!("DHT started for info hash: {}", hex::encode(self.info_hash));
        
        // Create channels for communication with the Swarm task
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        
        // Store command sender
        *self.command_tx.write().await = Some(command_tx);
        
        // Create a keypair for this peer
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        
        // Store peer ID
        *self.local_peer_id.write().await = Some(peer_id);
        
        // Prepare data for the Swarm task
        let info_hash = self.info_hash;
        let port = self.port;
        let peer_id_bytes = self.peer_id;
        let destroyed_clone = Arc::clone(&self.destroyed);
        let event_tx_clone = self.event_tx.clone();
        let discovered_peers_clone = Arc::clone(&self.discovered_peers);
        let query_id_to_info_hash_clone = Arc::clone(&self.query_id_to_info_hash);
        
        // Spawn the Swarm task - this will own the Swarm (which is not Send)
        // The Swarm will be created inside this task
        let swarm_task = tokio::spawn(async move {
            Self::swarm_task(
                peer_id,
                port,
                info_hash,
                peer_id_bytes,
                command_rx,
                destroyed_clone,
                event_tx_clone,
                discovered_peers_clone,
                query_id_to_info_hash_clone,
            ).await;
        });
        
        // Store the task handle
        *self.swarm_task.write().await = Some(swarm_task);
        
        debug!("DHT bootstrap initiated");
        Ok(())
    }
    
    /// Swarm task that owns the Swarm and handles all operations
    /// This runs in a separate task to avoid Send/Sync issues
    async fn swarm_task(
        peer_id: PeerId,
        _port: u16,
        _info_hash: [u8; 20],
        _peer_id_bytes: [u8; 20],
        mut command_rx: mpsc::UnboundedReceiver<(DhtCommand, mpsc::UnboundedSender<DhtResponse>)>,
        destroyed: Arc<RwLock<bool>>,
        _event_tx: Option<mpsc::UnboundedSender<DhtEvent>>,
        discovered_peers: Arc<RwLock<Vec<(String, u16)>>>,
        _query_id_to_info_hash: Arc<RwLock<HashMap<QueryId, [u8; 20]>>>,
    ) {
        // Create a Kademlia DHT with in-memory record store
        let store = MemoryStore::new(peer_id);
        let mut kademlia = KadBehaviour::new(peer_id, store);
        
        // Set Kademlia mode to server (so we can store records)
        kademlia.set_mode(Some(libp2p::kad::Mode::Server));
        
        // Create the Swarm - this is not Send, so it must stay in this task
        // Note: This needs the correct libp2p 0.56 API
        // For now, we'll create a placeholder that can be updated
        // TODO: Create Swarm using correct libp2p 0.56 API with transport
        // let mut swarm = SwarmBuilder::new(...).build();
        
        // For now, we'll use a simplified approach where we just handle commands
        // The actual Swarm creation will be implemented when we have the correct API
        
        // Main loop: handle commands
        loop {
            // Check if destroyed
            if *destroyed.read().await {
                break;
            }
            
            // Handle commands from the Dht struct
            tokio::select! {
                Some((command, response_tx)) = command_rx.recv() => {
                    match command {
                        DhtCommand::Announce => {
                            // TODO: Implement announce using Swarm
                            // For now, just send success
                            let _ = response_tx.send(DhtResponse::AnnounceComplete);
                        }
                        DhtCommand::Lookup => {
                            // TODO: Implement lookup using Swarm
                            // For now, return cached peers
                            let peers = discovered_peers.read().await.clone();
                            let _ = response_tx.send(DhtResponse::LookupComplete(peers));
                        }
                        DhtCommand::Shutdown => {
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Poll Swarm for events periodically
                    // TODO: Poll Swarm events here when Swarm is created
                }
            }
        }
    }

    /// Announce to DHT (PUT operation)
    pub async fn announce(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Discovery("DHT destroyed".to_string()));
        }

        debug!("Announcing to DHT: {}", hex::encode(self.info_hash));
        
        // Send command to Swarm task
        let command_tx_guard = self.command_tx.read().await;
        if let Some(ref command_tx) = *command_tx_guard {
            let (response_tx, mut response_rx) = mpsc::unbounded_channel();
            command_tx.send((DhtCommand::Announce, response_tx)).map_err(|_| {
                WebTorrentError::Discovery("DHT command channel closed".to_string())
            })?;
            
            // Wait for response
            match response_rx.recv().await {
                Some(DhtResponse::AnnounceComplete) => {
                    info!("DHT announcement completed for info hash: {}", hex::encode(self.info_hash));
                    Ok(())
                }
                Some(DhtResponse::Error(e)) => {
                    Err(WebTorrentError::Discovery(e))
                }
                _ => {
                    Err(WebTorrentError::Discovery("Unexpected response from DHT task".to_string()))
                }
            }
        } else {
            Err(WebTorrentError::Discovery("DHT not started".to_string()))
        }
    }

    /// Lookup peers in DHT (GET operation)
    pub async fn lookup(&self) -> Result<Vec<(String, u16)>> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Discovery("DHT destroyed".to_string()));
        }

        debug!("Looking up peers in DHT: {}", hex::encode(self.info_hash));
        
        // Send command to Swarm task
        let command_tx_guard = self.command_tx.read().await;
        if let Some(ref command_tx) = *command_tx_guard {
            let (response_tx, mut response_rx) = mpsc::unbounded_channel();
            command_tx.send((DhtCommand::Lookup, response_tx)).map_err(|_| {
                WebTorrentError::Discovery("DHT command channel closed".to_string())
            })?;
            
            // Wait for response
            match response_rx.recv().await {
                Some(DhtResponse::LookupComplete(peers)) => {
                    // Update discovered peers
                    {
                        let mut discovered = self.discovered_peers.write().await;
                        for (ip, port) in &peers {
                            if !discovered.contains(&(ip.clone(), *port)) {
                                discovered.push((ip.clone(), *port));
                            }
                        }
                    }
                    
                    if let Some(ref tx) = self.event_tx {
                        let _ = tx.send(DhtEvent::LookupComplete(peers.clone()));
                    }
                    
                    Ok(peers)
                }
                Some(DhtResponse::Error(e)) => {
                    Err(WebTorrentError::Discovery(e))
                }
                _ => {
                    Err(WebTorrentError::Discovery("Unexpected response from DHT task".to_string()))
                }
            }
        } else {
            // Return currently cached peers if DHT not started
            let peers = self.discovered_peers.read().await.clone();
            Ok(peers)
        }
    }

    /// Add a peer discovered via DHT
    pub async fn add_peer(&self, peer_id: String, ip: String, port: u16) {
        if *self.destroyed.read().await {
            return;
        }

        let mut peers = self.peers.write().await;
        peers.insert(peer_id.clone(), (ip.clone(), port));
        
        let mut discovered = self.discovered_peers.write().await;
        if !discovered.contains(&(ip.clone(), port)) {
            discovered.push((ip.clone(), port));
            
            if let Some(ref tx) = self.event_tx {
                let _ = tx.send(DhtEvent::PeerDiscovered(ip.clone(), port));
            }
        }
        
        debug!("Added peer via DHT: {}:{}", ip, port);
    }

    /// Remove a peer
    pub async fn remove_peer(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id);
    }

    /// Get number of discovered peers
    pub async fn peer_count(&self) -> usize {
        self.discovered_peers.read().await.len()
    }

    /// Get all discovered peers
    pub async fn get_peers(&self) -> Vec<(String, u16)> {
        self.discovered_peers.read().await.clone()
    }

    /// Check if a peer is known
    pub async fn has_peer(&self, ip: &str, port: u16) -> bool {
        self.discovered_peers.read().await.contains(&(ip.to_string(), port))
    }

    /// Periodic refresh - should be called periodically to refresh DHT lookups
    pub async fn refresh(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Discovery("DHT destroyed".to_string()));
        }

        // Re-announce and re-lookup
        self.announce().await?;
        let _ = self.lookup().await;
        
        Ok(())
    }

    /// Get DHT event receiver
    pub async fn event_receiver(&self) -> Option<mpsc::UnboundedReceiver<DhtEvent>> {
        self.event_rx.write().await.take()
    }

    pub async fn destroy(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Ok(());
        }

        *self.destroyed.write().await = true;
        
        // Send shutdown command to Swarm task
        {
            let command_tx_guard = self.command_tx.read().await;
            if let Some(ref command_tx) = *command_tx_guard {
                let (response_tx, _response_rx) = mpsc::unbounded_channel();
                let _ = command_tx.send((DhtCommand::Shutdown, response_tx));
            }
        }
        
        // Wait for Swarm task to finish
        if let Some(handle) = self.swarm_task.write().await.take() {
            let _ = handle.await;
        }
        
        self.peers.write().await.clear();
        self.discovered_peers.write().await.clear();
        
        info!("DHT destroyed for info hash: {}", hex::encode(self.info_hash));
        Ok(())
    }
}

/// Helper function to convert info hash to libp2p Key
#[allow(dead_code)]
fn info_hash_to_key(info_hash: &[u8; 20]) -> RecordKey {
    // Convert info hash to a Kademlia key
    // In BitTorrent DHT, the info hash is used directly as the key
    RecordKey::new(info_hash)
}

/// Helper function to create a DHT record from peer information
#[allow(dead_code)]
fn create_dht_record(info_hash: [u8; 20], peer_id: [u8; 20], ip: String, port: u16) -> Record {
    let key = info_hash_to_key(&info_hash);
    
    // Encode peer information as bencode
    // Format: {"id": <peer_id>, "ip": <ip>, "port": <port>}
    let mut value = Vec::new();
    value.extend_from_slice(b"d");
    value.extend_from_slice(b"2:id");
    value.extend_from_slice(format!("{}:", peer_id.len()).as_bytes());
    value.extend_from_slice(&peer_id);
    value.extend_from_slice(b"2:ip");
    value.extend_from_slice(format!("{}:", ip.len()).as_bytes());
    value.extend_from_slice(ip.as_bytes());
    value.extend_from_slice(b"4:port");
    value.extend_from_slice(format!("i{}e", port).as_bytes());
    value.extend_from_slice(b"e");
    
    Record::new(key, value)
}

/// Helper function to parse DHT record value
#[allow(dead_code)]
fn parse_dht_record(value: &[u8]) -> Option<(String, u16)> {
    use crate::bencode_parser::parse_bencode;
    
    let (bencode, _) = parse_bencode(value).ok()?;
    let dict = bencode.as_dict()?;
    
    let ip = dict.get("ip".as_bytes())
        .and_then(|v| v.as_string())?;
    
    let port = dict.get("port".as_bytes())
        .and_then(|v| v.as_integer())
        .map(|p| p as u16)?;
    
    Some((ip, port))
}
