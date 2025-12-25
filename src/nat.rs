use crate::error::{Result, WebTorrentError};
use std::collections::HashMap;
use std::net::SocketAddrV4;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Port mapping information
#[derive(Debug, Clone)]
struct PortMapping {
    public_port: u16,
    private_port: u16,
    protocol: String,
    description: String,
    mapping_type: MappingType,
}

#[derive(Debug, Clone)]
enum MappingType {
    Upnp,
    Pmp,
}

/// NAT traversal using UPnP and NAT-PMP
pub struct NatTraversal {
    upnp_enabled: bool,
    pmp_enabled: bool,
    destroyed: Arc<RwLock<bool>>,
    mappings: Arc<RwLock<HashMap<String, PortMapping>>>,
}

impl NatTraversal {
    /// Create a new NAT traversal instance
    pub async fn new(upnp_enabled: bool, pmp_enabled: bool) -> Result<Self> {
        Ok(Self {
            upnp_enabled,
            pmp_enabled,
            destroyed: Arc::new(RwLock::new(false)),
            mappings: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Map a port via UPnP or NAT-PMP
    pub async fn map_port(
        &self,
        public_port: u16,
        private_port: u16,
        protocol: &str,
        description: &str,
    ) -> Result<()> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Nat("NAT traversal destroyed".to_string()));
        }

        // Check if already mapped
        let key = format!("{}:{}:{}", protocol, public_port, private_port);
        {
            let mappings = self.mappings.read().await;
            if mappings.contains_key(&key) {
                debug!("Port mapping already exists: {}", key);
                return Ok(());
            }
        }

        // Try UPnP first if enabled
        if self.upnp_enabled {
            match self.map_upnp(public_port, private_port, protocol, description).await {
                Ok(_) => {
                    let mapping = PortMapping {
                        public_port,
                        private_port,
                        protocol: protocol.to_string(),
                        description: description.to_string(),
                        mapping_type: MappingType::Upnp,
                    };
                    self.mappings.write().await.insert(key, mapping);
                    info!("UPnP port mapping successful: {} -> {} ({})", public_port, private_port, protocol);
                    return Ok(());
                }
                Err(e) => {
                    warn!("UPnP port mapping failed: {}", e);
                    // Fall through to try NAT-PMP if enabled
                }
            }
        }

        // Try NAT-PMP if enabled
        if self.pmp_enabled {
            match self.map_pmp(public_port, private_port, protocol, description).await {
                Ok(_) => {
                    let mapping = PortMapping {
                        public_port,
                        private_port,
                        protocol: protocol.to_string(),
                        description: description.to_string(),
                        mapping_type: MappingType::Pmp,
                    };
                    self.mappings.write().await.insert(key, mapping);
                    info!("NAT-PMP port mapping successful: {} -> {} ({})", public_port, private_port, protocol);
                    return Ok(());
                }
                Err(e) => {
                    error!("NAT-PMP port mapping failed: {}", e);
                    return Err(e);
                }
            }
        }

        Err(WebTorrentError::Nat("No NAT traversal method enabled or available".to_string()))
    }

    /// Unmap a port
    pub async fn unmap_port(
        &self,
        public_port: u16,
        private_port: u16,
        protocol: &str,
    ) -> Result<()> {
        if *self.destroyed.read().await {
            return Err(WebTorrentError::Nat("NAT traversal destroyed".to_string()));
        }

        let key = format!("{}:{}:{}", protocol, public_port, private_port);
        let mapping = {
            let mut mappings = self.mappings.write().await;
            mappings.remove(&key)
        };

        if let Some(mapping) = mapping {
            match mapping.mapping_type {
                MappingType::Upnp => {
                    if let Err(e) = self.unmap_upnp(public_port, private_port, &mapping.protocol).await {
                        warn!("UPnP port unmapping failed: {}", e);
                        // Continue anyway - mapping removed from tracking
                    } else {
                        debug!("UPnP port unmapping successful: {} -> {}", public_port, private_port);
                    }
                }
                MappingType::Pmp => {
                    if let Err(e) = self.unmap_pmp(public_port, private_port, &mapping.protocol).await {
                        warn!("NAT-PMP port unmapping failed: {}", e);
                        // Continue anyway - mapping removed from tracking
                    } else {
                        debug!("NAT-PMP port unmapping successful: {} -> {}", public_port, private_port);
                    }
                }
            }
        }

        Ok(())
    }

