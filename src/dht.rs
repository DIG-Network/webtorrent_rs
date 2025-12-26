use crate::error::{Result, WebTorrentError};
use libp2p::{
    kad::{Behaviour as KadBehaviour, QueryId, RecordKey, store::MemoryStore},
    PeerId, Transport,
};
use libp2p::kad::Record;
use libp2p::identity::Keypair;
use libp2p::core::upgrade;
use libp2p::swarm::SwarmEvent;
use libp2p::noise;
use libp2p::yamux;
use libp2p::tcp;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};

/// Commands to send to the DHT Swarm task
#[derive(Debug)]
enum DhtCommand {
    Announce {
        response_tx: mpsc::UnboundedSender<DhtResponse>,
    },
    Lookup {
        response_tx: mpsc::UnboundedSender<DhtResponse>,
    },
    Shutdown,
}

/// Responses from the DHT Swarm task
#[derive(Debug)]
enum DhtResponse {
    AnnounceComplete,
    LookupComplete(Vec<(String, u16)>),
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
    command_tx: Arc<RwLock<Option<mpsc::UnboundedSender<DhtCommand>>>>,
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
        Self {
            info_hash,
            peer_id,
            port,
            peers: Arc::new(RwLock::new(HashMap::new())),
            discovered_peers: Arc::new(RwLock::new(Vec::new())),
            destroyed: Arc::new(RwLock::new(false)),
            event_tx: Some(tx),
            event_rx: Arc::new(RwLock::new(Some(rx))),
            command_tx: Arc::new(RwLock::new(None)),
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
        let query_id_to_response_clone = Arc::new(RwLock::new(HashMap::<QueryId, mpsc::UnboundedSender<DhtResponse>>::new()));
        let query_id_to_response_clone_for_swarm = Arc::clone(&query_id_to_response_clone);
        
        // Store command sender before spawning task
        *self.command_tx.write().await = Some(command_tx.clone());
        
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
                query_id_to_response_clone_for_swarm,
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
        port: u16,
        info_hash: [u8; 20],
        _peer_id_bytes: [u8; 20],
        mut command_rx: mpsc::UnboundedReceiver<DhtCommand>,
        destroyed: Arc<RwLock<bool>>,
        event_tx: Option<mpsc::UnboundedSender<DhtEvent>>,
        discovered_peers: Arc<RwLock<Vec<(String, u16)>>>,
        query_id_to_info_hash: Arc<RwLock<HashMap<QueryId, [u8; 20]>>>,
        query_id_to_response: Arc<RwLock<HashMap<QueryId, mpsc::UnboundedSender<DhtResponse>>>>,
    ) {
        // Create a keypair for transport encryption
        let keypair = Keypair::generate_ed25519();
        
        // Create a Kademlia DHT with in-memory record store
        let store = MemoryStore::new(peer_id);
        let mut kademlia = KadBehaviour::new(peer_id, store);
        
        // Set Kademlia mode to server (so we can store records)
        kademlia.set_mode(Some(libp2p::kad::Mode::Server));
        
        // Create transport: TCP with Noise encryption and Yamux multiplexing
        // libp2p 0.56 API: Use tcp::Transport (tokio feature provides async support)
        // Create transport - libp2p 0.56 API (explicitly use Tokio provider)
        let transport = tcp::tokio::Transport::default()
         .upgrade(upgrade::Version::V1)
         .authenticate(noise::Config::new(&keypair).unwrap())
         .multiplex(yamux::Config::default())
         .boxed();
        
        // Create Swarm - libp2p 0.56.0 API
        // Swarm::new() requires transport, behaviour, peer_id, and config
        // Reference: https://docs.rs/libp2p/0.56.0/libp2p/swarm/struct.Swarm.html
        use libp2p::swarm::{Swarm, Config};
        use futures::executor::ThreadPool;
        let thread_pool = ThreadPool::new().unwrap();
        let config = Config::with_executor(thread_pool);
        let mut swarm = Swarm::new(transport, kademlia, peer_id, config);
        
        // Bootstrap nodes for DHT (BitTorrent DHT bootstrap nodes)
        let bootstrap_nodes = vec![
            "/ip4/67.215.246.10/tcp/6881",
            "/ip4/87.98.162.88/tcp/6881",
            "/ip4/82.221.103.244/tcp/6881",
        ];
        
        for addr_str in bootstrap_nodes {
            if let Ok(addr) = addr_str.parse::<libp2p::core::Multiaddr>() {
                if let Err(e) = swarm.dial(addr) {
                    debug!("Failed to dial bootstrap node {}: {}", addr_str, e);
                }
            }
        }
        
        // Listen on the specified port (or random if 0)
        let listen_addr = if port > 0 {
            format!("/ip4/0.0.0.0/tcp/{}", port)
        } else {
            "/ip4/0.0.0.0/tcp/0".to_string()
        };
        
        if let Ok(addr) = listen_addr.parse() {
            if let Err(e) = swarm.listen_on(addr) {
                warn!("Failed to listen on {}: {}", listen_addr, e);
            } else {
                info!("DHT listening on {}", listen_addr);
            }
        }
        
        // Main loop: handle commands and Swarm events
        loop {
            // Check if destroyed
            if *destroyed.read().await {
                break;
            }
            
            // Handle commands and Swarm events
            tokio::select! {
                Some(command) = command_rx.recv() => {
                    match command {
                        DhtCommand::Announce { response_tx } => {
                            // Implement announce using Swarm
                            let key = info_hash_to_key(&info_hash);
                            
                            // Create a record with peer information
                            // For BitTorrent DHT, we store peer addresses for the info hash
                            // Get our actual listening address if available
                            let peer_data = format!("{}:{}", "0.0.0.0", port);
                            let record = Record::new(key, peer_data.as_bytes().to_vec());
                            
                            // Put the record in the DHT
                            match swarm.behaviour_mut().put_record(record, libp2p::kad::Quorum::One) {
                                Ok(query_id) => {
                                    debug!("DHT announce started with query ID: {:?}", query_id);
                                    // Store query ID to track this announce and response channel
                                    {
                                        let mut query_map = query_id_to_info_hash.write().await;
                                        query_map.insert(query_id, info_hash);
                                    }
                                    {
                                        let mut response_map = query_id_to_response.write().await;
                                        response_map.insert(query_id, response_tx);
                                    }
                                    // Response will be sent when the query completes via Swarm events
                                }
                                Err(e) => {
                                    warn!("Failed to start DHT announce: {:?}", e);
                                    let _ = response_tx.send(DhtResponse::Error(format!("Failed to start announce: {:?}", e)));
                                }
                            }
                        }
                        DhtCommand::Lookup { response_tx } => {
                            // Implement lookup using Swarm
                            let key = info_hash_to_key(&info_hash);
                            
                            // Get records from the DHT
                            // In libp2p 0.56, get_record returns QueryId directly, not a Result
                            let query_id = swarm.behaviour_mut().get_record(key);
                            debug!("DHT lookup started with query ID: {:?}", query_id);
                            // Store query ID to track this lookup and response channel
                            {
                                let mut query_map = query_id_to_info_hash.write().await;
                                query_map.insert(query_id, info_hash);
                            }
                            {
                                let mut response_map = query_id_to_response.write().await;
                                response_map.insert(query_id, response_tx);
                            }
                            // Response will be sent when the query completes via Swarm events
                        }
                        DhtCommand::Shutdown => {
                            break;
                        }
                    }
                }
                event = swarm.select_next_some() => {
                    // Poll Swarm events
                    match event {
                        SwarmEvent::Behaviour(libp2p::kad::Event::OutboundQueryProgressed { id, result, .. }) => {
                            // Check if we have a response channel for this query
                            let response_tx = {
                                let mut response_map = query_id_to_response.write().await;
                                response_map.remove(&id)
                            };
                            
                            match result {
                                libp2p::kad::QueryResult::GetRecord(Ok(libp2p::kad::GetRecordOk::FoundRecord(record))) => {
                                    // Found a record - parse it and add to discovered peers
                                    if let Some((ip, port)) = parse_dht_record(&record.record.value) {
                                        let mut peers = discovered_peers.write().await;
                                        if !peers.contains(&(ip.clone(), port)) {
                                            peers.push((ip.clone(), port));
                                            debug!("DHT discovered peer: {}:{}", ip, port);
                                            
                                            // Send event
                                            if let Some(ref tx) = event_tx {
                                                let _ = tx.send(DhtEvent::PeerDiscovered(ip.clone(), port));
                                            }
                                            
                                            // Send response if this was a lookup query
                                            if let Some(tx) = response_tx {
                                                let current_peers = peers.clone();
                                                let _ = tx.send(DhtResponse::LookupComplete(current_peers));
                                            }
                                        }
                                    }
                                }
                                libp2p::kad::QueryResult::PutRecord(Ok(_)) => {
                                    // Record stored successfully
                                    debug!("DHT record stored successfully");
                                    if let Some(ref tx) = event_tx {
                                        let _ = tx.send(DhtEvent::AnnounceComplete);
                                    }
                                    
                                    // Send response if this was an announce query
                                    if let Some(tx) = response_tx {
                                        let _ = tx.send(DhtResponse::AnnounceComplete);
                                    }
                                }
                                libp2p::kad::QueryResult::GetRecord(Ok(libp2p::kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. })) => {
                                    // Lookup finished but no more records
                                    debug!("DHT lookup finished");
                                    
                                    // Send response with current peers
                                    if let Some(tx) = response_tx {
                                        let peers = discovered_peers.read().await.clone();
                                        let _ = tx.send(DhtResponse::LookupComplete(peers));
                                    }
                                }
                                libp2p::kad::QueryResult::GetRecord(Err(e)) => {
                                    warn!("DHT get record query error: {:?}", e);
                                    
                                    // Send error response
                                    if let Some(tx) = response_tx {
                                        let _ = tx.send(DhtResponse::Error(format!("Get record query failed: {:?}", e)));
                                    }
                                }
                                libp2p::kad::QueryResult::PutRecord(Err(e)) => {
                                    warn!("DHT query error: {:?}", e);
                                    
                                    // Send error response
                                    if let Some(tx) = response_tx {
                                        let _ = tx.send(DhtResponse::Error(format!("Query failed: {:?}", e)));
                                    }
                                }
                                _ => {
                                    debug!("DHT query result: {:?}", result);
                                    
                                    // Send response for other query types
                                    if let Some(tx) = response_tx {
                                        let peers = discovered_peers.read().await.clone();
                                        let _ = tx.send(DhtResponse::LookupComplete(peers));
                                    }
                                }
                            }
                            
                            // Clean up query tracking
                            {
                                let mut query_map = query_id_to_info_hash.write().await;
                                query_map.remove(&id);
                            }
                        }
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("DHT listening on new address: {}", address);
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            debug!("DHT connection established with peer: {}", peer_id);
                        }
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            debug!("DHT connection closed with peer: {}", peer_id);
                        }
                        _ => {
                            // Handle other Swarm events as needed
                        }
                    }
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
            command_tx.send(DhtCommand::Announce { response_tx }).map_err(|_| {
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
            command_tx.send(DhtCommand::Lookup { response_tx }).map_err(|_| {
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
                let _ = command_tx.send(DhtCommand::Shutdown);
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
