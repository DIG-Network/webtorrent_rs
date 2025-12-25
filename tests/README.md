# WebTorrent Rust Test Suite

This directory contains comprehensive tests for the WebTorrent Rust library.

## Test Structure

### Unit Tests

- **test_bencode.rs**: Tests for bencode parsing and encoding
- **test_magnet.rs**: Tests for magnet URI parsing
- **test_client.rs**: Tests for WebTorrent client functionality
- **test_torrent.rs**: Tests for torrent management
- **test_piece.rs**: Tests for piece management
- **test_file.rs**: Tests for file handling
- **test_store.rs**: Tests for chunk storage
- **test_throttling.rs**: Tests for rate limiting
- **test_protocol.rs**: Tests for BitTorrent protocol
- **test_extensions.rs**: Tests for protocol extensions
- **test_selections.rs**: Tests for piece selection
- **test_rarity_map.rs**: Tests for rarity tracking

### Integration Tests

- **test_integration.rs**: End-to-end integration tests

## Running Tests

Run all tests:
```bash
cargo test
```

Run specific test file:
```bash
cargo test --test test_bencode
```

Run with output:
```bash
cargo test -- --nocapture
```

Run in release mode:
```bash
cargo test --release
```

## Test Coverage

The test suite covers:

- ✅ Bencode encoding/decoding
- ✅ Magnet URI parsing
- ✅ Client lifecycle
- ✅ Torrent management
- ✅ Piece operations
- ✅ File handling
- ✅ Storage backends
- ✅ Throttling
- ✅ Protocol handshakes
- ✅ Extension protocols
- ✅ Selection management
- ✅ Rarity tracking
- ✅ Integration scenarios

## Adding New Tests

When adding new functionality, ensure you:

1. Add unit tests for the new module
2. Add integration tests if applicable
3. Update this README
4. Ensure tests pass with `cargo test`

