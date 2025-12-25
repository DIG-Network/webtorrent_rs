use webtorrent::store::{ChunkStore, MemoryChunkStore};
use bytes::Bytes;

#[tokio::test]
async fn test_memory_store_put_get() {
    let store = MemoryChunkStore::new();
    
    let data = Bytes::from(vec![1, 2, 3, 4, 5]);
    store.put(0, data.clone()).await.unwrap();
    
    let retrieved = store.get(0, 0, 5).await.unwrap();
    assert_eq!(retrieved, data);
}

#[tokio::test]
async fn test_memory_store_get_nonexistent() {
    let store = MemoryChunkStore::new();
    
    let result = store.get(0, 0, 10).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_memory_store_get_partial() {
    let store = MemoryChunkStore::new();
    
    let data = Bytes::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    store.put(0, data).await.unwrap();
    
    let retrieved = store.get(0, 2, 5).await.unwrap();
    assert_eq!(retrieved, Bytes::from(vec![3, 4, 5, 6, 7]));
}

#[tokio::test]
async fn test_memory_store_multiple_pieces() {
    let store = MemoryChunkStore::new();
    
    let data1 = Bytes::from(vec![1, 2, 3]);
    let data2 = Bytes::from(vec![4, 5, 6]);
    
    store.put(0, data1.clone()).await.unwrap();
    store.put(1, data2.clone()).await.unwrap();
    
    assert_eq!(store.get(0, 0, 3).await.unwrap(), data1);
    assert_eq!(store.get(1, 0, 3).await.unwrap(), data2);
}

#[tokio::test]
async fn test_memory_store_close() {
    let store = MemoryChunkStore::new();
    
    let result = store.close().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_memory_store_destroy() {
    let store = MemoryChunkStore::new();
    
    let data = Bytes::from(vec![1, 2, 3]);
    store.put(0, data).await.unwrap();
    
    store.destroy().await.unwrap();
    
    // After destroy, get should fail
    let result = store.get(0, 0, 3).await;
    assert!(result.is_err());
}

