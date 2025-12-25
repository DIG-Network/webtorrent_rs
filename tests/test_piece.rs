use webtorrent::piece::Piece;
use bytes::Bytes;

#[tokio::test]
async fn test_piece_creation() {
    let piece = Piece::new(16384);
    assert_eq!(piece.length(), 16384);
    assert_eq!(piece.missing().await, 1); // 1 block for 16KB piece
}

#[tokio::test]
async fn test_piece_reserve() {
    let piece = Piece::new(16384);
    
    let block = piece.reserve().await;
    assert!(block.is_some());
    assert_eq!(block.unwrap(), 0);
}

#[tokio::test]
async fn test_piece_set_block() {
    let piece = Piece::new(16384);
    
    let block_index = piece.reserve().await.unwrap();
    let data = Bytes::from(vec![0u8; 16384]);
    
    let complete = piece.set(block_index, data).await.unwrap();
    assert!(complete); // Should be complete after setting the only block
    assert_eq!(piece.missing().await, 0);
}

#[tokio::test]
async fn test_piece_multiple_blocks() {
    let piece = Piece::new(32768); // 32KB = 2 blocks
    
    let block1 = piece.reserve().await.unwrap();
    let block2 = piece.reserve().await.unwrap();
    
    assert_eq!(block1, 0);
    assert_eq!(block2, 1);
    
    let data1 = Bytes::from(vec![0u8; 16384]);
    let complete1 = piece.set(block1, data1).await.unwrap();
    assert!(!complete1); // Not complete yet
    
    let data2 = Bytes::from(vec![0u8; 16384]);
    let complete2 = piece.set(block2, data2).await.unwrap();
    assert!(complete2); // Now complete
}

#[tokio::test]
async fn test_piece_flush() {
    let piece = Piece::new(16384);
    
    let block_index = piece.reserve().await.unwrap();
    let data = Bytes::from(vec![1u8; 16384]);
    piece.set(block_index, data).await.unwrap();
    
    let flushed = piece.flush().await;
    assert_eq!(flushed.len(), 16384);
    assert_eq!(flushed[0], 1);
}

#[tokio::test]
async fn test_piece_chunk_offset() {
    let piece = Piece::new(32768);
    
    assert_eq!(piece.chunk_offset(0), 0);
    assert_eq!(piece.chunk_offset(1), 16384);
}

#[tokio::test]
async fn test_piece_chunk_length() {
    let piece = Piece::new(16384);
    
    let length = piece.chunk_length(0).await;
    assert_eq!(length, 16384);
}

#[tokio::test]
async fn test_piece_last_chunk_length() {
    let piece = Piece::new(20000); // Not a multiple of block size
    
    let length = piece.chunk_length(0).await;
    assert_eq!(length, 20000); // Last (and only) block should be 20000 bytes
}

