# Dig-Relay Tracker Integration

This document describes how to use the WebTorrent Rust library with the dig-relay tracker.

## Tracker URL

The dig-relay tracker is available at:
- **Tracker URL**: `http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000`
- **Announce Endpoint**: `http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/announce`
- **Stats Page**: `http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/stats`

## Seeding Files

### Using the Seed Function

The easiest way to seed a file to the dig-relay tracker is using the `seed` function:

```rust
use webtorrent::{WebTorrent, WebTorrentOptions};
use bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = WebTorrent::new(WebTorrentOptions::default()).await?;
    
    // Seed a file
    let data = Bytes::from("Hello, World! This is a test file.");
    let torrent = client.seed(
        "test.txt".to_string(),
        data,
        Some(vec!["http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/announce".to_string()])
    ).await?;
    
    println!("Seeding torrent: {}", torrent.name());
    println!("Info hash: {}", hex::encode(torrent.info_hash()));
    
    // Keep seeding
    tokio::signal::ctrl_c().await?;
    client.destroy().await?;
    
    Ok(())
}
```

### Using the Example

Run the provided example:

```bash
cargo run --example seed_to_dig_relay
```

This will:
1. Create a test file
2. Create a torrent with the dig-relay tracker
3. Start seeding
4. Periodically re-announce to the tracker

### Manual Torrent Creation

You can also create torrents manually:

```rust
use webtorrent::{WebTorrent, TorrentCreator};
use bytes::Bytes;

let creator = TorrentCreator::new()
    .with_announce(vec!["http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/announce".to_string()]);

let (torrent_file, info_hash) = creator
    .create_from_data("my_file.txt".to_string(), file_data)
    .await?;

let torrent = client.add(torrent_file).await?;
```

## Automatic Announcement

When you add a torrent with the dig-relay tracker in its announce list, the discovery module will automatically:

1. Announce to the tracker with event "started" when the torrent is ready
2. Periodically re-announce based on the tracker's interval
3. Announce with event "completed" when seeding starts
4. Announce with event "stopped" when the torrent is removed

## Checking Stats

After seeding, you can check the tracker stats at:
http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/stats

The stats page will show:
- Number of torrents
- Number of active torrents
- Connected peers
- Peers seeding only
- Peers leeching only
- Peers seeding & leeching
- IPv4/IPv6 peer counts
- Client information

## Testing

Run the integration test (marked with `#[ignore]` by default):

```bash
# Run the ignored test
cargo test -- --ignored test_seed_to_dig_relay_tracker

# Or run all ignored tests
cargo test -- --ignored
```

## Tracker Protocol

The library implements the standard BitTorrent tracker protocol:

- **Announce**: GET request to `/announce` with query parameters
- **Scrape**: GET request to `/scrape` with query parameters
- **Compact format**: Uses compact peer format (6 bytes per peer)
- **Events**: Supports "started", "stopped", "completed", and periodic updates

## Troubleshooting

### Tracker Not Responding

If the tracker doesn't respond:
1. Check network connectivity
2. Verify the tracker URL is correct
3. Check if the tracker requires authentication
4. Look at the error messages in the logs

### Stats Not Updating

If stats don't update on the tracker:
1. Ensure the torrent is actually seeding (not just added)
2. Wait for the tracker's update interval
3. Check that the announce was successful (look for errors)
4. Verify the info hash matches what's shown on the stats page

### Port Issues

If you have port issues:
1. Ensure the client port is accessible
2. Check firewall settings
3. Consider using NAT traversal (UPnP/NAT-PMP) if behind a router

## Example Output

When seeding successfully, you should see:

```
Torrent created!
Info hash: 0123456789abcdef0123456789abcdef01234567
Torrent file size: 234 bytes
Adding torrent to client...
Torrent added: test_file.txt
Announcing to tracker...
✓ Successfully announced to tracker!
  Interval: 1800 seconds
  Seeders: 1
  Leechers: 0

Check the stats page: http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/stats

Seeding... Press Ctrl+C to stop
```

