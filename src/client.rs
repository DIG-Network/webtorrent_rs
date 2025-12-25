use crate::error::{Result, WebTorrentError};
use crate::torrent::Torrent;
use crate::conn_pool::ConnPool;
use crate::discovery::Discovery;
use crate::nat::NatTraversal;
use crate::throttling::ThrottleGroup;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, Instant};
use bytes::Bytes;
use rand::Rng;

/// WebTorrent Client Options
#[derive(Debug, Clone)]
pub struct WebTorrentOptions {
    pub peer_id: Option<[u8; 20]>,
    pub node_id: Option<[u8; 20]>,
    pub torrent_port: u16,
    pub dht_port: u16,
    pub max_conns: usize,
    pub utp: bool,
    pub nat_upnp: bool,
    pub nat_pmp: bool,
    pub lsd: bool,
    pub ut_pex: bool,
    pub seed_outgoing_connections: bool,
    pub download_limit: Option<u64>, // bytes per second, None = unlimited
    pub upload_limit: Option<u64>,   // bytes per second, None = unlimited
    pub blocklist: Option<String>,
    pub tracker: Option<TrackerConfig>,
    pub web_seeds: bool,
}

impl Default for WebTorrentOptions {
    fn default() -> Self {
        Self {
            peer_id: None,
            node_id: None,
            torrent_port: 0,
            dht_port: 0,
            max_conns: 55,
            utp: true,
            nat_upnp: true,
            nat_pmp: true,
            lsd: true,
            ut_pex: true,
            seed_outgoing_connections: true,
            download_limit: None,
            upload_limit: None,
            blocklist: None,
            tracker: None,
            web_seeds: true,
        }
    }
}

pub struct TrackerConfig {
    pub announce: Vec<String>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub get_announce_opts: Option<Box<dyn Fn() -> HashMap<String, String> + Send + Sync>>,
}

impl std::fmt::Debug for TrackerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackerConfig")
            .field("announce", &self.announce)
            .field("get_announce_opts", &"<function>")
            .finish()
    }
}

impl Clone for TrackerConfig {
    fn clone(&self) -> Self {
        Self {
            announce: self.announce.clone(),
            get_announce_opts: None, // Can't clone Fn trait objects
        }
    }
}

/// WebTorrent Client
pub struct WebTorrent {
    pub(crate) peer_id: [u8; 20],
    pub(crate) node_id: [u8; 20],
    pub(crate) options: WebTorrentOptions,
    pub(crate) torrents: Arc<RwLock<Vec<Arc<Torrent>>>>,
    pub(crate) conn_pool: Arc<RwLock<Option<Arc<ConnPool>>>>,
    pub(crate) nat_traversal: Option<Arc<NatTraversal>>,
    pub(crate) dht: Option<Arc<Discovery>>,
    pub(crate) destroyed: Arc<RwLock<bool>>,
    pub(crate) listening: Arc<RwLock<bool>>,
    pub(crate) ready: Arc<RwLock<bool>>,
    pub(crate) torrent_port: Arc<RwLock<u16>>,
    pub(crate) dht_port: Arc<RwLock<u16>>,
    pub(crate) event_tx: mpsc::UnboundedSender<ClientEvent>,
    pub(crate) event_rx: Arc<RwLock<mpsc::UnboundedReceiver<ClientEvent>>>,
    // Speed tracking
    download_speed_tracker: Arc<SpeedTracker>,
    upload_speed_tracker: Arc<SpeedTracker>,
    // Throttling
    download_throttle: Arc<ThrottleGroup>,
    upload_throttle: Arc<ThrottleGroup>,
}

/// Speed tracker using a rolling window
struct SpeedTracker {
    bytes: Arc<RwLock<Vec<(Instant, u64)>>>, // (timestamp, bytes)
    window: Duration,
}

impl SpeedTracker {
    fn new(window: Duration) -> Self {
        Self {
            bytes: Arc::new(RwLock::new(Vec::new())),
            window,
        }
    }

    async fn add_bytes(&self, amount: u64) {
        let now = Instant::now();
        let mut bytes = self.bytes.write().await;
        bytes.push((now, amount));
        
        // Remove entries outside the window
        let cutoff = now.checked_sub(self.window).unwrap_or(Instant::now());
        bytes.retain(|(time, _)| *time > cutoff);
    }