    async fn map_upnp(
        &self,
        public_port: u16,
        private_port: u16,
        protocol: &str,
        description: &str,
    ) -> Result<()> {
        use igd::*;

        // search_gateway is blocking, so run it in a blocking task
        let gateway = match tokio::task::spawn_blocking(|| {
            search_gateway(Default::default())
        }).await {
            Ok(Ok(gateway)) => gateway,
            Ok(Err(e)) => {
                return Err(WebTorrentError::Nat(format!("UPnP gateway search failed: {}", e)));
            }
            Err(e) => {
                return Err(WebTorrentError::Nat(format!("UPnP gateway search task failed: {}", e)));
            }
        };

        let protocol_enum = match protocol.to_lowercase().as_str() {
            "tcp" => PortMappingProtocol::TCP,
            "udp" => PortMappingProtocol::UDP,
            _ => {
                return Err(WebTorrentError::Nat(format!("Invalid protocol: {}", protocol)));
            }
        };

        // Map port with 3600 second (1 hour) lease time
        let lease_duration = 3600u32;
        let private_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, private_port);
        let description = description.to_string();
        let result = tokio::task::spawn_blocking(move || {
            gateway.add_port(protocol_enum, public_port, private_addr, lease_duration, &description)
        }).await.map_err(|e| {
            WebTorrentError::Nat(format!("UPnP port mapping task failed: {}", e))
        })?;
        
