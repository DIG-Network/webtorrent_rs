use webtorrent::{WebTorrent, WebTorrentOptions};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let options = WebTorrentOptions {
        torrent_port: 6881,
        dht_port: 6882,
        ..Default::default()
    };

    let client = WebTorrent::new(options).await?;

    println!("WebTorrent client started");
    println!("Use the library API to add torrents and manage downloads");

    // Keep the client alive
    tokio::signal::ctrl_c().await?;
    client.destroy().await?;

    Ok(())
}