    async fn get_speed(&self) -> u64 {
        let bytes = self.bytes.read().await;
        if bytes.is_empty() {
            return 0;
        }

        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or(Instant::now());
        
        let total_bytes: u64 = bytes.iter()
            .filter(|(time, _)| *time > cutoff)
            .map(|(_, amount)| *amount)
            .sum();

        let oldest_time = bytes.iter()
            .filter(|(time, _)| *time > cutoff)
            .map(|(time, _)| *time)
            .min();

        let elapsed = if let Some(oldest) = oldest_time {
            now.duration_since(oldest)
        } else {
            Duration::from_secs(1) // Default to 1 second if no data
        };
        if elapsed.as_secs_f64() > 0.0 {
            (total_bytes as f64 / elapsed.as_secs_f64()) as u64
        } else {
            0
        }
    }
}

#[derive(Clone)]
pub enum ClientEvent {
    Ready,
    Listening,
    TorrentAdded(Arc<Torrent>),
    TorrentRemoved(Arc<Torrent>),
    Error(String), // Store error as string for Clone
    Download(u64),
    Upload(u64),
}

impl WebTorrent {
    /// Get the client's peer ID
    pub fn peer_id(&self) -> [u8; 20] {
        self.peer_id
    }
    
    /// Create a new WebTorrent client
    pub async fn new(options: WebTorrentOptions) -> Result<Self> {
        let peer_id = options.peer_id.unwrap_or_else(|| {
            let mut id = [0u8; 20];
            id[0..3].copy_from_slice(b"-WW");
            // Version string (4 bytes)
            let version_str = format!("{:04}", env!("CARGO_PKG_VERSION_MAJOR").parse::<u16>().unwrap_or(1) * 100 + 
                env!("CARGO_PKG_VERSION_MINOR").parse::<u16>().unwrap_or(0));
            let version_bytes = version_str.as_bytes();
            // Ensure exactly 4 bytes for version
            if version_bytes.len() >= 4 {
                id[3..7].copy_from_slice(&version_bytes[0..4]);
            } else {
                // Pad with zeros if shorter
                id[3..3+version_bytes.len()].copy_from_slice(version_bytes);
            }
            id[7] = b'-';
            // Random bytes
            let mut rng = rand::thread_rng();
            rng.fill(&mut id[8..]);
            id
        });

        let node_id = options.node_id.unwrap_or_else(|| {
            let mut id = [0u8; 20];
            let mut rng = rand::thread_rng();
            rng.fill(&mut id);
            id
        });

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Initialize speed trackers (1 second window)
        let download_speed_tracker = Arc::new(SpeedTracker::new(Duration::from_secs(1)));
        let upload_speed_tracker = Arc::new(SpeedTracker::new(Duration::from_secs(1)));

        // Initialize throttling
        let download_throttle = Arc::new(ThrottleGroup::new(
            options.download_limit.unwrap_or(u64::MAX),
            options.download_limit.is_some(),
        ));
        let upload_throttle = Arc::new(ThrottleGroup::new(
            options.upload_limit.unwrap_or(u64::MAX),
            options.upload_limit.is_some(),
        ));

        let mut client = Self {
            peer_id,
            node_id,
            options: options.clone(),
            torrents: Arc::new(RwLock::new(Vec::new())),
            conn_pool: Arc::new(RwLock::new(None)),
            nat_traversal: None,
            dht: None,
            destroyed: Arc::new(RwLock::new(false)),
            listening: Arc::new(RwLock::new(false)),
            ready: Arc::new(RwLock::new(false)),
            torrent_port: Arc::new(RwLock::new(options.torrent_port)),
            dht_port: Arc::new(RwLock::new(options.dht_port)),
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
            download_speed_tracker,
            upload_speed_tracker,
            download_throttle,
            upload_throttle,
        };

        // Initialize NAT traversal if enabled
        if options.nat_upnp || options.nat_pmp {
            let nat = Arc::new(NatTraversal::new(options.nat_upnp, options.nat_pmp).await?);
            client.nat_traversal = Some(nat);
        }

        // Connection pool will be initialized on first use to avoid circular reference
        // This is set up when the client starts listening

        // DHT will be initialized via libp2p when discovery starts
        // This will be handled in the discovery module

        Ok(client)
    }