        match result {
            Ok(_) => {
                debug!("UPnP port mapping successful: {} -> {} ({})", public_port, private_port, protocol);
                Ok(())
            }
            Err(e) => Err(WebTorrentError::Nat(format!("UPnP port mapping failed: {}", e))),
        }
    }

    async fn unmap_upnp(
        &self,
        public_port: u16,
        _private_port: u16,
        protocol: &str,
    ) -> Result<()> {
        use igd::*;

        let gateway = match tokio::task::spawn_blocking(|| {
            search_gateway(Default::default())
        }).await {
            Ok(Ok(gateway)) => gateway,
            Ok(Err(e)) => {
                return Err(WebTorrentError::Nat(format!("UPnP gateway search failed: {}", e)));
            }
            Err(e) => {
                return Err(WebTorrentError::Nat(format!("UPnP gateway search task failed: {}", e)));
            }
        };

        let protocol_enum = match protocol.to_lowercase().as_str() {
            "tcp" => PortMappingProtocol::TCP,
            "udp" => PortMappingProtocol::UDP,
            _ => {
                return Err(WebTorrentError::Nat(format!("Invalid protocol: {}", protocol)));
            }
        };

        let result = tokio::task::spawn_blocking(move || {
            gateway.remove_port(protocol_enum, public_port)
        }).await.map_err(|e| {
            WebTorrentError::Nat(format!("UPnP port unmapping task failed: {}", e))
        })?;

        match result {
            Ok(_) => {
                debug!("UPnP port unmapping successful: {} ({})", public_port, protocol);
                Ok(())
            }
            Err(e) => Err(WebTorrentError::Nat(format!("UPnP port unmapping failed: {}", e))),
        }
    }

    async fn map_pmp(
        &self,
        public_port: u16,
        private_port: u16,
        protocol: &str,
        _description: &str,
    ) -> Result<()> {
        // NAT-PMP is synchronous, so run it in a blocking task
        let public_port = public_port;
        let private_port = private_port;
        let protocol = protocol.to_string();
        
        tokio::task::spawn_blocking(move || {
            use natpmp::Natpmp;

            // Create NAT-PMP client
            let mut nat = match Natpmp::new() {
                Ok(nat) => nat,
                Err(e) => {
                    return Err(WebTorrentError::Nat(format!("NAT-PMP client creation failed: {}", e)));
                }
            };

            // Get public IP address first
            match nat.send_public_address_request() {
                Ok(_) => {}
                Err(e) => {
                    return Err(WebTorrentError::Nat(format!("NAT-PMP public address request failed: {}", e)));
                }
            }

            // Read public address response
            let _public_addr = match nat.read_response_or_retry() {
                Ok(resp) => {
                    debug!("NAT-PMP public address response: {:?}", resp);
                    resp
                }
                Err(e) => {
                    return Err(WebTorrentError::Nat(format!("NAT-PMP response read failed: {}", e)));
                }
            };

            // Determine protocol - natpmp::Protocol enum
            // Based on natpmp 0.3 crate documentation, Protocol has TCP and UDP variants
            let protocol_enum = match protocol.to_lowercase().as_str() {
                "tcp" => natpmp::Protocol::TCP,
                "udp" => natpmp::Protocol::UDP,
                _ => {
                    return Err(WebTorrentError::Nat(format!("Invalid protocol: {}", protocol)));
                }
            };

            // Request port mapping with 3600 second (1 hour) lease time
            let lease_duration = 3600u32;
            match nat.send_port_mapping_request(protocol_enum, private_port, public_port, lease_duration) {
                Ok(_) => {}
                Err(e) => {
                    return Err(WebTorrentError::Nat(format!("NAT-PMP port mapping request failed: {}", e)));
                }
            }

            // Read port mapping response
            let _port_mapping_response = match nat.read_response_or_retry() {
                Ok(resp) => {
                    debug!("NAT-PMP port mapping response: {:?}", resp);
                    resp
                }
                Err(e) => {
                    return Err(WebTorrentError::Nat(format!("NAT-PMP port mapping response read failed: {}", e)));
                }
            };

            debug!("NAT-PMP port mapping successful: {} -> {} ({})", 
                public_port, 
                private_port,
                protocol
            );
            Ok(())
        }).await.map_err(|e| {
            WebTorrentError::Nat(format!("NAT-PMP task failed: {}", e))
        })?
    }

    async fn unmap_pmp(
        &self,
        public_port: u16,
        private_port: u16,
        protocol: &str,
    ) -> Result<()> {
        let public_port = public_port;
        let private_port = private_port;
        let protocol = protocol.to_string();

        tokio::task::spawn_blocking(move || {
            use natpmp::Natpmp;

            let mut nat = match Natpmp::new() {
                Ok(nat) => nat,
                Err(e) => {
                    return Err(WebTorrentError::Nat(format!("NAT-PMP client creation failed: {}", e)));
                }
            };

            // Determine protocol - use natpmp::Protocol enum
            let protocol_enum = match protocol.to_lowercase().as_str() {
                "tcp" => natpmp::Protocol::TCP,
                "udp" => natpmp::Protocol::UDP,
                _ => {
                    return Err(WebTorrentError::Nat(format!("Invalid protocol: {}", protocol)));
                }
            };

            // Remove port mapping by setting lease time to 0
            match nat.send_port_mapping_request(protocol_enum, private_port, public_port, 0) {
                Ok(_) => {}
                Err(e) => {
                    return Err(WebTorrentError::Nat(format!("NAT-PMP port unmapping request failed: {}", e)));
                }
            }

            // Read response (may fail for unmapping, but that's okay)
            let _ = nat.read_response_or_retry();

            debug!("NAT-PMP port unmapping successful: {} ({})", public_port, protocol);
            Ok(())
        }).await.map_err(|e| {
            WebTorrentError::Nat(format!("NAT-PMP unmapping task failed: {}", e))
        })?
    }

    /// Get all active port mappings
    pub async fn get_mappings(&self) -> Vec<(u16, u16, String, String)> {
        let mappings = self.mappings.read().await;
        mappings.values()
            .map(|m| (m.public_port, m.private_port, m.protocol.clone(), m.description.clone()))
            .collect()
    }

    /// Check if a port is mapped
    pub async fn is_mapped(&self, public_port: u16, private_port: u16, protocol: &str) -> bool {
        let key = format!("{}:{}:{}", protocol, public_port, private_port);
        self.mappings.read().await.contains_key(&key)
    }

    /// Destroy the NAT traversal instance and unmap all ports
    pub async fn destroy(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Ok(());
        }

        *self.destroyed.write().await = true;

        // Unmap all ports
        let mappings = {
            let mut mappings = self.mappings.write().await;
            mappings.drain().collect::<Vec<_>>()
        };

        for (_, mapping) in mappings {
            let _ = match mapping.mapping_type {
                MappingType::Upnp => {
                    self.unmap_upnp(mapping.public_port, mapping.private_port, &mapping.protocol).await
                }
                MappingType::Pmp => {
                    self.unmap_pmp(mapping.public_port, mapping.private_port, &mapping.protocol).await
                }
            };
        }

        info!("NAT traversal destroyed and all ports unmapped");
        Ok(())
    }
}
