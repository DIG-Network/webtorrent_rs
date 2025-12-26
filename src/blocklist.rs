use crate::error::{Result, WebTorrentError};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Represents a CIDR block (network address and prefix length)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CidrBlock {
    network: IpAddr,
    prefix_len: u8,
}

impl CidrBlock {
    fn contains(&self, ip: IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(net_v4), IpAddr::V4(ip_v4)) => {
                self.contains_v4(net_v4, ip_v4)
            }
            (IpAddr::V6(net_v6), IpAddr::V6(ip_v6)) => {
                self.contains_v6(net_v6, ip_v6)
            }
            _ => false,
        }
    }

    fn contains_v4(&self, network: Ipv4Addr, ip: Ipv4Addr) -> bool {
        if self.prefix_len > 32 {
            return false;
        }
        let mask = !0u32 << (32 - self.prefix_len);
        let network_u32 = u32::from_be_bytes(network.octets());
        let ip_u32 = u32::from_be_bytes(ip.octets());
        (network_u32 & mask) == (ip_u32 & mask)
    }

    fn contains_v6(&self, network: Ipv6Addr, ip: Ipv6Addr) -> bool {
        if self.prefix_len > 128 {
            return false;
        }
        let network_u128 = u128::from_be_bytes(network.octets());
        let ip_u128 = u128::from_be_bytes(ip.octets());
        let mask = if self.prefix_len == 128 {
            !0u128
        } else {
            !0u128 << (128 - self.prefix_len)
        };
        (network_u128 & mask) == (ip_u128 & mask)
    }
}

/// Blocklist for blocking malicious IP addresses
pub struct Blocklist {
    blocked_ips: Arc<RwLock<HashSet<IpAddr>>>,
    blocked_cidrs: Arc<RwLock<HashSet<CidrBlock>>>,
}

impl Blocklist {
    /// Create a new empty blocklist
    pub fn new() -> Self {
        Self {
            blocked_ips: Arc::new(RwLock::new(HashSet::new())),
            blocked_cidrs: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Check if an IP address is blocked
    pub async fn is_blocked(&self, ip: IpAddr) -> bool {
        // Check individual IPs
        {
            let ips = self.blocked_ips.read().await;
            if ips.contains(&ip) {
                return true;
            }
        }

        // Check CIDR blocks
        {
            let cidrs = self.blocked_cidrs.read().await;
            for cidr in cidrs.iter() {
                if cidr.contains(ip) {
                    return true;
                }
            }
        }

        false
    }

    /// Add an IP address to the blocklist
    pub async fn add_ip(&self, ip: IpAddr) {
        let mut ips = self.blocked_ips.write().await;
        ips.insert(ip);
    }

    /// Remove an IP address from the blocklist
    pub async fn remove_ip(&self, ip: IpAddr) {
        let mut ips = self.blocked_ips.write().await;
        ips.remove(&ip);
    }

    /// Add a CIDR block to the blocklist
    pub async fn add_cidr(&self, network: IpAddr, prefix_len: u8) -> Result<()> {
        // Validate prefix length
        match network {
            IpAddr::V4(_) if prefix_len > 32 => {
                return Err(WebTorrentError::InvalidTorrent(
                    format!("Invalid prefix length {} for IPv4", prefix_len)
                ));
            }
            IpAddr::V6(_) if prefix_len > 128 => {
                return Err(WebTorrentError::InvalidTorrent(
                    format!("Invalid prefix length {} for IPv6", prefix_len)
                ));
            }
            _ => {}
        }

        let mut cidrs = self.blocked_cidrs.write().await;
        cidrs.insert(CidrBlock {
            network,
            prefix_len,
        });
        Ok(())
    }

    /// Remove a CIDR block from the blocklist
    pub async fn remove_cidr(&self, network: IpAddr, prefix_len: u8) {
        let mut cidrs = self.blocked_cidrs.write().await;
        cidrs.remove(&CidrBlock {
            network,
            prefix_len,
        });
    }

    /// Load blocklist from a string (P2P format)
    /// Supports both IP addresses and CIDR notation
    pub async fn load_from_string(&self, data: &str) -> Result<()> {
        for line in data.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Try to parse as CIDR (e.g., "192.168.1.0/24" or "2001:db8::/32")
            if let Some(slash_pos) = line.find('/') {
                let network_str = &line[..slash_pos];
                let prefix_str = &line[slash_pos + 1..];
                
                if let Ok(prefix_len) = prefix_str.parse::<u8>() {
                    if let Ok(network) = network_str.parse::<IpAddr>() {
                        self.add_cidr(network, prefix_len).await?;
                        continue;
                    }
                }
            }

            // Try to parse as individual IP address
            if let Ok(ip) = line.parse::<IpAddr>() {
                self.add_ip(ip).await;
            }
        }

        Ok(())
    }

    /// Load blocklist from a file
    pub async fn load_from_file(&self, path: &str) -> Result<()> {
        let contents = tokio::fs::read_to_string(path).await.map_err(|e| {
            WebTorrentError::Network(format!("Failed to read blocklist file {}: {}", path, e))
        })?;
        self.load_from_string(&contents).await
    }

    /// Load blocklist from a URL
    pub async fn load_from_url(&self, url: &str) -> Result<()> {
        let response = reqwest::get(url).await.map_err(|e| {
            WebTorrentError::Network(format!("Failed to fetch blocklist from {}: {}", url, e))
        })?;

        if !response.status().is_success() {
            return Err(WebTorrentError::Network(format!(
                "Failed to fetch blocklist: HTTP {}",
                response.status()
            )));
        }

        let contents = response.text().await.map_err(|e| {
            WebTorrentError::Network(format!("Failed to read blocklist response: {}", e))
        })?;

        self.load_from_string(&contents).await
    }

    /// Get the number of blocked IPs
    pub async fn len(&self) -> usize {
        let ips = self.blocked_ips.read().await;
        let cidrs = self.blocked_cidrs.read().await;
        ips.len() + cidrs.len()
    }

    /// Check if the blocklist is empty
    pub async fn is_empty(&self) -> bool {
        let ips = self.blocked_ips.read().await;
        let cidrs = self.blocked_cidrs.read().await;
        ips.is_empty() && cidrs.is_empty()
    }

    /// Clear the blocklist
    pub async fn clear(&self) {
        let mut ips = self.blocked_ips.write().await;
        let mut cidrs = self.blocked_cidrs.write().await;
        ips.clear();
        cidrs.clear();
    }
}

impl Default for Blocklist {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blocklist_ip() {
        let blocklist = Blocklist::new();
        
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(!blocklist.is_blocked(ip).await);
        
