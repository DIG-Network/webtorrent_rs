use crate::client::WebTorrent;
use crate::error::Result;
use crate::protocol::Handshake;
use bytes::BufMut;
use bytes::BytesMut;
use hex;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

// BitVec types used in message handling

/// Connection pool for managing incoming TCP/uTP connections
pub struct ConnPool {
    #[allow(dead_code)]
    client: Arc<WebTorrent>,
    #[allow(dead_code)]
    tcp_server: Option<tokio::net::TcpListener>,
    #[allow(dead_code)]
    utp_server: Option<Arc<RwLock<UtpServer>>>, // uTP server - wrapped in Arc<RwLock> for mutability
    destroyed: Arc<RwLock<bool>>,
    #[allow(dead_code)]
    handle: Option<tokio::task::JoinHandle<()>>,
}

/// uTP connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UtpConnectionState {
    SynSent,
    SynReceived,
    Established,
    FinSent,
    FinReceived,
    Closed,
}

/// uTP connection information
#[derive(Clone)]
struct UtpConnection {
    connection_id: u16,
    state: UtpConnectionState,
    seq_nr: u16,
    ack_nr: u16,
    window_size: u32,
    last_timestamp: u32,
    remote_addr: std::net::SocketAddr,
    created_at: std::time::Instant,
}

/// uTP server for handling incoming uTP connections
/// uTP (Micro Transport Protocol) is a UDP-based transport protocol
/// used as an alternative to TCP for BitTorrent connections
struct UtpServer {
    #[allow(dead_code)]
    socket: Arc<tokio::net::UdpSocket>,
    #[allow(dead_code)]
    port: u16,
    destroyed: Arc<RwLock<bool>>,
    #[allow(dead_code)]
    handle: Option<tokio::task::JoinHandle<()>>,
    // Track uTP connections by connection ID and remote address
    connections: Arc<RwLock<std::collections::HashMap<(std::net::SocketAddr, u16), UtpConnection>>>,
}

impl UtpServer {
    /// Create a new uTP server bound to the specified port
    async fn new(port: u16, client: Arc<WebTorrent>) -> Result<Self> {
        let addr = format!("0.0.0.0:{}", port);
        let socket = tokio::net::UdpSocket::bind(&addr).await.map_err(|e| {
            crate::error::WebTorrentError::Network(format!("Failed to bind uTP server: {}", e))
        })?;
        
        info!("uTP server listening on {}", addr);
        
        let socket = Arc::new(socket);
        let destroyed = Arc::new(RwLock::new(false));
        let connections = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let destroyed_clone = Arc::clone(&destroyed);
        let socket_clone = Arc::clone(&socket);
        let client_clone = Arc::clone(&client);
        let connections_clone = Arc::clone(&connections);
        
        // Spawn task to handle incoming uTP connections
        let handle = tokio::spawn(async move {
            Self::accept_loop(socket_clone, client_clone, destroyed_clone, connections_clone).await;
        });
        
        Ok(Self {
            socket,
            port,
            destroyed,
            handle: Some(handle),
            connections,
        })
    }
    
