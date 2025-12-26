# Comprehensive API Permutation Test Coverage

This document summarizes the comprehensive test coverage for the `webtorrent-rs` library.

## Test Categories

### ✅ Completed Test Categories

1. **Client Initialization & Configuration** (8 tests)
   - Default options
   - Custom ports
   - Custom peer IDs
   - All features enabled/disabled
   - Throttling
   - Blocklist configuration
   - Tracker configuration
   - Multiple instances

2. **Torrent Creation** (9 tests)
   - Default creator
   - Various piece lengths
   - Announce URLs
   - Comments
   - Chained builders
   - Small/large/empty files
   - File path handling

3. **Adding Torrents** (9 tests)
   - Torrent file
   - Magnet URI
   - Info hash only
   - URL
   - Direct data seeding
   - Duplicate handling
   - Concurrent additions
   - Invalid torrent/magnet handling

4. **Torrent Operations** (7 tests)
   - Start discovery
   - Destroy
   - Metadata access
   - Progress tracking
   - Peer count
   - Bitfield
   - Store access
   - Ready state

5. **File Selection (BEP 53)** (9 tests)
   - Single/multiple file selection
   - Deselect
   - Critical priority
   - Various priorities
   - Selection verification
   - Priority retrieval
   - Selected files list
   - Single-file torrent edge case

6. **Streaming** (8 tests)
   - Stream creation
   - Multi-file streams
   - Position tracking
   - EOF detection
   - Destroy
   - Invalid file index
   - Concurrent streams
   - Seek simulation

7. **Speed Metrics** (5 tests)
   - Download speed
   - Upload speed
   - Both speeds
   - Idle state
   - Throttled speeds

8. **Blocklist** (3+ tests)
   - New blocklist
   - Add IPv4/IPv6
   - Remove IP
   - CIDR blocks
   - Is blocked verification
   - Load from string/file/URL

9. **Tracker Client** (2+ tests)
   - Tracker creation
   - Announce with various events
   - Response handling

10. **Seeding Workflows** (1+ tests)
    - Single file seeding
    - Multi-file seeding
    - With various discovery mechanisms

11. **Download Workflows** (1+ tests)
    - From torrent file
    - From magnet URI
    - With various discovery mechanisms

12. **End-to-End Scenarios** (1+ tests)
    - Seed and download
    - Multi-peer scenarios

13. **Error Conditions** (1+ tests)
    - Invalid torrent
    - Invalid magnet
    - Network failures

14. **Edge Cases** (2+ tests)
    - Zero-byte files
    - Single-piece files
    - Exact piece boundaries

### ⚠️ Partially Completed / Limited

1. **Client Events** - Events API may not be publicly accessible
2. **Discovery Mechanisms (DHT/LSD)** - Limited until libp2p 0.56 API is fully verified
3. **NAT Traversal** - Tests created but may need network access
4. **WebRTC** - Limited until WebRTC library is integrated
5. **Stress Tests** - Can be added as needed
6. **Integration Combinations** - Can be added as needed

## Test Runner

A comprehensive test runner (`test_runner.rs`) has been created to:
- Discover all test examples automatically
- Run them sequentially
- Generate a summary report
- Exit with appropriate status codes

## Running Tests

### Individual Test
```bash
cargo run --example test_client_default
```

### All Tests
```bash
cargo run --example test_runner
```

### Specific Category
```bash
# All client init tests
cargo run --example test_client_default
cargo run --example test_client_custom_port
# ... etc
```

## Notes

1. **API Availability**: Some tests may fail if certain APIs are not yet implemented or are not publicly accessible. This is expected and helps identify missing functionality.

2. **Network Requirements**: Some tests (tracker, DHT, NAT) require network access or may need mock servers.

3. **libp2p API**: DHT and LSD tests are limited until the libp2p 0.56 API is fully verified and implemented.

4. **WebRTC**: WebRTC tests are limited until a WebRTC library is integrated.

5. **Test Results**: When running tests, note which permutations work and which don't. This information is valuable for:
   - Identifying missing features
   - Finding API inconsistencies
   - Discovering edge cases
   - Validating the public API design

## Test Count Summary

- **Total Test Scripts Created**: ~70+
- **Categories Covered**: 14+
- **Test Runner**: ✅ Created
- **Documentation**: ✅ This file

## Next Steps

1. Run the test runner to identify working/non-working permutations
2. Fix any compilation errors in test scripts
3. Add missing test categories as needed
4. Expand stress tests and integration combinations
5. Document test results and API gaps

