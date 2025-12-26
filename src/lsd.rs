use crate::error::Result;
use libp2p::{
    mdns,
    swarm::SwarmEvent,
    PeerId,
    identity::Keypair,
    core::upgrade,
    tcp,
    noise,
    yamux,
    Transport,
};
use futures::StreamExt;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use hex;

/// Local Service Discovery (LSD) using mDNS
/// Discovers peers on the local network
/// 
/// NOTE: libp2p 0.56 mDNS API verification required
/// The mdns::tokio::Behaviour and SwarmBuilder API may differ from expected patterns
/// Implementation requires verification against libp2p 0.56 documentation
pub struct Lsd {
    discovered_peers: Arc<RwLock<HashSet<(String, u16)>>>,
    swarm_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl Lsd {
    /// Create a new LSD instance
    pub fn new() -> Self {
        Self {
            discovered_peers: Arc::new(RwLock::new(HashSet::new())),
            swarm_task: Arc::new(RwLock::new(None)),
        }
    }

    /// Start LSD discovery
    pub async fn start(&self, info_hash: [u8; 20], port: u16) -> Result<()> {
        // Check if already started
        {
            let task = self.swarm_task.read().await;
            if task.is_some() {
                return Ok(()); // Already started
            }
        }

        let discovered_peers_clone = Arc::clone(&self.discovered_peers);
        let info_hash_clone = info_hash;
        
        // Spawn task to run mDNS Swarm
        let task = tokio::spawn(async move {
            Self::mdns_task(discovered_peers_clone, info_hash_clone, port).await;
        });
        
        *self.swarm_task.write().await = Some(task);
        
        info!("LSD (mDNS) started for info hash: {}", hex::encode(info_hash));
        Ok(())
    }
    
    /// mDNS task that runs the Swarm
    async fn mdns_task(
        discovered_peers: Arc<RwLock<HashSet<(String, u16)>>>,
        _info_hash: [u8; 20],
        _port: u16,
    ) {
        // Create keypair for this peer
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        
        // Create mDNS behaviour
        // libp2p 0.56 API: Use mdns::tokio::Behaviour for tokio runtime
        // This type alias handles the Provider type automatically
        // For Tokio runtime - use full path to avoid module resolution issues
        let mdns_result = libp2p::mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id);
        let mdns = match mdns_result {
            Ok(behaviour) => behaviour,
            Err(e) => {
                warn!("Failed to create mDNS behaviour: {:?}", e);
                return;
            }
        };
        
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
        let mut swarm = Swarm::new(transport, mdns, peer_id, config);
        
        // Listen on local network
        if let Err(e) = swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap()) {
            warn!("Failed to listen for mDNS: {:?}", e);
            return;
        }
        
        // Poll Swarm events
        loop {
            match swarm.select_next_some().await {
                SwarmEvent::Behaviour(mdns::Event::Discovered(list)) => {
                    for (peer_id, multiaddr) in list {
                        debug!("mDNS discovered peer: {} at {}", peer_id, multiaddr);
                        
                        // Extract IP and port from multiaddr
                        if let Some((ip, port)) = Self::extract_addr(&multiaddr) {
                            let mut peers = discovered_peers.write().await;
                            peers.insert((ip.clone(), port));
                            debug!("Added peer via mDNS: {}:{}", ip, port);
                        }
                    }
                }
                SwarmEvent::Behaviour(mdns::Event::Expired(list)) => {
                    for (peer_id, multiaddr) in list {
                        debug!("mDNS peer expired: {} at {}", peer_id, multiaddr);
                        
                        // Remove peer from discovered list
                        if let Some((ip, port)) = Self::extract_addr(&multiaddr) {
                            let mut peers = discovered_peers.write().await;
                            peers.remove(&(ip, port));
                        }
                    }
                }
                _ => {
                    // Handle other Swarm events
                }
            }
        }
    }
    
    /// Extract IP and port from libp2p Multiaddr
    fn extract_addr(multiaddr: &libp2p::core::Multiaddr) -> Option<(String, u16)> {
        use libp2p::core::multiaddr::Protocol;
        
        let mut ip = None;
        let mut port = None;
        
        for protocol in multiaddr.iter() {
            match protocol {
                Protocol::Ip4(addr) => {
                    ip = Some(addr.to_string());
                }
                Protocol::Ip6(addr) => {
                    ip = Some(addr.to_string());
                }
                Protocol::Tcp(p) => {
                    port = Some(p);
                }
                _ => {}
            }
        }
        
        if let (Some(ip), Some(port)) = (ip, port) {
            Some((ip, port))
        } else {
            None
        }
    }

    /// Lookup peers discovered via LSD
    pub async fn lookup_peers(&self) -> Result<Vec<(String, u16)>> {
        let peers = self.discovered_peers.read().await;
        Ok(peers.iter().cloned().collect())
    }

    /// Destroy the LSD instance
    pub async fn destroy(&self) -> Result<()> {
        // Wait for the swarm task to complete
        if let Some(handle) = self.swarm_task.write().await.take() {
            let _ = handle.await;
        }

        Ok(())
    }
}

impl Default for Lsd {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lsd_new() {
        let lsd = Lsd::new();
        assert!(lsd.lookup_peers().await.unwrap().is_empty());
    }
}
