#![allow(dead_code)] // Many struct fields are accessed through methods, not directly

use crate::client::{WebTorrent, TorrentId};
use crate::error::{Result, WebTorrentError};
use crate::peer::Peer;
use crate::piece::Piece;
use crate::file::File;
use crate::discovery::Discovery;
use crate::store::ChunkStore;
use crate::selections::Selections;
use crate::rarity_map::RarityMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
// Tracing imports removed - not currently used
use bytes::Bytes;
use sha1::{Sha1, Digest};
use bitvec::prelude::*;

/// Torrent represents a single torrent in the client
#[allow(dead_code)] // Many fields are accessed through methods, not directly
pub struct Torrent {
    #[allow(dead_code)] // Used in start_discovery() via self.client
    client: WebTorrent,
    #[allow(dead_code)] // Used in info_hash() method, but linter doesn't detect
    info_hash: [u8; 20],
    #[allow(dead_code)]
    info_hash_hash: Option<[u8; 20]>,
    #[allow(dead_code)] // Used in name() method, but linter doesn't detect
    name: String,
    #[allow(dead_code)] // Used in length() and progress() methods, but linter doesn't detect
    length: u64,
    #[allow(dead_code)]
    piece_length: u64,
    #[allow(dead_code)]
    pieces: Vec<Option<Piece>>,
    #[allow(dead_code)]
    piece_hashes: Vec<[u8; 20]>,
    #[allow(dead_code)] // Used in files() method, but linter doesn't detect
    files: Vec<File>,
    #[allow(dead_code)] // Used in get_bitfield() method, but linter doesn't detect
    bitfield: Arc<RwLock<BitVec>>,
    #[allow(dead_code)]
    metadata: Option<Bytes>,
    #[allow(dead_code)]
    magnet_uri: Option<String>,
    #[allow(dead_code)]
    torrent_file: Option<Bytes>,
    #[allow(dead_code)] // Used in start_discovery() method, but linter doesn't detect
    announce: Vec<String>,
    #[allow(dead_code)]
    url_list: Vec<String>,
    #[allow(dead_code)]
    private: bool,
    #[allow(dead_code)] // Used internally
    ready: Arc<RwLock<bool>>,
    #[allow(dead_code)] // Used internally
    destroyed: Arc<RwLock<bool>>,
    #[allow(dead_code)]
    paused: Arc<RwLock<bool>>,
    #[allow(dead_code)]
    done: Arc<RwLock<bool>>,
    #[allow(dead_code)] // Used in downloaded() method, but linter doesn't detect
    downloaded: Arc<RwLock<u64>>,
    #[allow(dead_code)] // Used in uploaded() method, but linter doesn't detect
    uploaded: Arc<RwLock<u64>>,
    #[allow(dead_code)] // Used in received() method, but linter doesn't detect
    received: Arc<RwLock<u64>>,
    #[allow(dead_code)] // Used in start_discovery() and destroy() methods, but linter doesn't detect
    discovery: Arc<RwLock<Option<Arc<Discovery>>>>,
    #[allow(dead_code)] // Used in destroy() method, but linter doesn't detect
    store: Option<Arc<dyn ChunkStore>>,
    pub(crate) peers: Arc<RwLock<HashMap<String, Arc<Peer>>>>,
    pub(crate) wires: Arc<RwLock<Vec<Arc<crate::wire::Wire>>>>,
    #[allow(dead_code)]
    selections: Arc<RwLock<Selections>>,
    #[allow(dead_code)]
    rarity_map: Option<Arc<RarityMap>>,
    #[allow(dead_code)]
    strategy: PieceStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum PieceStrategy {
    Sequential,
    Rarest,
}

impl Torrent {
    pub async fn new(torrent_id: TorrentId, client: WebTorrent) -> Result<Self> {
        // Parse torrent ID
        let (info_hash, metadata, announce, url_list, private_flag, name, length, piece_length, piece_hashes, files) = 
            Self::parse_torrent_id(torrent_id, &client).await?;

        let num_pieces = piece_hashes.len();
        let pieces: Vec<Option<Piece>> = (0..num_pieces)
            .map(|i| {
                let len = if i == num_pieces - 1 {
                    length - (i as u64 * piece_length)
                } else {
                    piece_length
                };
                Some(Piece::new(len))
            })
            .collect();

        let bitfield = Arc::new(RwLock::new(bitvec![0; num_pieces]));

        Ok(Self {
            client,
            info_hash,
            info_hash_hash: None,
            name,
            length,
            piece_length,
            pieces,
            piece_hashes,
            files,
            bitfield,
            metadata,
            magnet_uri: None,
            torrent_file: None,
            announce,
            url_list,
            private: private_flag,
            ready: Arc::new(RwLock::new(false)),
            destroyed: Arc::new(RwLock::new(false)),
            paused: Arc::new(RwLock::new(false)),
            done: Arc::new(RwLock::new(false)),
            downloaded: Arc::new(RwLock::new(0)),
            uploaded: Arc::new(RwLock::new(0)),
            received: Arc::new(RwLock::new(0)),
            discovery: Arc::new(RwLock::new(None)),
            store: None,
            peers: Arc::new(RwLock::new(HashMap::new())),
            wires: Arc::new(RwLock::new(Vec::new())),
            selections: Arc::new(RwLock::new(Selections::new())),
            rarity_map: None,
            strategy: PieceStrategy::Sequential,
        })
    }

