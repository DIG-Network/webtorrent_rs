use thiserror::Error;

pub type Result<T> = std::result::Result<T, WebTorrentError>;

#[derive(Error, Debug)]
pub enum WebTorrentError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("Bencode error: {0}")]
    Bencode(String),

    #[error("Invalid torrent: {0}")]
    InvalidTorrent(String),

    #[error("Invalid info hash: {0}")]
    InvalidInfoHash(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Peer error: {0}")]
    Peer(String),

    #[error("Store error: {0}")]
    Store(String),

    #[error("NAT traversal error: {0}")]
    Nat(String),

    #[error("Discovery error: {0}")]
    Discovery(String),

    #[error("Torrent destroyed")]
    TorrentDestroyed,

    #[error("Client destroyed")]
    ClientDestroyed,

    #[error("Invalid peer address: {0}")]
    InvalidPeerAddress(String),

    #[error("Connection timeout")]
    ConnectionTimeout,

    #[error("Handshake timeout")]
    HandshakeTimeout,

    #[error("Invalid piece index: {0}")]
    InvalidPieceIndex(usize),

    #[error("Invalid block request")]
    InvalidBlockRequest,

    #[error("Piece verification failed")]
    PieceVerificationFailed,

    #[error("Duplicate torrent: {0}")]
    DuplicateTorrent(String),
}

impl From<std::io::Error> for WebTorrentError {
    fn from(err: std::io::Error) -> Self {
        WebTorrentError::Io(err.to_string())
    }
}

// Note: We use our own bencode_parser, so this is not needed
// If we switch to a bencode crate, uncomment this:
// impl From<bencode::Error> for WebTorrentError {
//     fn from(err: bencode::Error) -> Self {
//         WebTorrentError::Bencode(err.to_string())
//     }
// }

impl From<reqwest::Error> for WebTorrentError {
    fn from(err: reqwest::Error) -> Self {
        WebTorrentError::Network(err.to_string())
    }
}

impl From<url::ParseError> for WebTorrentError {
    fn from(err: url::ParseError) -> Self {
        WebTorrentError::InvalidTorrent(err.to_string())
    }
}