        blocklist.add_ip(ip).await;
        assert!(blocklist.is_blocked(ip).await);
        
        blocklist.remove_ip(ip).await;
        assert!(!blocklist.is_blocked(ip).await);
    }

    #[tokio::test]
    async fn test_blocklist_cidr() {
        let blocklist = Blocklist::new();
        
        // Block 192.168.1.0/24
        let network: IpAddr = "192.168.1.0".parse().unwrap();
        blocklist.add_cidr(network, 24).await.unwrap();
        
        // Test IPs in the range
        assert!(blocklist.is_blocked("192.168.1.1".parse().unwrap()).await);
        assert!(blocklist.is_blocked("192.168.1.100".parse().unwrap()).await);
        assert!(blocklist.is_blocked("192.168.1.255".parse().unwrap()).await);
        
        // Test IPs outside the range
        assert!(!blocklist.is_blocked("192.168.2.1".parse().unwrap()).await);
        assert!(!blocklist.is_blocked("10.0.0.1".parse().unwrap()).await);
    }

    #[tokio::test]
    async fn test_blocklist_load_from_string() {
        let blocklist = Blocklist::new();
        
        let data = r#"
            # This is a comment
            192.168.1.1
            10.0.0.0/8
            172.16.0.0/12
            # Another comment
            203.0.113.0/24
        "#;
        
        blocklist.load_from_string(data).await.unwrap();
        
        assert!(blocklist.is_blocked("192.168.1.1".parse().unwrap()).await);
        assert!(blocklist.is_blocked("10.1.2.3".parse().unwrap()).await);
        assert!(blocklist.is_blocked("172.16.1.1".parse().unwrap()).await);
        assert!(blocklist.is_blocked("203.0.113.50".parse().unwrap()).await);
        assert!(!blocklist.is_blocked("8.8.8.8".parse().unwrap()).await);
    }
}