    /// Main accept loop for uTP connections
    async fn accept_loop(
        socket: Arc<tokio::net::UdpSocket>,
        client: Arc<WebTorrent>,
        destroyed: Arc<RwLock<bool>>,
        connections: Arc<RwLock<std::collections::HashMap<(std::net::SocketAddr, u16), UtpConnection>>>,
    ) {
        let mut buffer = vec![0u8; 1500]; // Standard MTU size
        
        loop {
            // Check if destroyed
            if *destroyed.read().await {
                break;
            }
            
            // Receive UDP packet
            let socket_clone = Arc::clone(&socket);
            match socket_clone.recv_from(&mut buffer).await {
                Ok((len, addr)) => {
                    debug!("Received uTP packet from {} ({} bytes)", addr, len);
                    
                    // Parse uTP packet and handle connection
                    let client_task = Arc::clone(&client);
                    let packet_data = buffer[..len].to_vec();
                    let socket_for_task = Arc::clone(&socket);
                    
                    let connections_clone = Arc::clone(&connections);
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_utp_packet(
                            client_task,
                            socket_for_task,
                            addr,
                            packet_data,
                            connections_clone,
                        ).await {
                            error!("Error handling uTP packet from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    if *destroyed.read().await {
                        break;
                    }
                    error!("Error receiving uTP packet: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
    
    /// Get current timestamp in microseconds (uTP uses microsecond timestamps)
    fn get_timestamp() -> u32 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u32
    }
    
    /// Build uTP packet header with proper fields
    fn build_utp_header(
        packet_type: u8,
        version: u8,
        connection_id: u16,
        timestamp: u32,
        timestamp_difference: u32,
        window_size: u32,
        seq_nr: u16,
        ack_nr: u16,
    ) -> [u8; 20] {
        let mut header = [0u8; 20];
        header[0] = (packet_type << 4) | (version & 0x0F);
        header[1..3].copy_from_slice(&connection_id.to_be_bytes());
        header[3..7].copy_from_slice(&timestamp.to_be_bytes());
        header[7..11].copy_from_slice(&timestamp_difference.to_be_bytes());
        header[11..15].copy_from_slice(&window_size.to_be_bytes());
        header[15..17].copy_from_slice(&seq_nr.to_be_bytes());
        header[17..19].copy_from_slice(&ack_nr.to_be_bytes());
        header
    }
    
    /// Handle incoming uTP packet and establish connection if needed
    async fn handle_utp_packet(
        client: Arc<WebTorrent>,
        socket: Arc<tokio::net::UdpSocket>,
        addr: std::net::SocketAddr,
        packet: Vec<u8>,
        connections: Arc<RwLock<std::collections::HashMap<(std::net::SocketAddr, u16), UtpConnection>>>,
    ) -> Result<()> {
        // Parse uTP packet header
        // uTP packet format: [type_version(1)][connection_id(2)][timestamp(4)][timestamp_difference(4)][wnd_size(4)][seq_nr(2)][ack_nr(2)][data...]
        if packet.len() < 20 {
            return Err(crate::error::WebTorrentError::Protocol(
                "uTP packet too short".to_string()
            ));
        }
        
        let type_version = packet[0];
        let packet_type = (type_version >> 4) & 0x0F;
        let version = type_version & 0x0F;
        let connection_id = u16::from_be_bytes([packet[1], packet[2]]);
        let timestamp = u32::from_be_bytes([packet[3], packet[4], packet[5], packet[6]]);
        let _timestamp_difference = u32::from_be_bytes([packet[7], packet[8], packet[9], packet[10]]);
        let window_size = u32::from_be_bytes([packet[11], packet[12], packet[13], packet[14]]);
        let seq_nr = u16::from_be_bytes([packet[15], packet[16]]);
        let ack_nr = u16::from_be_bytes([packet[17], packet[18]]);
        
        // Get or create connection state
        let connection_key = (addr, connection_id);
        let mut connection_opt = {
            let mut conns = connections.write().await;
            if let Some(c) = conns.get_mut(&connection_key) {
                // Update last timestamp
                c.last_timestamp = timestamp;
                Some(c.clone())
            } else {
                None
            }
        };
        
        if packet_type == 1 {
            // SYN packet - new connection
            debug!("uTP SYN packet from {} (connection_id: {}, seq: {}, ack: {})", 
                addr, connection_id, seq_nr, ack_nr);
            
            // Calculate timestamp difference (our timestamp - their timestamp)
            let our_timestamp = Self::get_timestamp();
            let ts_diff = our_timestamp.wrapping_sub(timestamp);
            
            // Create connection state
            let connection = UtpConnection {
                connection_id,
                state: UtpConnectionState::SynReceived,
                seq_nr: 1, // Start our sequence at 1
                ack_nr: seq_nr.wrapping_add(1), // ACK their sequence + 1
                window_size: 65536, // Default window size
                last_timestamp: timestamp,
                remote_addr: addr,
                created_at: std::time::Instant::now(),
            };
            
            // Store connection state
            {
                let mut conns = connections.write().await;
                conns.insert(connection_key, connection.clone());
            }
            
            // Send SYN-ACK response
            let header = Self::build_utp_header(
                2, // ST_STATE (SYN-ACK)
                version,
                connection_id,
                our_timestamp,
                ts_diff,
                connection.window_size,
                connection.seq_nr,
                connection.ack_nr,
            );
            
            socket.send_to(&header, addr).await.map_err(|e| {
                crate::error::WebTorrentError::Network(format!("Failed to send uTP SYN-ACK: {}", e))
            })?;
            
            // If handshake data is included, handle it
            if packet.len() > 20 {
                let handshake_data = &packet[20..];
                if handshake_data.len() >= 68 {
                    // This looks like a BitTorrent handshake
                    Self::handle_utp_connection(client, socket, addr, connection_id, handshake_data).await?;
                }
            }
        } else if packet_type == 0 {
            // DATA packet - existing connection
            debug!("uTP DATA packet from {} (connection_id: {}, seq: {}, ack: {})", 
                addr, connection_id, seq_nr, ack_nr);
            
            // Update connection state
            if let Some(ref mut conn) = connection_opt {
                // Update ACK number
                conn.ack_nr = seq_nr.wrapping_add(1);
                // Update window size if provided
                if window_size > 0 {
                    conn.window_size = window_size;
                }
                // Update state to established if not already
                if conn.state == UtpConnectionState::SynReceived {
                    conn.state = UtpConnectionState::Established;
                }
            } else {
                // Connection not found - might be a stray packet, ignore
                debug!("uTP DATA packet from {} for unknown connection {}", addr, connection_id);
                return Ok(());
            }
            
            // Extract BitTorrent message from data
            if packet.len() >= 20 {
                let message_data = &packet[20..];
                if message_data.len() >= 68 {
                    // This looks like a BitTorrent handshake
                    Self::handle_utp_connection(client, socket, addr, connection_id, message_data).await?;
                } else if !message_data.is_empty() {
                    // Regular BitTorrent message
                    // Handle it via the unified message handler
                    // Note: We'd need to get the torrent and wire from the connection state
                    // Regular BitTorrent messages are handled in handle_utp_peer_connection
                }
            }
        } else if packet_type == 2 {
            // ST_STATE (SYN-ACK) - response to our SYN
            debug!("uTP SYN-ACK from {} (connection_id: {}, seq: {}, ack: {})", 
                addr, connection_id, seq_nr, ack_nr);
            
            // Update connection state to established
            if let Some(ref mut conn) = connection_opt {
                conn.state = UtpConnectionState::Established;
                conn.ack_nr = seq_nr.wrapping_add(1);
            } else {
                // Create connection state if it doesn't exist (outgoing connection)
                let connection = UtpConnection {
                    connection_id,
                    state: UtpConnectionState::Established,
                    seq_nr: 1,
                    ack_nr: seq_nr.wrapping_add(1),
                    window_size: window_size.max(65536),
                    last_timestamp: timestamp,
                    remote_addr: addr,
                    created_at: std::time::Instant::now(),
                };
                let mut conns = connections.write().await;
                conns.insert(connection_key, connection);
            }
        } else if packet_type == 3 {
            // RESET packet
            debug!("uTP RESET from {} (connection_id: {})", addr, connection_id);
            // Connection reset - clean up state
            let mut conns = connections.write().await;
            conns.remove(&connection_key);
        }
        
        Ok(())
    }
    
    /// Handle established uTP connection with BitTorrent protocol
    async fn handle_utp_connection(
        client: Arc<WebTorrent>,
        socket: Arc<tokio::net::UdpSocket>,
        addr: std::net::SocketAddr,
        connection_id: u16,
        handshake_data: &[u8],
    ) -> Result<()> {
        // Parse BitTorrent handshake
        let handshake = Handshake::decode(handshake_data)?;
        let info_hash = handshake.info_hash;
        
        debug!("uTP connection for info hash: {} from {}", hex::encode(info_hash), addr);
        
        // Check blocklist
        if client.blocklist.is_blocked(addr.ip()).await {
            debug!("Blocked incoming uTP connection from {} (blocklisted)", addr);
            return Ok(()); // Silently reject blocked IPs
        }
        
        // Find the torrent
        let torrent = if let Some(t) = client.get(&info_hash).await {
            t
        } else {
            warn!("No torrent found for info hash: {} from {}", hex::encode(info_hash), addr);
            return Ok(()); // Close connection if torrent not found
        };
        
        // Create peer and wire
        let peer_id = handshake.peer_id;
        let peer_id_str = hex::encode(peer_id);
        
        // Check if peer already exists
        let peer_exists = {
            let peers = torrent.peers.read().await;
            peers.contains_key(&peer_id_str)
        };
        
        if peer_exists {
            debug!("Peer {} already exists for torrent {}", peer_id_str, hex::encode(info_hash));
            return Ok(()); // Close connection if peer already exists
        }
        
        // Send our handshake
        let our_handshake = Handshake::new(
            info_hash,
            client.peer_id,
        );
        let handshake_bytes = our_handshake.encode();
        
        // Send handshake over uTP
        // Build uTP DATA packet with handshake
        let mut utp_packet = vec![0u8; 20 + handshake_bytes.len()];
        utp_packet[0] = 0x01; // DATA packet + version 1
        utp_packet[1..3].copy_from_slice(&connection_id.to_be_bytes());
        // Add other uTP header fields (timestamp, etc.)
        utp_packet[20..].copy_from_slice(&handshake_bytes);
        
        socket.send_to(&utp_packet, addr).await.map_err(|e| {
            crate::error::WebTorrentError::Network(format!("Failed to send uTP handshake: {}", e))
        })?;
        
        // Create wire for this connection
        let wire = Arc::new(crate::wire::Wire::new("utp".to_string()));
        
        // Create peer with address
        let peer_addr = format!("{}:{}", addr.ip(), addr.port());
        let peer = Arc::new(crate::peer::Peer::new(
            peer_id_str.clone(),
            crate::peer::PeerType::UtpIncoming,
        ).with_addr(peer_addr.clone()));
        
        // Add peer and wire to torrent
        {
            let mut peers = torrent.peers.write().await;
            peers.insert(peer_id_str.clone(), Arc::clone(&peer));
        }
        
        {
            let mut wires = torrent.wires.write().await;
            wires.push(Arc::clone(&wire));
        }
        
        info!("Accepted uTP connection from {} for torrent {}", addr, hex::encode(info_hash));
        
        // Continue with BitTorrent protocol handling over uTP
        // Spawn a task to handle the connection
        let wire_clone = Arc::clone(&wire);
        let torrent_clone = Arc::clone(&torrent);
        let client_clone = Arc::clone(&client);
        
        tokio::spawn(async move {
            if let Err(e) = Self::handle_utp_peer_connection(
                client_clone,
                torrent_clone,
                wire_clone,
                socket,
                addr,
                connection_id,
            ).await {
                error!("Error handling uTP peer connection from {}: {}", addr, e);
            }
        });
        
        Ok(())
    }
    
    /// Handle ongoing uTP peer connection - reads and processes BitTorrent messages
    async fn handle_utp_peer_connection(
        client: Arc<WebTorrent>,
        torrent: Arc<crate::torrent::Torrent>,
        wire: Arc<crate::wire::Wire>,
        socket: Arc<tokio::net::UdpSocket>,
        addr: std::net::SocketAddr,
        connection_id: u16,
    ) -> Result<()> {
        use crate::protocol::MessageType;
        use std::io;

        // Enable PEX if enabled and torrent is not private
        if client.options.ut_pex && !torrent.is_private() {
            wire.enable_pex().await;
        }

        // Send bitfield if we have pieces
        let bitfield = torrent.get_bitfield().await;
        if bitfield.any() {
            Self::send_utp_bitfield(&socket, addr, connection_id, &bitfield).await?;
        }
        
        // Send unchoke and interested
        Self::send_utp_message(&socket, addr, connection_id, MessageType::Unchoke, None).await?;
        Self::send_utp_message(&socket, addr, connection_id, MessageType::Interested, None).await?;
        
        // Read messages in a loop
        let mut buffer = vec![0u8; 1500];
        
        loop {
            // Check if wire is destroyed
            if wire.destroyed().await {
                break;
            }
            
            // Receive uTP packet
            match socket.recv_from(&mut buffer).await {
                Ok((len, recv_addr)) => {
                    if recv_addr != addr {
                        continue; // Ignore packets from other addresses
                    }
                    
                    // Parse uTP packet
                    if len < 20 {
                        continue; // Invalid packet
                    }
                    
                    let packet_connection_id = u16::from_be_bytes([buffer[1], buffer[2]]);
                    if packet_connection_id != connection_id {
                        continue; // Wrong connection ID
                    }
                    
                    // Extract BitTorrent message from uTP data
                    let message_data = &buffer[20..len];
                    
                    if message_data.is_empty() {
                        continue; // Keep-alive
                    }
                    
                    // Parse and handle BitTorrent message using unified handler
                    ConnPool::handle_bittorrent_message(
                        client.clone(),
                        torrent.clone(),
                        wire.clone(),
                        addr,
                        message_data,
                    ).await?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    continue;
                }
                Err(e) => {
                    debug!("uTP connection closed by peer {}: {}", addr, e);
                    break;
                }
            }
        }
        
        // Clean up
        wire.destroy().await?;
        Ok(())
    }
    
    
    /// Send a BitTorrent message over uTP
    async fn send_utp_message(
        socket: &Arc<tokio::net::UdpSocket>,
        addr: std::net::SocketAddr,
        connection_id: u16,
        message_type: crate::protocol::MessageType,
        payload: Option<Vec<u8>>,
    ) -> Result<()> {
        // Build BitTorrent message
        let payload_len = payload.as_ref().map(|p| p.len()).unwrap_or(0);
        let mut message = BytesMut::with_capacity(1 + payload_len);
        message.put_u8(message_type as u8);
        if let Some(payload) = payload {
            message.put_slice(&payload);
        }
        
        // Build uTP header with proper fields
        let timestamp = Self::get_timestamp();
        let header = Self::build_utp_header(
            0, // DATA packet
            1, // Version 1
            connection_id,
            timestamp,
            0, // Timestamp difference (not used for outgoing)
            65536, // Window size
            1, // Sequence number (would be tracked per connection)
            0, // ACK number (would be tracked per connection)
        );
        
        // Build complete uTP packet
        let mut utp_packet = Vec::with_capacity(20 + message.len());
        utp_packet.extend_from_slice(&header);
        utp_packet.extend_from_slice(&message);
        
        socket.send_to(&utp_packet, addr).await.map_err(|e| {
            crate::error::WebTorrentError::Network(format!("Failed to send uTP message: {}", e))
        })?;
        
        Ok(())
    }
    
    /// Send bitfield message over uTP - uses unified bitfield conversion
    async fn send_utp_bitfield(
        socket: &Arc<tokio::net::UdpSocket>,
        addr: std::net::SocketAddr,
        connection_id: u16,
        bitfield: &bitvec::prelude::BitVec,
    ) -> Result<()> {
        let bitfield_bytes = ConnPool::bitfield_to_bytes(bitfield);
        
        let mut message = BytesMut::with_capacity(1 + bitfield_bytes.len());
        message.put_u8(5); // Bitfield message ID
        message.put_slice(&bitfield_bytes);
        
        // Build uTP header with proper fields
        let timestamp = Self::get_timestamp();
        let header = Self::build_utp_header(
            0, // DATA packet
            1, // Version 1
            connection_id,
            timestamp,
            0, // Timestamp difference (not used for outgoing)
            65536, // Window size
            1, // Sequence number (would be tracked per connection)
            0, // ACK number (would be tracked per connection)
        );
        
        // Build complete uTP packet
        let mut utp_packet = Vec::with_capacity(20 + message.len());
        utp_packet.extend_from_slice(&header);
        utp_packet.extend_from_slice(&message);
        
        socket.send_to(&utp_packet, addr).await.map_err(|e| {
            crate::error::WebTorrentError::Network(format!("Failed to send uTP bitfield: {}", e))
        })?;
        
        Ok(())
    }
    
    /// Destroy the uTP server
    #[allow(dead_code)]
    async fn destroy(&mut self) -> Result<()> {
        *self.destroyed.write().await = true;
        
        // Wait for accept loop to finish
        if let Some(handle) = self.handle.take() {
            let _ = handle.await;
        }
        
        Ok(())
    }
}

impl ConnPool {
    pub async fn new(client: Arc<WebTorrent>) -> Result<Self> {
        let torrent_port = *client.torrent_port.read().await;
        
        let tcp_server = if torrent_port > 0 {
            let addr = format!("0.0.0.0:{}", torrent_port);
            let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
                crate::error::WebTorrentError::Network(format!("Failed to bind TCP server: {}", e))
            })?;
            info!("TCP server listening on {}", addr);
            
            // Map port via NAT traversal if enabled
            if let Some(ref nat) = client.nat_traversal {
                let port = torrent_port;
                let nat_clone = Arc::clone(nat);
                tokio::spawn(async move {
                    // Determine protocol - TCP for torrent port (unless uTP is enabled, then it's null in JS version)
                    // In JS version, if utp is enabled, protocol is null, otherwise it's 'tcp'
                    // Map TCP port via NAT traversal
                    if let Err(e) = nat_clone.map_port(port, port, "tcp", "WebTorrent Torrent").await {
                        warn!("Failed to map torrent port {} via NAT traversal: {}", port, e);
                    } else {
                        info!("Mapped torrent port {} via NAT traversal", port);
                    }
                });
            }
            
            Some(listener)
        } else {
            None
        };

        let utp_server = if client.options.utp && torrent_port > 0 {
            // Create uTP server
            match UtpServer::new(torrent_port, Arc::clone(&client)).await {
                Ok(server) => {
                    info!("uTP server created on port {}", torrent_port);
                    Some(Arc::new(RwLock::new(server)))
                }
                Err(e) => {
                    warn!("Failed to create uTP server: {}. Continuing without uTP.", e);
                    None
                }
            }
        } else {
            None
        };

        let destroyed = Arc::new(RwLock::new(false));
        let destroyed_clone = Arc::clone(&destroyed);

        // Start accepting connections
        let handle = if let Some(listener) = tcp_server {
            let client_clone = Arc::clone(&client);
            Some(tokio::spawn(async move {
                loop {
                    // Check if destroyed
                    if *destroyed_clone.read().await {
                        break;
                    }

                    // Accept incoming connections
                    match listener.accept().await {
                        Ok((stream, addr)) => {
                            debug!("Incoming TCP connection from {}", addr);
                            let client_task = Arc::clone(&client_clone);
                            // Spawn task - the future must be Send
                            // We ensure this by only using Send-safe operations
                            let handle_result = tokio::task::spawn(async move {
                                if let Err(e) = Self::handle_incoming_connection(client_task, stream, addr).await {
                                    error!("Error handling incoming connection from {}: {}", addr, e);
                                }
                            });
                            // Note: We don't await the handle here to avoid blocking
                            // The task will run independently
                            drop(handle_result);
                        }
                        Err(e) => {
                            if *destroyed_clone.read().await {
                                break;
                            }
                            error!("Error accepting TCP connection: {}", e);
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                    }
                }
            }))
        } else {
            None
        };

        // Note: tcp_server is moved into the task, so we set it to None here
        // In a production implementation, we might use Arc<Mutex<TcpListener>> to share it
        Ok(Self {
            client,
            tcp_server: None, // Moved into the task
            utp_server,
            destroyed,
            handle,
        })
    }


    async fn handle_incoming_connection(
        client: Arc<WebTorrent>,
        mut stream: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
    ) -> Result<()> {
        // Read BitTorrent handshake
        let mut handshake_buf = [0u8; 68]; // 20 + 8 + 20 + 20
        stream.read_exact(&mut handshake_buf).await.map_err(|e| {
            eprintln!("[DEBUG] Failed to read incoming handshake from {}: {}", addr, e);
            crate::error::WebTorrentError::Network(format!("Failed to read handshake: {}", e))
        })?;

        // Parse handshake
        let handshake = Handshake::decode(&handshake_buf)?;
        let info_hash = handshake.info_hash;
        let peer_id = hex::encode(handshake.peer_id);
        let our_peer_id = hex::encode(client.peer_id);

        eprintln!("[DEBUG] Incoming connection for info hash: {} from {} (peer_id: {}, our_peer_id: {})", 
            hex::encode(info_hash), addr, peer_id, our_peer_id);

        // Check blocklist
        if client.blocklist.is_blocked(addr.ip()).await {
            debug!("Blocked incoming connection from {} (blocklisted)", addr);
            return Ok(()); // Silently reject blocked IPs
        }

        // Gracefully skip accepting connections from the same client instance (same peer_id)
        // But allow connections from different clients on the same machine
        if peer_id == our_peer_id {
            // Silently close - this is our own client instance
            return Ok(());
        }

        // Find the torrent
        let torrent = if let Some(t) = client.get(&info_hash).await {
            t
        } else {
            warn!("No torrent found for info hash: {} from {}", hex::encode(info_hash), addr);
            return Ok(()); // Close connection if torrent not found
        };

        // Create peer and wire
        let peer_id = handshake.peer_id;
        let peer_id_str = hex::encode(peer_id);
        
        // Check if peer already exists
        let peer_exists = {
            let peers = torrent.peers.read().await;
            peers.contains_key(&peer_id_str)
        };

        if peer_exists {
            debug!("Peer {} already exists for torrent {}", peer_id_str, hex::encode(info_hash));
            return Ok(()); // Close connection if peer already exists
        }

        // Send our handshake
        let our_handshake = Handshake::new(
            info_hash,
            client.peer_id,
        );
        let handshake_bytes = our_handshake.encode();
        stream.write_all(&handshake_bytes).await.map_err(|e| {
            crate::error::WebTorrentError::Network(format!("Failed to send handshake: {}", e))
        })?;

        // Create wire for this connection
        let wire = Arc::new(crate::wire::Wire::new("tcp".to_string()));
        
        // Create peer with address
        let peer_addr = format!("{}:{}", addr.ip(), addr.port());
        let peer = Arc::new(crate::peer::Peer::new(
            peer_id_str.clone(),
            crate::peer::PeerType::TcpIncoming,
        ).with_addr(peer_addr.clone()));
        
        // Add peer and wire to torrent
        {
            let mut peers = torrent.peers.write().await;
            peers.insert(peer_id_str.clone(), Arc::clone(&peer));
        }
        
        {
            let mut wires = torrent.wires.write().await;
            wires.push(Arc::clone(&wire));
        }

        info!("Accepted incoming connection from {} for torrent {}", addr, hex::encode(info_hash));

        // Set up message channel for immediate PEX sending
        let (message_tx, mut message_rx) = mpsc::unbounded_channel();
        wire.set_message_sender(message_tx).await;
        
        // Split stream into reader and writer for concurrent access
        let (reader, mut writer) = tokio::io::split(stream);
        
        // Spawn task to handle outgoing messages from wire (for immediate PEX sending)
        let writer_addr = addr;
        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                if let Err(e) = writer.write_all(&message).await {
                    debug!("Failed to send message to {}: {}", writer_addr, e);
                    break;
                }
            }
        });
        
        // Continue with BitTorrent protocol handling
        // Spawn a task to handle the connection
        let wire_clone = Arc::clone(&wire);
        let torrent_clone = Arc::clone(&torrent);
        let client_clone = Arc::clone(&client);
        
        tokio::task::spawn(async move {
            if let Err(e) = Self::handle_peer_connection(
                client_clone,
                torrent_clone,
                wire_clone,
                reader,
                addr,
            ).await {
                error!("Error handling peer connection from {}: {}", addr, e);
            }
        });

        Ok(())
    }

    /// Handle ongoing peer connection - reads and processes BitTorrent messages
    pub(crate) async fn handle_peer_connection(
        client: Arc<WebTorrent>,
        torrent: Arc<crate::torrent::Torrent>,
        wire: Arc<crate::wire::Wire>,
        mut reader: tokio::io::ReadHalf<tokio::net::TcpStream>,
        addr: std::net::SocketAddr,
    ) -> Result<()> {
        use bytes::BytesMut;
        use std::io;

        // Enable PEX if enabled and torrent is not private
        if client.options.ut_pex && !torrent.is_private() {
            wire.enable_pex().await;
        }

        // Get writer for sending messages
        // Note: We can't easily get the writer here since it's in another task
        // Initial messages (bitfield, unchoke, interested) can be sent through the message channel
        // PEX messages are sent immediately through the message channel when peers change
        
        // Read messages in a loop
        let mut reader = tokio::io::BufReader::new(reader);
        let mut buffer = BytesMut::with_capacity(1024 * 64); // 64KB buffer

        loop {
            // Check if wire is destroyed
            if wire.destroyed().await {
                break;
            }

            // Read message length (4 bytes)
            let mut length_buf = [0u8; 4];
            match reader.read_exact(&mut length_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    debug!("Connection closed by peer {}", addr);
                    break;
                }
                Err(e) => {
                    return Err(crate::error::WebTorrentError::Network(
                        format!("Failed to read message length: {}", e)
                    ));
                }
            }

            let message_length = u32::from_be_bytes(length_buf) as usize;

            if message_length == 0 {
                // Keep-alive message
                continue;
            }

            if message_length > 1024 * 1024 {
                // Message too large (max 1MB)
                return Err(crate::error::WebTorrentError::Protocol(
                    format!("Message too large: {} bytes", message_length)
                ));
            }

            // Read message payload
            buffer.resize(message_length, 0);
            reader.read_exact(&mut buffer).await.map_err(|e| {
                crate::error::WebTorrentError::Network(format!("Failed to read message: {}", e))
            })?;

            // Parse and handle message using unified handler
            if buffer.is_empty() {
                continue;
            }

            // Use unified message handler
            Self::handle_bittorrent_message(
                client.clone(),
                torrent.clone(),
                wire.clone(),
                addr,
                &buffer,
            ).await?;
        }

        // Clean up
        wire.destroy().await?;
        Ok(())
    }

