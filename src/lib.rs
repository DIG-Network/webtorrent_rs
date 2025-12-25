/*! webtorrent-rs. MIT License. WebTorrent Rust Port */

pub mod client;
pub mod torrent;
pub mod peer;
pub mod conn_pool;
pub mod server;
pub mod discovery;
pub mod piece;
pub mod file;
pub mod store;
pub mod nat;
pub mod error;
pub mod protocol;
pub mod wire;
pub mod selections;
pub mod rarity_map;
pub mod bencode_parser;
pub mod magnet;
pub mod tracker;
pub mod dht;
pub mod throttling;
pub mod extensions;
pub mod torrent_creator;

// Re-export for easier use
pub use bencode_parser::{parse_bencode, BencodeValue};
pub use magnet::MagnetUri;
pub use throttling::ThrottleGroup;
pub use extensions::{UtMetadata, UtPex, ExtensionProtocol};

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

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

