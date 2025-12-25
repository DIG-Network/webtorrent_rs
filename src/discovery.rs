use crate::error::{Result, WebTorrentError};
use crate::dht::Dht;
use crate::tracker::TrackerClient;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;
use std::collections::HashSet;
use hex;

/// Handles peer discovery via DHT, trackers, LSD, and PEX
pub struct Discovery {
    info_hash: [u8; 20],
    #[allow(dead_code)]
    peer_id: [u8; 20],
    #[allow(dead_code)]
    announce: Vec<String>,
    #[allow(dead_code)]
    port: u16,
    dht: Option<Arc<Dht>>,
    trackers: Vec<Arc<TrackerClient>>,
    discovered_peers: Arc<RwLock<HashSet<String>>>, // "ip:port" format
    destroyed: Arc<RwLock<bool>>,
    #[allow(dead_code)]
    lsd_enabled: bool,
    #[allow(dead_code)]
    pex_enabled: bool,
}

impl Discovery {
    pub fn new(
        info_hash: [u8; 20],
        peer_id: [u8; 20],
        announce: Vec<String>,
        port: u16,
        dht_enabled: bool,
        lsd_enabled: bool,
        pex_enabled: bool,
    ) -> Self {
        let dht = if dht_enabled {
            Some(Arc::new(Dht::new(info_hash, peer_id, port)))
        } else {
            None
        };

        let trackers = announce.iter()
            .map(|url| Arc::new(TrackerClient::new(url.clone(), info_hash, peer_id, port)))
            .collect();

        Self {
            info_hash,
            peer_id,
            announce,
            port,
            dht,
            trackers,
            discovered_peers: Arc::new(RwLock::new(HashSet::new())),
            destroyed: Arc::new(RwLock::new(false)),
            lsd_enabled,
            pex_enabled,
        }
    }

    /// Start discovery
    pub async fn start(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Discovery("Discovery destroyed".to_string()));
        }

        // Start DHT if enabled
        if let Some(ref dht) = self.dht {
            dht.start().await?;
            dht.announce().await?;
        }

        // Announce to all trackers and collect peers
        let mut all_peers = Vec::new();
        for tracker in &self.trackers {
            match tracker.announce(0, 0, 0, "started").await {
                Ok((response, peers)) => {
                    eprintln!("[DEBUG] Announced to tracker: interval={:?}, complete={:?}, incomplete={:?}, peers={}", 
                        response.interval,
                        response.complete,
                        response.incomplete,
                        peers.len()
                    );
                    if !peers.is_empty() {
                        eprintln!("[DEBUG] Discovered {} peers from tracker", peers.len());
                        for (ip, port) in &peers {
                            eprintln!("[DEBUG]   Peer: {}:{}", ip, port);
                        }
                    } else {
                        eprintln!("[DEBUG] No peers returned from tracker (this is normal if no other peers have announced yet)");
                    }
                    all_peers.extend(peers);
                }
                Err(e) => {
                    eprintln!("[ERROR] Failed to announce to tracker: {}", e);
                }
            }
        }
        
        // Store discovered peers
        let mut discovered = self.discovered_peers.write().await;
        for (ip, port) in &all_peers {
            discovered.insert(format!("{}:{}", ip, port));
        }
        
        if !all_peers.is_empty() {
            debug!("Total discovered peers: {}", all_peers.len());
        }

        debug!("Discovery started for info hash: {}", hex::encode(self.info_hash));
        Ok(())
    }

    /// Announce completion to trackers
    pub async fn complete(&self, uploaded: u64, downloaded: u64) -> Result<()> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Discovery("Discovery destroyed".to_string()));
        }

        for tracker in &self.trackers {
            let _ = tracker.announce(uploaded, downloaded, 0, "completed").await.map(|(_, _)| ());
        }

        Ok(())
    }

    /// Lookup peers via all discovery methods
    pub async fn lookup_peers(&self) -> Result<Vec<(String, u16)>> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Discovery("Discovery destroyed".to_string()));
        }

        let mut all_peers = HashSet::new();

        // Lookup via DHT
        if let Some(ref dht) = self.dht {
            if let Ok(peers) = dht.lookup().await {
                for (ip, port) in peers {
                    all_peers.insert((ip, port));
                }
            }
        }

        // Lookup via trackers
        for tracker in &self.trackers {
            match tracker.announce(0, 0, 0, "started").await {
                Ok((response, peers)) => {
                    debug!("Tracker lookup: interval={:?}, complete={:?}, incomplete={:?}, peers={}", 
                        response.interval,
                        response.complete,
                        response.incomplete,
                        peers.len()
                    );
                    for (ip, port) in peers {
                        all_peers.insert((ip, port));
                    }
                }
                Err(e) => {
                    debug!("Tracker lookup failed: {}", e);
                }
            }
        }

        let peers_vec: Vec<(String, u16)> = all_peers.into_iter().collect();
        
        // Update discovered peers
        let mut discovered = self.discovered_peers.write().await;
        for (ip, port) in &peers_vec {
            discovered.insert(format!("{}:{}", ip, port));
        }

        Ok(peers_vec)
    }

    /// Add peer discovered via PEX
    pub async fn add_pex_peer(&self, ip: String, port: u16) {
        let mut discovered = self.discovered_peers.write().await;
        discovered.insert(format!("{}:{}", ip, port));
    }

    pub async fn destroy(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Ok(());
        }

        *self.destroyed.write().await = true;

        if let Some(ref dht) = self.dht {
            dht.destroy().await?;
        }

        self.discovered_peers.write().await.clear();
        Ok(())
    }
}