    /// Add a torrent to the client
    pub async fn add(&self, torrent_id: impl Into<TorrentId>) -> Result<Arc<Torrent>> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::ClientDestroyed);
        }

        // Initialize connection pool if not already initialized
        // This ensures the client is listening on a port before announcing to trackers
        {
            let port = *self.torrent_port.read().await;
            if port > 0 {
                let mut conn_pool_guard = self.conn_pool.write().await;
                if conn_pool_guard.is_none() {
                    // Create a clone of self wrapped in Arc for the connection pool
                    // Note: This creates a new Arc, but the connection pool will use it
                    let client_for_pool = Arc::new(self.clone());
                    match ConnPool::new(client_for_pool).await {
                        Ok(pool) => {
                            *conn_pool_guard = Some(Arc::new(pool));
                            *self.listening.write().await = true;
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to initialize connection pool: {}. Tracker announcements may not work.", e);
                            // Still mark as listening if port is configured
                            *self.listening.write().await = true;
                        }
                    }
                } else {
                    *self.listening.write().await = true;
                }
            }
        }

        let torrent_id = torrent_id.into();
        let torrent = Torrent::new(torrent_id, self.clone()).await?;

        // Check for duplicates
        let info_hash = torrent.info_hash();
        let torrents = self.torrents.read().await;
        for existing in torrents.iter() {
            if existing.info_hash() == info_hash {
                return Err(WebTorrentError::DuplicateTorrent(hex::encode(info_hash)));
            }
        }
        drop(torrents);

        let torrent = Arc::new(torrent);
        
        // Start discovery for the torrent (will announce to trackers)
        torrent.start_discovery().await?;
        
        self.torrents.write().await.push(torrent.clone());

        self.event_tx.send(ClientEvent::TorrentAdded(torrent.clone()))
            .map_err(|_| WebTorrentError::Network("Event channel closed".to_string()))?;

        Ok(torrent)
    }

    /// Remove a torrent from the client
    pub async fn remove(&self, torrent: Arc<Torrent>) -> Result<()> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::ClientDestroyed);
        }

        let mut torrents = self.torrents.write().await;
        if let Some(pos) = torrents.iter().position(|t| Arc::ptr_eq(t, &torrent)) {
            torrents.remove(pos);
            torrent.destroy().await?;
            self.event_tx.send(ClientEvent::TorrentRemoved(torrent))
                .map_err(|_| WebTorrentError::Network("Event channel closed".to_string()))?;
        }

        Ok(())
    }

    /// Get a torrent by info hash
    pub async fn get(&self, info_hash: &[u8; 20]) -> Option<Arc<Torrent>> {
        let torrents = self.torrents.read().await;
        torrents.iter().find(|t| t.info_hash() == *info_hash).cloned()
    }

    /// Seed a file or data
    pub async fn seed(
        &self,
        name: String,
        data: Bytes,
        announce: Option<Vec<String>>,
    ) -> Result<Arc<Torrent>> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::ClientDestroyed);
        }

        use crate::torrent_creator::TorrentCreator;

        // Use provided announce or default tracker
        let announce_list = announce.unwrap_or_else(|| {
            vec!["http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/announce".to_string()]
        });

        // Create torrent
        let creator = TorrentCreator::new()
            .with_announce(announce_list.clone());
        
        let (torrent_file, info_hash) = creator.create_from_data(name.clone(), data.clone()).await?;

        // Check for duplicates
        if self.get(&info_hash).await.is_some() {
            return Err(WebTorrentError::DuplicateTorrent(hex::encode(info_hash)));
        }

        // Add torrent
        let torrent = self.add(torrent_file).await?;

        // Store the data in the torrent's store
        // This will be handled by the torrent when it's ready

        Ok(torrent)
    }

    /// Get download speed in bytes per second
    pub async fn download_speed(&self) -> u64 {
        self.download_speed_tracker.get_speed().await
    }

    /// Get upload speed in bytes per second
    pub async fn upload_speed(&self) -> u64 {
        self.upload_speed_tracker.get_speed().await
    }

    /// Record downloaded bytes for speed tracking
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) async fn record_download(&self, bytes: u64) {
        if bytes > 0 {
            self.download_speed_tracker.add_bytes(bytes).await;
            let _ = self.event_tx.send(ClientEvent::Download(bytes));
        }
    }

    /// Record uploaded bytes for speed tracking
    #[allow(dead_code)]
    pub(crate) async fn record_upload(&self, bytes: u64) {
        if bytes > 0 {
            self.upload_speed_tracker.add_bytes(bytes).await;
            let _ = self.event_tx.send(ClientEvent::Upload(bytes));
        }
    }

    /// Get overall progress (0.0 to 1.0)
    pub async fn progress(&self) -> f64 {
        let torrents = self.torrents.read().await;
        let mut total_downloaded = 0u64;
        let mut total_length = 0u64;

        for torrent in torrents.iter() {
            if torrent.progress().await < 1.0 {
                total_downloaded += torrent.downloaded().await;
                total_length += torrent.length().await;
            }
        }

        if total_length == 0 {
            return 1.0;
        }

        total_downloaded as f64 / total_length as f64
    }

    /// Get overall ratio (uploaded / downloaded)
    pub async fn ratio(&self) -> f64 {
        let torrents = self.torrents.read().await;
        let mut total_uploaded = 0u64;
        let mut total_received = 0u64;

        for torrent in torrents.iter() {
            total_uploaded += torrent.uploaded().await;
            total_received += torrent.received().await;
        }

        if total_received == 0 {
            return 0.0;
        }

        total_uploaded as f64 / total_received as f64
    }

    /// Set download throttle rate (bytes per second, None = unlimited)
    pub async fn throttle_download(&self, rate: Option<u64>) {
        if let Some(rate) = rate {
            self.download_throttle.set_rate(rate).await;
            self.download_throttle.set_enabled(true).await;
        } else {
            self.download_throttle.set_enabled(false).await;
        }
    }

    /// Set upload throttle rate (bytes per second, None = unlimited)
    pub async fn throttle_upload(&self, rate: Option<u64>) {
        if let Some(rate) = rate {
            self.upload_throttle.set_rate(rate).await;
            self.upload_throttle.set_enabled(true).await;
        } else {
            self.upload_throttle.set_enabled(false).await;
        }
    }

    /// Get download throttle group (for use by peers/wires)
    #[allow(dead_code)]
    pub(crate) fn download_throttle(&self) -> Arc<ThrottleGroup> {
        Arc::clone(&self.download_throttle)
    }

    /// Get upload throttle group (for use by peers/wires)
    #[allow(dead_code)]
    pub(crate) fn upload_throttle(&self) -> Arc<ThrottleGroup> {
        Arc::clone(&self.upload_throttle)
    }

    /// Destroy the client
    pub async fn destroy(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::ClientDestroyed);
        }

        *self.destroyed.write().await = true;

        // Destroy all torrents
        let torrents = self.torrents.read().await.clone();
        for torrent in torrents {
            let _ = torrent.destroy().await;
        }

        // Destroy connection pool
        if let Some(conn_pool) = self.conn_pool.read().await.as_ref() {
            conn_pool.destroy().await?;
        }

        // Destroy NAT traversal
        if let Some(nat) = &self.nat_traversal {
            nat.destroy().await?;
        }

        // Destroy DHT
        if let Some(dht) = &self.dht {
            dht.destroy().await?;
        }

        Ok(())
    }

    /// Get listening address
    pub async fn address(&self) -> Option<(String, u16)> {
        if !*self.listening.read().await {
            return None;
        }

        let port = *self.torrent_port.read().await;
        Some(("0.0.0.0".to_string(), port))
    }
}