    /// Send a BitTorrent message over TCP
    async fn send_message(
        stream: &mut tokio::net::TcpStream,
        message_type: crate::protocol::MessageType,
        payload: Option<Vec<u8>>,
    ) -> Result<()> {
        let mut buffer = BytesMut::with_capacity(5);
        let payload_len = payload.as_ref().map(|p| p.len()).unwrap_or(0);
        buffer.put_u32((1 + payload_len) as u32); // Message length
        buffer.put_u8(message_type as u8); // Message ID

        if let Some(payload) = payload {
            buffer.put_slice(&payload);
        }

        stream.write_all(&buffer).await.map_err(|e| {
            crate::error::WebTorrentError::Network(format!("Failed to send message: {}", e))
        })?;

        Ok(())
    }

    /// Send bitfield message over TCP - uses unified bitfield conversion
    async fn send_bitfield(
        stream: &mut tokio::net::TcpStream,
        bitfield: &bitvec::prelude::BitVec,
    ) -> Result<()> {
        let bitfield_bytes = Self::bitfield_to_bytes(bitfield);
        let mut buffer = BytesMut::with_capacity(5 + bitfield_bytes.len());
        buffer.put_u32((1 + bitfield_bytes.len()) as u32); // Message length
        buffer.put_u8(5); // Bitfield message ID
        buffer.put_slice(&bitfield_bytes);

        stream.write_all(&buffer).await.map_err(|e| {
            crate::error::WebTorrentError::Network(format!("Failed to send bitfield: {}", e))
        })?;

        Ok(())
    }

