# WebTorrent Rust

A Rust port of [WebTorrent](https://github.com/webtorrent/webtorrent) - a streaming torrent client for the web.

## ⚠️ Important Disclaimer

**This is an unofficial, vibe-coded port of WebTorrent to Rust. This project is:**
- **NOT for production use** - Do not use in production environments
- **NOT actively maintained** - This is an experimental port
- **For testing purposes only** - Use at your own risk
- **No guarantees** - No warranties, no support, use at your own risk

This port was created for experimentation and learning purposes. If you need a production-ready solution, please use the official [WebTorrent](https://github.com/webtorrent/webtorrent) JavaScript implementation.

---

## Table of Contents

1. [Key Crate Concepts](#key-crate-concepts)
2. [Crate Usage](#crate-usage)
3. [Exhaustive Reference Guide](#exhaustive-reference-guide)
4. [Architecture](#architecture)
5. [Testing](#testing)
6. [License](#license)

---

## Key Crate Concepts

### Core Architecture

WebTorrent Rust follows a modular architecture that mirrors the JavaScript WebTorrent library:

- **Client (`WebTorrent`)**: The main entry point that manages torrents, connections, and global state
- **Torrent (`Torrent`)**: Represents a single torrent with its metadata, pieces, files, and peer connections
- **Peer (`Peer`)**: Represents a connected peer (TCP, uTP, or WebRTC)
- **Wire (`Wire`)**: Manages the BitTorrent protocol communication with a peer
- **Discovery (`Discovery`)**: Orchestrates peer discovery via DHT, trackers, LSD, and PEX
- **Connection Pool (`ConnPool`)**: Manages incoming TCP and uTP connections

### BitTorrent Protocol Flow

1. **Torrent Addition**: Client adds a torrent via magnet URI, info hash, or torrent file
2. **Metadata Parsing**: Torrent metadata is parsed to extract info hash, files, pieces, and trackers
3. **Peer Discovery**: Discovery module announces to trackers and queries DHT to find peers
4. **Connection Establishment**: Client connects to discovered peers via TCP or uTP
5. **Handshake**: BitTorrent handshake is exchanged to verify info hash and exchange peer IDs
6. **Bitfield Exchange**: Peers exchange bitfields showing which pieces they have
7. **Piece Download**: Client requests pieces from peers using rarest-first or sequential strategy
8. **Piece Verification**: Downloaded pieces are verified against SHA-1 hashes
9. **Data Storage**: Verified pieces are stored in the chunk store (memory or filesystem)

### Peer Discovery Mechanisms

- **Trackers**: HTTP/HTTPS trackers that return lists of peers
- **DHT (Distributed Hash Table)**: Decentralized peer discovery using libp2p Kademlia
- **LSD (Local Service Discovery)**: Discovers peers on the local network via mDNS
- **PEX (Peer Exchange)**: Peers exchange peer lists with each other

### NAT Traversal

Automatic port mapping via:
- **UPnP**: Universal Plug and Play for automatic router port forwarding
- **NAT-PMP**: NAT Port Mapping Protocol (Apple routers)

### Transport Protocols

- **TCP**: Standard BitTorrent TCP connections
- **uTP (Micro Transport Protocol)**: UDP-based transport for better congestion control
- **WebRTC**: Planned for browser compatibility (not yet implemented)

### Piece Selection Strategies

- **Sequential**: Download pieces in order (good for streaming)
- **Rarest First**: Download rarest pieces first (good for swarm health)

---

## Crate Usage

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
webtorrent = { path = "../webtorrent_rs" }
tokio = { version = "1", features = ["full"] }
```

### Basic Example: Downloading a Torrent

```rust
use webtorrent::{WebTorrent, WebTorrentOptions};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with default options
    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    
    // Add a torrent via magnet URI
    let torrent = client.add("magnet:?xt=urn:btih:...").await?;
    
    // Wait for download to complete
    loop {
        let progress = torrent.progress().await;
        println!("Progress: {:.2}%", progress * 100.0);
        
        if progress >= 1.0 {
            println!("Download complete!");
            break;
        }
        
        sleep(Duration::from_secs(1)).await;
    }
    
    // Clean up
    client.destroy().await?;
    Ok(())
}
```

### Seeding Files

```rust
use webtorrent::{WebTorrent, WebTorrentOptions, TorrentCreator};
use bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let options = WebTorrentOptions {
        torrent_port: 6881,
        dht_port: 0,
        ..Default::default()
    };
    let client = WebTorrent::new(options).await?;
    
    // Create torrent from data
    let data = Bytes::from("Hello, World!");
    let creator = TorrentCreator::new()
        .with_announce(vec!["http://tracker.example.com/announce".to_string()]);
    
    let (torrent_file, _info_hash) = creator.create_from_data(
        "hello.txt".to_string(),
        data.clone()
    ).await?;
    
    // Seed the torrent
    let torrent = client.add(torrent_file).await?;
    
    println!("Seeding torrent: {}", torrent.name());
    println!("Info hash: {}", hex::encode(torrent.info_hash()));
    
    // Keep seeding...
    tokio::signal::ctrl_c().await?;
    
    client.destroy().await?;
    Ok(())
}
```

### Custom Client Options

```rust
use webtorrent::{WebTorrent, WebTorrentOptions};

let options = WebTorrentOptions {
    // Custom peer ID (20 bytes)
    peer_id: Some([0u8; 20]),
    
    // Listening ports
    torrent_port: 6881,  // BitTorrent port
    dht_port: 6882,      // DHT port
    
    // Connection limits
    max_conns: 55,       // Maximum concurrent connections
    
    // Transport protocols
    utp: true,            // Enable uTP
    
    // NAT traversal
    nat_upnp: true,      // Enable UPnP
    nat_pmp: true,       // Enable NAT-PMP
    
    // Peer discovery
    lsd: true,           // Local Service Discovery
    ut_pex: true,        // Peer Exchange
    
    // Rate limiting (bytes per second)
    download_limit: Some(1024 * 1024),  // 1 MB/s
    upload_limit: Some(512 * 1024),     // 512 KB/s
    
    // Custom tracker configuration
    tracker: Some(webtorrent::TrackerConfig {
        announce: vec!["http://tracker.example.com/announce".to_string()],
        get_announce_opts: None,
    }),
    
    ..Default::default()
};

let client = WebTorrent::new(options).await?;
```

### Monitoring Download Progress

```rust
use webtorrent::WebTorrent;
use tokio::time::{interval, Duration};

async fn monitor_torrent(torrent: Arc<webtorrent::Torrent>) {
    let mut interval = interval(Duration::from_secs(1));
    
    loop {
        interval.tick().await;
        
        let progress = torrent.progress().await;
        let downloaded = torrent.downloaded().await;
        let length = torrent.length().await;
        let uploaded = torrent.uploaded().await;
        let num_peers = torrent.num_peers().await;
        
        println!("Progress: {:.2}%", progress * 100.0);
        println!("Downloaded: {} / {} bytes", downloaded, length);
        println!("Uploaded: {} bytes", uploaded);
        println!("Peers: {}", num_peers);
        
        if progress >= 1.0 {
            break;
        }
    }
}
```

### Speed Throttling

```rust
// Set download limit to 1 MB/s
client.throttle_download(Some(1024 * 1024)).await;

// Set upload limit to 512 KB/s
client.throttle_upload(Some(512 * 1024)).await;

// Remove limits
client.throttle_download(None).await;
client.throttle_upload(None).await;

// Get current speeds
let download_speed = client.download_speed().await;
let upload_speed = client.upload_speed().await;
```

### Working with Files

```rust
// Get all files in a torrent
let files = torrent.files();
for file in files {
    println!("File: {} ({} bytes)", file.name(), file.length());
}

// Access file data (once downloaded)
// Note: File data access depends on the chunk store implementation
```

### Error Handling

```rust
use webtorrent::{WebTorrent, WebTorrentError};

match client.add("magnet:?xt=urn:btih:...").await {
    Ok(torrent) => {
        println!("Torrent added: {}", torrent.name());
    }
    Err(WebTorrentError::InvalidTorrent(msg)) => {
        eprintln!("Invalid torrent: {}", msg);
    }
    Err(WebTorrentError::DuplicateTorrent(hash)) => {
        eprintln!("Torrent already exists: {}", hash);
    }
    Err(WebTorrentError::Network(msg)) => {
        eprintln!("Network error: {}", msg);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

---

## Exhaustive Reference Guide

### Core Types

#### `WebTorrent`

The main client that manages torrents and connections.

**Methods:**

- `new(options: WebTorrentOptions) -> Result<Self>`: Create a new WebTorrent client
- `add(torrent_id: impl Into<TorrentId>) -> Result<Arc<Torrent>>`: Add a torrent to the client
- `seed(name: String, data: Bytes, announce: Option<Vec<String>>) -> Result<Arc<Torrent>>`: Seed data as a torrent
- `remove(torrent: Arc<Torrent>) -> Result<()>`: Remove a torrent from the client
- `get(info_hash: &[u8; 20]) -> Option<Arc<Torrent>>`: Get a torrent by info hash
- `destroy() -> Result<()>`: Destroy the client and all torrents
- `progress() -> f64`: Get overall download progress (0.0 to 1.0)
- `ratio() -> f64`: Get overall upload/download ratio
- `download_speed() -> u64`: Get current download speed in bytes per second
- `upload_speed() -> u64`: Get current upload speed in bytes per second
- `throttle_download(rate: Option<u64>)`: Set download throttle rate (bytes per second, None = unlimited)
- `throttle_upload(rate: Option<u64>)`: Set upload throttle rate (bytes per second, None = unlimited)
- `address() -> Option<(String, u16)>`: Get listening address (IP, port)
- `peer_id() -> [u8; 20]`: Get the client's peer ID

#### `WebTorrentOptions`

Configuration options for the WebTorrent client.

**Fields:**

- `peer_id: Option<[u8; 20]>`: Custom peer ID (20 bytes)
- `node_id: Option<[u8; 20]>`: Custom DHT node ID (20 bytes)
- `torrent_port: u16`: Port for BitTorrent connections (0 = auto)
- `dht_port: u16`: Port for DHT (0 = disabled)
- `max_conns: usize`: Maximum concurrent peer connections (default: 55)
- `utp: bool`: Enable uTP transport (default: true)
- `nat_upnp: bool`: Enable UPnP NAT traversal (default: true)
- `nat_pmp: bool`: Enable NAT-PMP NAT traversal (default: true)
- `lsd: bool`: Enable Local Service Discovery (default: true)
- `ut_pex: bool`: Enable Peer Exchange extension (default: true)
- `seed_outgoing_connections: bool`: Seed on outgoing connections (default: true)
- `download_limit: Option<u64>`: Download rate limit in bytes per second (None = unlimited)
- `upload_limit: Option<u64>`: Upload rate limit in bytes per second (None = unlimited)
- `blocklist: Option<String>`: Path to blocklist file
- `tracker: Option<TrackerConfig>`: Custom tracker configuration
- `web_seeds: bool`: Enable web seed support (default: true)

#### `Torrent`

Represents a single torrent with its metadata and peer connections.

**Methods:**

- `info_hash() -> [u8; 20]`: Get the torrent's info hash
- `name() -> String`: Get the torrent name
- `length() -> u64`: Get total torrent length in bytes
- `files() -> Vec<File>`: Get list of files in the torrent
- `progress() -> f64`: Get download progress (0.0 to 1.0)
- `downloaded() -> u64`: Get number of bytes downloaded
- `uploaded() -> u64`: Get number of bytes uploaded
- `received() -> u64`: Get number of bytes received (includes duplicates)
- `num_peers() -> usize`: Get number of connected peers
- `destroy() -> Result<()>`: Destroy the torrent and disconnect from peers
- `get_bitfield() -> BitVec`: Get the torrent's bitfield (pieces we have)

#### `TorrentId`

Represents different ways to identify a torrent.

**Variants:**

- `InfoHash([u8; 20])`: Info hash only (requires metadata fetching)
- `MagnetUri(String)`: Magnet URI (e.g., `"magnet:?xt=urn:btih:..."`)
- `TorrentFile(Bytes)`: Torrent file data (bencoded)
- `Url(String)`: URL to fetch torrent file from

**From implementations:**

- `From<[u8; 20]>`: Info hash
- `From<String>`: Automatically detects magnet URI or URL
- `From<&str>`: Same as `String`
- `From<Bytes>`: Torrent file data
- `From<Vec<u8>>`: Torrent file data

#### `File`

Represents a single file within a torrent.

**Methods:**

- `name() -> String`: Get the file name
- `length() -> u64`: Get the file length in bytes
- `offset() -> u64`: Get the file offset within the torrent

#### `TorrentCreator`

Creates torrent files from data.

**Methods:**

- `new() -> Self`: Create a new TorrentCreator
- `with_announce(announce: Vec<String>) -> Self`: Set tracker announce URLs
- `with_comment(comment: String) -> Self`: Set torrent comment
- `with_created_by(created_by: String) -> Self`: Set created by field
- `with_piece_length(piece_length: u64) -> Self`: Set piece length (default: 256 KB)
- `create_from_data(name: String, data: Bytes) -> Result<(Bytes, [u8; 20])>`: Create torrent from data, returns (torrent file, info hash)

### Error Types

#### `WebTorrentError`

Comprehensive error type for all WebTorrent operations.

**Variants:**

- `Io(String)`: I/O error
- `Bencode(String)`: Bencode parsing error
- `InvalidTorrent(String)`: Invalid torrent file or metadata
- `InvalidInfoHash(String)`: Invalid info hash format
- `Network(String)`: Network error (connection, timeout, etc.)
- `Protocol(String)`: BitTorrent protocol error
- `Peer(String)`: Peer-related error
- `Store(String)`: Storage error
- `Nat(String)`: NAT traversal error
- `Discovery(String)`: Peer discovery error
- `TorrentDestroyed`: Torrent has been destroyed
- `ClientDestroyed`: Client has been destroyed
- `InvalidPeerAddress(String)`: Invalid peer address format
- `ConnectionTimeout`: Connection timeout
- `HandshakeTimeout`: Handshake timeout
- `InvalidPieceIndex(usize)`: Invalid piece index
- `InvalidBlockRequest`: Invalid block request
- `PieceVerificationFailed`: Piece hash verification failed
- `DuplicateTorrent(String)`: Torrent already exists (info hash)

### Utility Types

#### `BencodeValue`

Represents a bencoded value (integer, string, list, or dictionary).

**Methods:**

- `as_integer() -> Option<i64>`: Get as integer
- `as_string() -> Option<String>`: Get as string
- `as_bytes() -> Option<&[u8]>`: Get as bytes
- `as_list() -> Option<&[BencodeValue]>`: Get as list
- `as_dict() -> Option<&HashMap<Vec<u8>, BencodeValue>>`: Get as dictionary
- `get(key: &[u8]) -> Option<&BencodeValue>`: Get value from dictionary by key
- `encode() -> Vec<u8>`: Encode to bencoded bytes

#### `MagnetUri`

Parsed magnet URI.

**Methods:**

- `parse(uri: &str) -> Result<Self>`: Parse a magnet URI
- `info_hash() -> [u8; 20]`: Get info hash
- `display_name() -> Option<String>`: Get display name (dn parameter)
- `trackers() -> Vec<String>`: Get tracker URLs (tr parameters)
- `exact_length() -> Option<u64>`: Get exact length (xl parameter)

#### `ThrottleGroup`

Token bucket rate limiter for download/upload throttling.

**Methods:**

- `new(rate: u64, enabled: bool) -> Self`: Create a new throttle group
- `set_rate(rate: u64)`: Set rate limit in bytes per second
- `set_enabled(enabled: bool)`: Enable or disable throttling
- `acquire(bytes: u64) -> impl Future<Output = ()>`: Acquire tokens (async)

#### `ExtensionProtocol`

BitTorrent protocol extensions.

**Types:**

- `UtMetadata`: ut_metadata extension for metadata exchange
- `UtPex`: ut_pex extension for peer exchange

### Module Exports

The crate re-exports the following for convenience:

```rust
pub use client::{WebTorrent, WebTorrentOptions, TorrentId};
pub use torrent::Torrent;
pub use error::{WebTorrentError, Result};
pub use selections::{Selections, Selection};
pub use rarity_map::RarityMap;
pub use piece::Piece;
pub use file::File;
pub use store::{ChunkStore, MemoryChunkStore};
pub use protocol::{Handshake, MessageType};
pub use torrent_creator::TorrentCreator;
pub use nat::NatTraversal;
pub use bencode_parser::{parse_bencode, BencodeValue};
pub use magnet::MagnetUri;
pub use throttling::ThrottleGroup;
pub use extensions::{UtMetadata, UtPex, ExtensionProtocol};
```

### Constants

- `VERSION: &str`: Crate version string

---

## Architecture

The library is organized into the following modules:

- **`client`**: Main WebTorrent client implementation
- **`torrent`**: Torrent management and metadata handling
- **`peer`**: Peer connection management (TCP, uTP, WebRTC)
- **`wire`**: BitTorrent protocol wire implementation
- **`discovery`**: Peer discovery via DHT, trackers, LSD, and PEX
- **`piece`**: Piece management and verification
- **`file`**: File handling within torrents
- **`store`**: Chunk storage backends (memory, filesystem)
- **`nat`**: NAT traversal (UPnP, NAT-PMP)
- **`conn_pool`**: Connection pool for managing incoming connections
- **`server`**: HTTP server for serving torrent files
- **`protocol`**: BitTorrent protocol message handling
- **`bencode_parser`**: Bencode encoding/decoding
- **`magnet`**: Magnet URI parsing
- **`tracker`**: Tracker client implementation
- **`dht`**: DHT implementation using libp2p
- **`throttling`**: Rate limiting and throttling
- **`extensions`**: Protocol extensions (ut_metadata, ut_pex)
- **`torrent_creator`**: Torrent file creation
- **`selections`**: Piece selection strategies
- **`rarity_map`**: Rarest-first piece selection

### NAT Traversal

The library supports automatic NAT traversal via:

- **UPnP**: Universal Plug and Play port mapping
- **NAT-PMP**: NAT Port Mapping Protocol

Both methods are enabled by default and will automatically map ports when the client starts listening.

### Dependencies

- **tokio**: Async runtime
- **libp2p**: Peer-to-peer networking and DHT
- **igd**: UPnP port mapping
- **natpmp**: NAT-PMP port mapping
- **bencode_parser**: Custom bencode format parsing
- **reqwest**: HTTP client for web seeds and torrent file fetching
- **bitvec**: Bitfield manipulation
- **sha1**: SHA-1 hashing for piece verification
- **hex**: Hex encoding/decoding
- **bytes**: Efficient byte buffer handling

---

## Status

This is an **unofficial, experimental, vibe-coded port** of WebTorrent to Rust. The core API is being implemented to match the JavaScript version as closely as possible, but this project is **not actively maintained** and should **not be used in production**. Use at your own risk, for testing purposes only.

### Implementation Status

All core components are implemented, including:

- Complete bencode parsing
- Magnet URI support
- NAT traversal (UPnP + NAT-PMP)
- Tracker client
- DHT integration
- Protocol extensions (ut_metadata, ut_pex)
- Throttling support
- Comprehensive error handling
- **Comprehensive test suite** (80+ tests covering all components)

See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for detailed status information.
See [TEST_COVERAGE.md](TEST_COVERAGE.md) for test coverage details.
See [TRACKER_INTEGRATION.md](TRACKER_INTEGRATION.md) for dig-relay tracker integration guide.

---

## Testing

The library includes a comprehensive test suite with 80+ tests:

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test test_bencode
cargo test --test test_client
cargo test --test test_torrent
cargo test --test test_tracker_integration

# Run with output
cargo test -- --nocapture
```

### Examples

```bash
# Test tracker connection
cargo run --example test_tracker_connection

# Quick seeder test
cargo run --example test_seeder_quick

# Seed and download test
cargo run --example test_seed_and_download

# Seed to dig-relay tracker
cargo run --example seed_to_dig_relay
```

See [examples/README.md](examples/README.md) for more information.

---

## License

MIT