    /// Start discovery for this torrent (called when torrent is ready)
    pub async fn start_discovery(&self) -> Result<()> {
        // Check if already started
        if self.discovery.read().await.is_some() {
            return Ok(());
        }

        if self.announce.is_empty() {
            return Ok(()); // No trackers to announce to
        }

        let port = *self.client.torrent_port.read().await;
        let discovery = Arc::new(Discovery::new(
            self.info_hash,
            self.client.peer_id,
            self.announce.clone(),
            port,
            false, // DHT
            false, // LSD  
            false, // PEX
        ));

        discovery.start().await?;
        
        // Clone discovery before storing (we need it for the spawned task)
        let discovery_clone = Arc::clone(&discovery);
        
        // Store discovery
        *self.discovery.write().await = Some(discovery);
        let info_hash = self.info_hash;
        let client_clone = Arc::new(self.client.clone());
        let peers_map = Arc::clone(&self.peers);
        let wires_vec = Arc::clone(&self.wires);
        let destroyed_flag = Arc::clone(&self.destroyed);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                
                // Check if torrent is destroyed
                if *destroyed_flag.read().await {
                    break;
                }
                
                // Lookup peers
                match discovery_clone.lookup_peers().await {
                    Ok(peers) => {
                        if !peers.is_empty() {
                            eprintln!("[DEBUG] Found {} peers, attempting connections", peers.len());
                        }
                        for (ip, port) in peers {
                            // Skip connecting to our own port (but allow connecting to other clients on same machine)
                            let our_port = *client_clone.torrent_port.read().await;
                            if port == our_port {
                                eprintln!("[DEBUG] Skipping connection to our own port {}:{}", ip, port);
                                continue;
                            }
                            
                            // Connect to peer using helper function
                            // Duplicate connections are handled in connect_to_peer_helper
                            let ip_clone = ip.clone();
                            eprintln!("[DEBUG] Attempting to connect to peer {}:{}", ip_clone, port);
                            if let Err(e) = Self::connect_to_peer_helper(
                                info_hash,
                                client_clone.clone(),
                                peers_map.clone(),
                                wires_vec.clone(),
                                ip,
                                port,
                            ).await {
                                eprintln!("[DEBUG] Failed to connect to {}:{}: {}", ip_clone, port, e);
                            } else {
                                eprintln!("[DEBUG] Successfully connected to peer {}:{}", ip_clone, port);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[DEBUG] Failed to lookup peers: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Connect to a peer (outbound connection) - helper function
    async fn connect_to_peer_helper(
        info_hash: [u8; 20],
        client: Arc<WebTorrent>,
        peers: Arc<RwLock<HashMap<String, Arc<Peer>>>>,
        wires: Arc<RwLock<Vec<Arc<crate::wire::Wire>>>>,
        ip: String,
        port: u16,
    ) -> Result<()> {
        use crate::protocol::Handshake;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use hex;
        
        // Gracefully skip connecting to our own port
        let our_port = *client.torrent_port.read().await;
        if port == our_port {
            // Silently skip - this is our own listening port
            return Ok(());
        }
        
        // For local testing: if the port matches common test ports, try localhost first
        // This handles the case where the tracker returns a NAT'd IP but we're on the same machine
        let addr = if our_port > 0 && (port == 6881 || port == 6882) {
            // Try localhost first for local testing
            format!("127.0.0.1:{}", port)
        } else {
            format!("{}:{}", ip, port)
        };
        
        let addr: std::net::SocketAddr = addr.parse().map_err(|e| {
            crate::error::WebTorrentError::Network(format!("Invalid peer address {}: {}", addr, e))
        })?;
        
        eprintln!("[DEBUG] Connecting to {} (original: {}:{})", addr, ip, port);
        
        // Connect via TCP with timeout
        let connect_result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tokio::net::TcpStream::connect(addr)
        ).await;
        
        let mut stream = match connect_result {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                // If localhost failed and we tried localhost, try the original IP
                if addr.ip().is_loopback() && ip != "127.0.0.1" {
                    eprintln!("[DEBUG] Localhost connection failed, trying original IP {}:{}", ip, port);
                    let fallback_addr: std::net::SocketAddr = format!("{}:{}", ip, port).parse()
                        .map_err(|_| crate::error::WebTorrentError::Network(format!("Invalid fallback address {}:{}", ip, port)))?;
                    tokio::net::TcpStream::connect(fallback_addr).await.map_err(|e| {
                        crate::error::WebTorrentError::Network(format!("Failed to connect to {}:{}: {}", ip, port, e))
                    })?
                } else {
                    return Err(crate::error::WebTorrentError::Network(format!("Failed to connect to {}: {}", addr, e)));
                }
            }
            Err(_) => {
                return Err(crate::error::WebTorrentError::Network(format!("Connection timeout to {}", addr)));
            }
        };
        
        // Send handshake
        let handshake = Handshake::new(info_hash, client.peer_id);
        let handshake_bytes = handshake.encode();
        stream.write_all(&handshake_bytes).await.map_err(|e| {
            crate::error::WebTorrentError::Network(format!("Failed to send handshake: {}", e))
        })?;
        
        // Read peer's handshake
        let mut handshake_buf = [0u8; 68];
        stream.read_exact(&mut handshake_buf).await.map_err(|e| {
            crate::error::WebTorrentError::Network(format!("Failed to read handshake: {}", e))
        })?;
        
        let peer_handshake = Handshake::decode(&handshake_buf)?;
        
        // Verify info hash matches
        if peer_handshake.info_hash != info_hash {
            return Err(crate::error::WebTorrentError::Protocol(
                format!("Peer info hash mismatch: expected {}, got {}", 
                    hex::encode(info_hash),
                    hex::encode(peer_handshake.info_hash)
                )
            ));
        }
        
        let peer_id = hex::encode(peer_handshake.peer_id);
        let our_peer_id = hex::encode(client.peer_id);
        
        // Gracefully skip connecting to the same client instance (same peer_id)
        // But allow connecting to different clients on the same machine
        if peer_id == our_peer_id {
            // Silently close - this is our own client instance
            return Ok(());
        }
        
        eprintln!("[DEBUG] Handshake successful with peer {} (our peer_id: {})", peer_id, our_peer_id);
        
        // Check if peer already exists
        {
            let peers_guard = peers.read().await;
            if peers_guard.contains_key(&peer_id) {
                eprintln!("[DEBUG] Peer {} already connected", peer_id);
                return Ok(()); // Already connected
            }
        }
        
        // Create peer and wire
        let _peer = Arc::new(Peer::new(peer_id.clone(), crate::peer::PeerType::TcpOutgoing));
        let wire = Arc::new(crate::wire::Wire::new("tcp".to_string()));
        
        // Add to torrent
        {
            let mut peers_guard = peers.write().await;
            peers_guard.insert(peer_id.clone(), Arc::clone(&_peer));
        }
        {
            let mut wires_guard = wires.write().await;
            wires_guard.push(Arc::clone(&wire));
        }
        
        // Get torrent from client to pass to handle_peer_connection
        let torrent_opt = client.get(&info_hash).await;
        if let Some(torrent) = torrent_opt {
            // Spawn task to handle the connection
            let client_clone = Arc::clone(&client);
            tokio::spawn(async move {
                use crate::conn_pool::ConnPool;
                if let Err(e) = ConnPool::handle_peer_connection(
                    client_clone,
                    torrent,
                    wire,
                    stream,
                    addr,
                ).await {
                    tracing::error!("Error handling outbound peer connection to {}: {}", addr, e);
                }
            });
        }
        
        Ok(())
    }

    async fn parse_torrent_id(
        torrent_id: TorrentId,
        _client: &WebTorrent,
    ) -> Result<(
        [u8; 20],
        Option<Bytes>,
        Vec<String>,
        Vec<String>,
        bool,
        String,
        u64,
        u64,
        Vec<[u8; 20]>,
        Vec<File>,
    )> {
        match torrent_id {
            TorrentId::InfoHash(_hash) => {
                // Need to fetch metadata via DHT/tracker
                return Err(WebTorrentError::InvalidTorrent(
                    "Info hash only - metadata required".to_string(),
                ));
            }
            TorrentId::MagnetUri(uri) => {
                // Parse magnet URI
                use crate::magnet::MagnetUri;
                let magnet = MagnetUri::parse(&uri)?;
                
                // For magnet URIs, we only have the info hash initially
                // Metadata will be fetched via DHT/tracker using ut_metadata extension
                // For now, return error indicating metadata is needed
                return Err(WebTorrentError::InvalidTorrent(
                    format!("Magnet URI parsed successfully. Info hash: {}. Metadata fetching via ut_metadata extension will be implemented in discovery module.", hex::encode(magnet.info_hash)),
                ));
            }
            TorrentId::TorrentFile(data) => {
                Self::parse_torrent_file(data).await
            }
            TorrentId::Url(url) => {
                // Fetch torrent file from URL
                let response = reqwest::get(&url).await?;
                let data = response.bytes().await?;
                Self::parse_torrent_file(data.into()).await
            }
        }
    }

    async fn parse_torrent_file(
        data: Bytes,
    ) -> Result<(
        [u8; 20],
        Option<Bytes>,
        Vec<String>,
        Vec<String>,
        bool,
        String,
        u64,
        u64,
        Vec<[u8; 20]>,
        Vec<File>,
    )> {
        use crate::bencode_parser::parse_bencode;

        let (bencode, _) = parse_bencode(&data)?;
        // Ensure it's a dictionary (required for torrent files)
        if bencode.as_dict().is_none() {
            return Err(WebTorrentError::InvalidTorrent("Torrent file must be a dictionary".to_string()));
        }

        // Calculate info hash
        let info = bencode.get(b"info").ok_or_else(|| {
            WebTorrentError::InvalidTorrent("Torrent file missing 'info' field".to_string())
        })?;
        let info_bytes = info.encode();
        let mut hasher = Sha1::new();
        hasher.update(&info_bytes);
        let info_hash: [u8; 20] = hasher.finalize().into();

        // Parse announce
        let mut all_announce = Vec::new();
        if let Some(announce_val) = bencode.get(b"announce") {
            if let Some(s) = announce_val.as_string() {
                all_announce.push(s);
            }
        }
        if let Some(announce_list_val) = bencode.get(b"announce-list") {
            if let Some(list) = announce_list_val.as_list() {
                for item in list {
                    if let Some(inner_list) = item.as_list() {
                        for inner_item in inner_list {
                            if let Some(s) = inner_item.as_string() {
                                all_announce.push(s);
                            }
                        }
                    }
                }
            }
        }
        let all_announce: Vec<String> = all_announce.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();

        // Parse url-list (web seeds)
        let mut url_list = Vec::new();
        if let Some(url_list_val) = bencode.get(b"url-list") {
            if let Some(s) = url_list_val.as_string() {
                url_list.push(s);
            } else if let Some(list) = url_list_val.as_list() {
                for item in list {
                    if let Some(s) = item.as_string() {
                        url_list.push(s);
                    }
                }
            }
        }

        // Parse private flag
        let private = bencode
            .get(b"private")
            .and_then(|p| p.as_integer())
            .map(|i| i != 0)
            .unwrap_or(false);

        // Parse info dictionary
        let name = info
            .get(b"name")
            .and_then(|n| n.as_string())
            .ok_or_else(|| {
                WebTorrentError::InvalidTorrent("Info missing 'name' field".to_string())
            })?;

        let piece_length = info
            .get(b"piece length")
            .and_then(|pl| pl.as_integer())
            .ok_or_else(|| {
                WebTorrentError::InvalidTorrent("Info missing 'piece length' field".to_string())
            })? as u64;

        let pieces_str = info
            .get(b"pieces")
            .and_then(|p| p.as_bytes())
            .ok_or_else(|| {
                WebTorrentError::InvalidTorrent("Info missing 'pieces' field".to_string())
            })?;

        let piece_hashes: Vec<[u8; 20]> = pieces_str
            .chunks_exact(20)
            .map(|chunk| {
                let mut hash = [0u8; 20];
                hash.copy_from_slice(chunk);
                hash
            })
            .collect();

        // Parse files
        let (length, files) = if let Some(length_val) = info.get(b"length") {
            if let Some(length) = length_val.as_integer() {
                // Single file
                let length = length as u64;
                let files = vec![File::new(name.clone(), length, 0)];
                (length, files)
            } else {
                return Err(WebTorrentError::InvalidTorrent(
                    "Info 'length' field must be an integer".to_string(),
                ));
            }
        } else if let Some(file_list_val) = info.get(b"files") {
            if let Some(file_list) = file_list_val.as_list() {
                // Multiple files
                let mut files = Vec::new();
                let mut offset = 0u64;
                for file_val in file_list {
                    let file_length = file_val
                        .get(b"length")
                        .and_then(|l| l.as_integer())
                        .unwrap_or(0) as u64;
                    let path_parts = file_val
                        .get(b"path")
                        .and_then(|p| p.as_list())
                        .map(|list| {
                            list.iter()
                                .filter_map(|s| s.as_string())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let path = path_parts.join("/");
                    files.push(File::new(path, file_length, offset));
                    offset += file_length;
                }
                let total_length = offset;
                (total_length, files)
            } else {
                return Err(WebTorrentError::InvalidTorrent(
                    "Info 'files' field must be a list".to_string(),
                ));
            }
        } else {
            return Err(WebTorrentError::InvalidTorrent(
                "Info missing 'length' or 'files' field".to_string(),
            ));
        };

        Ok((
            info_hash,
            Some(data),
            all_announce,
            url_list,
            private,
            name,
            length,
            piece_length,
            piece_hashes,
            files,
        ))
    }

    pub fn info_hash(&self) -> [u8; 20] {
        self.info_hash
    }

    pub async fn length(&self) -> u64 {
        self.length
    }

    pub async fn downloaded(&self) -> u64 {
        *self.downloaded.read().await
    }

    pub async fn uploaded(&self) -> u64 {
        *self.uploaded.read().await
    }

    pub async fn received(&self) -> u64 {
        *self.received.read().await
    }

    pub async fn progress(&self) -> f64 {
        let downloaded = self.downloaded().await;
        if self.length == 0 {
            return 1.0;
        }
        downloaded as f64 / self.length as f64
    }

    pub async fn ratio(&self) -> f64 {
        let uploaded = self.uploaded().await;
        let received = self.received().await;
        if received == 0 {
            return 0.0;
        }
        uploaded as f64 / received as f64
    }

    pub async fn num_peers(&self) -> usize {
        self.wires.read().await.len()
    }

    /// Get the torrent's bitfield (for sending to peers)
    pub async fn get_bitfield(&self) -> bitvec::prelude::BitVec {
        self.bitfield.read().await.clone()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn files(&self) -> &[File] {
        &self.files
    }

    pub async fn destroy(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Ok(());
        }

        *self.destroyed.write().await = true;

        // Destroy discovery
        if let Some(discovery) = self.discovery.read().await.as_ref() {
            discovery.destroy().await?;
        }

        // Destroy peers
        let peers = self.peers.read().await.clone();
        for peer in peers.values() {
            peer.destroy().await?;
        }

        // Destroy store
        if let Some(store) = &self.store {
            store.close().await?;
        }

        Ok(())
    }
}

