use webtorrent::rarity_map::RarityMap;
use bitvec::prelude::*;

#[tokio::test]
async fn test_rarity_map_new() {
    let rarity_map = RarityMap::new(100);
    
    // Should be empty initially
    let rarest = rarity_map.get_rarest_piece(|_| true).await;
    assert!(rarest.is_none());
}

#[tokio::test]
async fn test_rarity_map_update_peer() {
    let rarity_map = RarityMap::new(10);
    
    let mut bitfield = bitvec![0; 10];
    bitfield.set(0, true);
    bitfield.set(1, true);
    bitfield.set(2, true);
    
    rarity_map.update_peer(&bitfield).await;
    
    // All pieces should have count 1
    let rarest = rarity_map.get_rarest_piece(|i| i < 3).await;
    assert!(rarest.is_some());
}

#[tokio::test]
async fn test_rarity_map_multiple_peers() {
    let rarity_map = RarityMap::new(10);
    
    // Peer 1 has pieces 0, 1, 2
    let mut bitfield1 = bitvec![0; 10];
    bitfield1.set(0, true);
    bitfield1.set(1, true);
    bitfield1.set(2, true);
    rarity_map.update_peer(&bitfield1).await;
    
    // Peer 2 has pieces 1, 2, 3
    let mut bitfield2 = bitvec![0; 10];
    bitfield2.set(1, true);
    bitfield2.set(2, true);
    bitfield2.set(3, true);
    rarity_map.update_peer(&bitfield2).await;
    
    // Piece 0 should be rarest (only 1 peer has it)
    let rarest = rarity_map.get_rarest_piece(|i| i < 4).await;
    assert_eq!(rarest, Some(0));
}

#[tokio::test]
async fn test_rarity_map_remove_peer() {
    let rarity_map = RarityMap::new(10);
    
    let mut bitfield = bitvec![0; 10];
    bitfield.set(0, true);
    bitfield.set(1, true);
    
    rarity_map.update_peer(&bitfield).await;
    rarity_map.remove_peer(&bitfield).await;
    
    // Should be empty again
    let rarest = rarity_map.get_rarest_piece(|_| true).await;
    assert!(rarest.is_none());
}

#[tokio::test]
async fn test_rarity_map_filter() {
    let rarity_map = RarityMap::new(10);
    
    let mut bitfield = bitvec![0; 10];
    bitfield.set(5, true);
    bitfield.set(6, true);
    bitfield.set(7, true);
    
    rarity_map.update_peer(&bitfield).await;
    
    // Filter should work
    let rarest = rarity_map.get_rarest_piece(|i| i >= 5 && i <= 7).await;
    assert!(rarest.is_some());
    assert!(rarest.unwrap() >= 5 && rarest.unwrap() <= 7);
    
    // Filter should exclude pieces
    let rarest = rarity_map.get_rarest_piece(|i| i < 5).await;
    assert!(rarest.is_none());
}

#[tokio::test]
async fn test_rarity_map_destroy() {
    let rarity_map = RarityMap::new(10);
    
    let mut bitfield = bitvec![0; 10];
    bitfield.set(0, true);
    rarity_map.update_peer(&bitfield).await;
    
    rarity_map.destroy().await;
    
    // Should be empty after destroy
    let rarest = rarity_map.get_rarest_piece(|_| true).await;
    assert!(rarest.is_none());
}

