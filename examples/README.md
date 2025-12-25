# WebTorrent Rust Examples

## seed_to_dig_relay

This example demonstrates how to seed a file to the dig-relay tracker.

### Usage

```bash
cargo run --example seed_to_dig_relay
```

### What it does

1. Creates a WebTorrent client
2. Creates a test file with sample data
3. Creates a torrent file with the dig-relay tracker URL
4. Adds the torrent to the client (automatically starts seeding)
5. Announces to the tracker
6. Keeps seeding and periodically re-announces

### Expected Output

```
WebTorrent Rust - Seed to dig-relay tracker example
Tracker: http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000
Client created
Creating torrent...
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

### Verifying

After running the example, check the tracker stats page:
http://dig-relay-prod.eba-2cmanxbe.us-east-1.elasticbeanstalk.com:8000/stats

You should see:
- The number of torrents increase
- Your peer appear in the connected peers
- The seeder count update