    /// Unified BitTorrent message handler - works for both TCP and uTP
    /// This is the single source of truth for message handling logic
    async fn handle_bittorrent_message(
        client: Arc<WebTorrent>,
        torrent: Arc<crate::torrent::Torrent>,
        wire: Arc<crate::wire::Wire>,
        addr: std::net::SocketAddr,
        message_data: &[u8],
    ) -> Result<()> {
        if message_data.is_empty() {
            return Ok(());
        }
        
        let message_id = message_data[0];
        let payload = if message_data.len() > 1 {
            Some(message_data[1..].to_vec())
        } else {
            None
        };
        
        // Unified message handling for all transport types
        match message_id {
            0 => {
                // Choke
                wire.set_peer_choking(true).await;
                debug!("Peer {} choked", addr);
            }
            1 => {
                // Unchoke
                wire.set_peer_choking(false).await;
                debug!("Peer {} unchoked", addr);
            }
            2 => {
                // Interested
                wire.set_peer_interested(true).await;
                debug!("Peer {} interested", addr);
            }
            3 => {
                // Not interested
                wire.set_peer_interested(false).await;
                debug!("Peer {} not interested", addr);
            }
            4 => {
                // Have
                if let Some(payload) = payload {
                    if payload.len() == 4 {
                        let piece_index = u32::from_be_bytes([
                            payload[0], payload[1], payload[2], payload[3]
                        ]) as usize;
                        let mut peer_pieces = wire.peer_pieces().await;
                        if piece_index < peer_pieces.len() {
                            peer_pieces.set(piece_index, true);
                            wire.set_peer_pieces(peer_pieces).await;
                        }
                        debug!("Peer {} has piece {}", addr, piece_index);
                    }
                }
            }
            5 => {
                // Bitfield
                if let Some(payload) = &payload {
                    let payload_len = payload.len();
                    let bitfield = Self::bitfield_from_bytes(payload.clone());
                    wire.set_peer_pieces(bitfield).await;
                    debug!("Peer {} sent bitfield ({} bytes)", addr, payload_len);
                }
            }
            6 => {
                // Request
                if let Some(payload) = payload {
                    if payload.len() == 12 {
                        let piece_index = u32::from_be_bytes([
                            payload[0], payload[1], payload[2], payload[3]
                        ]) as usize;
                        let offset = u32::from_be_bytes([
                            payload[4], payload[5], payload[6], payload[7]
                        ]) as usize;
                        let length = u32::from_be_bytes([
                            payload[8], payload[9], payload[10], payload[11]
                        ]) as usize;
                        wire.request(piece_index, offset, length).await;
                        debug!("Peer {} requested piece {} offset {} length {}", 
                            addr, piece_index, offset, length);
                    }
                }
            }
            7 => {
                // Piece
                if let Some(payload) = payload {
                    if payload.len() >= 8 {
                        let piece_index = u32::from_be_bytes([
                            payload[0], payload[1], payload[2], payload[3]
                        ]) as usize;
                        let block_offset = u32::from_be_bytes([
                            payload[4], payload[5], payload[6], payload[7]
                        ]) as usize;
                        let block_data = &payload[8..];
                        
                        debug!("Peer {} sent piece {} block offset {} length {}", 
                            addr, piece_index, block_offset, block_data.len());
                        
                        // Record download
                        client.record_download(block_data.len() as u64).await;
                    }
                }
            }
            8 => {
                // Cancel
                if let Some(payload) = payload {
                    if payload.len() == 12 {
                        let piece_index = u32::from_be_bytes([
                            payload[0], payload[1], payload[2], payload[3]
                        ]) as usize;
                        let offset = u32::from_be_bytes([
                            payload[4], payload[5], payload[6], payload[7]
                        ]) as usize;
                        let length = u32::from_be_bytes([
                            payload[8], payload[9], payload[10], payload[11]
                        ]) as usize;
                        debug!("Peer {} cancelled piece {} offset {} length {}", 
                            addr, piece_index, offset, length);
                    }
                }
            }
            20 => {
                // Extended message (extension protocol)
                if let Some(payload) = payload {
                    if payload.len() >= 2 {
                        let ext_id = payload[0];
                        let ext_data = &payload[1..];
                        
                        // Handle PEX extension (typically ID 1)
                        if ext_id == 1 {
                            // PEX message
                            if let Ok((added, dropped)) = crate::extensions::UtPex::decode_full(ext_data) {
                                debug!("PEX message from {}: {} added, {} dropped", addr, added.len(), dropped.len());
                                
                                // Handle added peers - add to discovery
                                for (ip, port) in added {
                                    // Add peer to discovery
                                    if let Some(discovery) = torrent.discovery.read().await.as_ref() {
                                        discovery.add_pex_peer(ip.clone(), port).await;
                                    }
                                    debug!("PEX: Added peer {}:{} from {}", ip, port, addr);
                                }
                                
                                // Handle dropped peers
                                for (ip, port) in dropped {
                                    let peer_addr = format!("{}:{}", ip, port);
                                    // Remove peer if exists and not connected
                                    let peer_opt = {
                                        let peers = torrent.peers.read().await;
                                        peers.get(&peer_addr).cloned()
                                    };
                                    
                                    if let Some(peer) = peer_opt {
                                        // Check if connected
                                        if !peer.connected().await {
                                            // Remove peer
                                            let mut peers = torrent.peers.write().await;
                                            peers.remove(&peer_addr);
                                            debug!("PEX: Dropped peer {}:{} from {}", ip, port, addr);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                debug!("Unknown message type {} from peer {}", message_id, addr);
            }
        }
        
        Ok(())
    }
    
    /// Convert bitfield bytes to BitVec - unified helper for all transports
    fn bitfield_from_bytes(bytes: Vec<u8>) -> bitvec::prelude::BitVec {
        let mut bitfield = bitvec::prelude::BitVec::new();
        for byte in bytes {
            for i in 0..8 {
                bitfield.push((byte & (1 << (7 - i))) != 0);
            }
        }
        bitfield
    }
    
    /// Convert BitVec to bytes - unified helper for all transports
    fn bitfield_to_bytes(bitfield: &bitvec::prelude::BitVec) -> Vec<u8> {
        // Manual conversion - ensures correct bit ordering
        let num_bits = bitfield.len();
        let num_bytes = (num_bits + 7) / 8;
        let mut bytes = Vec::with_capacity(num_bytes);
        for i in 0..num_bytes {
            let mut byte = 0u8;
            for j in 0..8 {
                let bit_index = i * 8 + j;
                if bit_index < num_bits && bitfield[bit_index] {
                    byte |= 1 << (7 - j);
                }
            }
            bytes.push(byte);
        }
        bytes
    }

    pub async fn destroy(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Ok(());
        }

        *self.destroyed.write().await = true;

        // Wait for accept loop to finish
        // Note: handle is behind &self, so we can't take it
        // The task will stop when destroyed is set to true
        // In production, we might use Arc<Mutex<JoinHandle>> or similar

        Ok(())
    }
}

