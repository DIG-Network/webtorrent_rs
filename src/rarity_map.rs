use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tracks piece rarity across peers
pub struct RarityMap {
    piece_counts: Arc<RwLock<HashMap<usize, usize>>>,
    #[allow(dead_code)]
    num_pieces: usize,
}

impl RarityMap {
    pub fn new(num_pieces: usize) -> Self {
        Self {
            piece_counts: Arc::new(RwLock::new(HashMap::new())),
            num_pieces,
        }
    }

    /// Update piece availability from a peer's bitfield
    pub async fn update_peer(&self, bitfield: &bitvec::prelude::BitVec) {
        let mut counts = self.piece_counts.write().await;
        for (i, has_piece) in bitfield.iter().enumerate() {
            if *has_piece {
                *counts.entry(i).or_insert(0) += 1;
            }
        }
    }

    /// Remove a peer's pieces from the rarity map
    pub async fn remove_peer(&self, bitfield: &bitvec::prelude::BitVec) {
        let mut counts = self.piece_counts.write().await;
        for (i, has_piece) in bitfield.iter().enumerate() {
            if *has_piece {
                if let Some(count) = counts.get_mut(&i) {
                    if *count > 0 {
                        *count -= 1;
                    }
                }
            }
        }
    }

    /// Get the rarest piece that matches the filter
    pub async fn get_rarest_piece<F>(&self, filter: F) -> Option<usize>
    where
        F: Fn(usize) -> bool,
    {
        let counts = self.piece_counts.read().await;
        let mut rarest: Option<(usize, usize)> = None;

        for (piece_index, count) in counts.iter() {
            if filter(*piece_index) {
                match rarest {
                    None => rarest = Some((*piece_index, *count)),
                    Some((_, min_count)) if *count < min_count => {
                        rarest = Some((*piece_index, *count));
                    }
                    _ => {}
                }
            }
        }

        rarest.map(|(index, _)| index)
    }

    pub async fn destroy(&self) {
        self.piece_counts.write().await.clear();
    }
}

