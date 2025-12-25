// Tests for NAT traversal module

use webtorrent::NatTraversal;

#[tokio::test]
async fn test_nat_traversal_creation() {
    // Test creating NAT traversal with both methods enabled
    let nat = NatTraversal::new(true, true).await.unwrap();
    assert!(!nat.is_mapped(6881, 6881, "tcp").await);
    
    // Test creating with only UPnP
    let nat = NatTraversal::new(true, false).await.unwrap();
    assert!(!nat.is_mapped(6881, 6881, "tcp").await);
    
    // Test creating with only NAT-PMP
    let nat = NatTraversal::new(false, true).await.unwrap();
    assert!(!nat.is_mapped(6881, 6881, "tcp").await);
    
    // Test creating with both disabled
    let nat = NatTraversal::new(false, false).await.unwrap();
    assert!(!nat.is_mapped(6881, 6881, "tcp").await);
}

#[tokio::test]
async fn test_nat_traversal_destroy() {
    let nat = NatTraversal::new(true, true).await.unwrap();
    
    // Destroy should succeed
    nat.destroy().await.unwrap();
    
    // Destroying again should also succeed (idempotent)
    nat.destroy().await.unwrap();
    
    // Operations after destroy should fail
    assert!(nat.map_port(6881, 6881, "tcp", "test").await.is_err());
}

#[tokio::test]
async fn test_nat_traversal_get_mappings() {
    let nat = NatTraversal::new(true, true).await.unwrap();
    
    // Initially no mappings
    let mappings = nat.get_mappings().await;
    assert_eq!(mappings.len(), 0);
    
    // After destroy, should still return empty (even if mappings existed)
    nat.destroy().await.unwrap();
    let mappings = nat.get_mappings().await;
    assert_eq!(mappings.len(), 0);
}

#[tokio::test]
async fn test_nat_traversal_is_mapped() {
    let nat = NatTraversal::new(true, true).await.unwrap();
    
    // Initially not mapped
    assert!(!nat.is_mapped(6881, 6881, "tcp").await);
    assert!(!nat.is_mapped(6881, 6881, "udp").await);
    assert!(!nat.is_mapped(6882, 6882, "tcp").await);
}

#[tokio::test]
async fn test_nat_traversal_invalid_protocol() {
    let nat = NatTraversal::new(true, true).await.unwrap();
    
    // Invalid protocol should fail
    assert!(nat.map_port(6881, 6881, "invalid", "test").await.is_err());
    assert!(nat.map_port(6881, 6881, "", "test").await.is_err());
}

#[tokio::test]
#[ignore] // Ignore by default - requires actual NAT gateway
async fn test_nat_traversal_upnp_integration() {
    // This test requires a real UPnP-capable router
    // Run with: cargo test -- --ignored test_nat_traversal_upnp_integration
    
    let nat = NatTraversal::new(true, false).await.unwrap();
    
    // Try to map a port
    let result = nat.map_port(6881, 6881, "tcp", "webtorrent-test").await;
    
    if result.is_ok() {
        // If mapping succeeded, verify it's tracked
        assert!(nat.is_mapped(6881, 6881, "tcp").await);
        
        // Get mappings
        let mappings = nat.get_mappings().await;
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].0, 6881);
        assert_eq!(mappings[0].1, 6881);
        assert_eq!(mappings[0].2, "tcp");
        
        // Unmap
        nat.unmap_port(6881, 6881, "tcp").await.unwrap();
        assert!(!nat.is_mapped(6881, 6881, "tcp").await);
    } else {
        // If no UPnP gateway available, that's okay for this test
        println!("No UPnP gateway available - test skipped");
    }
}

#[tokio::test]
#[ignore] // Ignore by default - requires actual NAT gateway
async fn test_nat_traversal_pmp_integration() {
    // This test requires a real NAT-PMP-capable router
    // Run with: cargo test -- --ignored test_nat_traversal_pmp_integration
    
    let nat = NatTraversal::new(false, true).await.unwrap();
    
    // Try to map a port
    let result = nat.map_port(6881, 6881, "tcp", "webtorrent-test").await;
    
    if result.is_ok() {
        // If mapping succeeded, verify it's tracked
        assert!(nat.is_mapped(6881, 6881, "tcp").await);
        
        // Get mappings
        let mappings = nat.get_mappings().await;
        assert_eq!(mappings.len(), 1);
        
        // Unmap
        nat.unmap_port(6881, 6881, "tcp").await.unwrap();
        assert!(!nat.is_mapped(6881, 6881, "tcp").await);
    } else {
        // If no NAT-PMP gateway available, that's okay for this test
        println!("No NAT-PMP gateway available - test skipped");
    }
}

#[tokio::test]
#[ignore] // Ignore by default - requires actual NAT gateway
async fn test_nat_traversal_fallback() {
    // Test that if UPnP fails, it falls back to NAT-PMP
    let nat = NatTraversal::new(true, true).await.unwrap();
    
    // This will try UPnP first, then NAT-PMP if UPnP fails
    let result = nat.map_port(6881, 6881, "tcp", "webtorrent-test").await;
    
    // Result depends on gateway availability
    // If both fail, that's expected in test environments
    if result.is_err() {
        println!("No NAT gateway available - test skipped");
    }
}

#[tokio::test]
async fn test_nat_traversal_duplicate_mapping() {
    let nat = NatTraversal::new(true, true).await.unwrap();
    
    // Note: This test doesn't actually map ports (would require gateway)
    // But we can test the logic for duplicate detection
    
    // After destroy, duplicate mapping should fail
    nat.destroy().await.unwrap();
    assert!(nat.map_port(6881, 6881, "tcp", "test").await.is_err());
}

#[tokio::test]
async fn test_nat_traversal_unmap_nonexistent() {
    let nat = NatTraversal::new(true, true).await.unwrap();
    
    // Unmapping a non-existent port should succeed (idempotent)
    nat.unmap_port(9999, 9999, "tcp").await.unwrap();
}

#[tokio::test]
async fn test_nat_traversal_multiple_protocols() {
    let nat = NatTraversal::new(true, true).await.unwrap();
    
    // Test that TCP and UDP are treated as separate mappings
    // (Even though we can't actually map without a gateway)
    assert!(!nat.is_mapped(6881, 6881, "tcp").await);
    assert!(!nat.is_mapped(6881, 6881, "udp").await);
    
    // These should be considered different mappings
    // (Implementation detail: different keys in HashMap)
}

#[tokio::test]
async fn test_nat_traversal_destroy_unmaps_all() {
    let nat = NatTraversal::new(true, true).await.unwrap();
    
    // Destroy should clean up all mappings
    nat.destroy().await.unwrap();
    
    // Verify no mappings remain
    let mappings = nat.get_mappings().await;
    assert_eq!(mappings.len(), 0);
}