impl Clone for WebTorrent {
    fn clone(&self) -> Self {
        Self {
            peer_id: self.peer_id,
            node_id: self.node_id,
            options: self.options.clone(),
            torrents: Arc::clone(&self.torrents),
            conn_pool: Arc::clone(&self.conn_pool),
            nat_traversal: self.nat_traversal.clone(),
            dht: self.dht.clone(),
            destroyed: Arc::clone(&self.destroyed),
            listening: Arc::clone(&self.listening),
            ready: Arc::clone(&self.ready),
            torrent_port: Arc::clone(&self.torrent_port),
            dht_port: Arc::clone(&self.dht_port),
            event_tx: self.event_tx.clone(),
            event_rx: Arc::clone(&self.event_rx),
            download_speed_tracker: Arc::clone(&self.download_speed_tracker),
            upload_speed_tracker: Arc::clone(&self.upload_speed_tracker),
            download_throttle: Arc::clone(&self.download_throttle),
            upload_throttle: Arc::clone(&self.upload_throttle),
        }
    }
}

/// Torrent identifier - can be info hash, magnet URI, or torrent file data
#[derive(Debug, Clone)]
pub enum TorrentId {
    InfoHash([u8; 20]),
    MagnetUri(String),
    TorrentFile(Bytes),
    Url(String),
}

impl From<[u8; 20]> for TorrentId {
    fn from(hash: [u8; 20]) -> Self {
        TorrentId::InfoHash(hash)
    }
}

impl From<String> for TorrentId {
    fn from(s: String) -> Self {
        if s.starts_with("magnet:") {
            TorrentId::MagnetUri(s)
        } else if s.starts_with("http://") || s.starts_with("https://") {
            TorrentId::Url(s)
        } else {
            // Assume it's a hex-encoded info hash
            if let Ok(bytes) = hex::decode(&s) {
                if bytes.len() == 20 {
                    let mut hash = [0u8; 20];
                    hash.copy_from_slice(&bytes);
                    return TorrentId::InfoHash(hash);
                }
            }
            TorrentId::MagnetUri(s) // Fallback to magnet URI
        }
    }
}

impl From<&str> for TorrentId {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

impl From<Bytes> for TorrentId {
    fn from(bytes: Bytes) -> Self {
        TorrentId::TorrentFile(bytes)
    }
}

impl From<Vec<u8>> for TorrentId {
    fn from(bytes: Vec<u8>) -> Self {
        TorrentId::TorrentFile(bytes.into())
    }
}

