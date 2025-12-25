use webtorrent::throttling::ThrottleGroup;
use tokio::time::Duration;

#[tokio::test]
async fn test_throttle_group_creation() {
    let throttle = ThrottleGroup::new(1000, true);
    
    // Should have initial tokens
    assert!(throttle.try_consume(100).await);
}

#[tokio::test]
async fn test_throttle_group_disabled() {
    let throttle = ThrottleGroup::new(1000, false);
    
    // When disabled, should always allow
    assert!(throttle.try_consume(10000).await);
}

#[tokio::test]
async fn test_throttle_group_rate_limit() {
    let throttle = ThrottleGroup::new(1000, true); // 1000 bytes/sec
    
    // Consume all tokens
    assert!(throttle.try_consume(1000).await);
    
    // Should not be able to consume more immediately
    assert!(!throttle.try_consume(1).await);
}

#[tokio::test]
async fn test_throttle_group_wait_for_tokens() {
    let throttle = ThrottleGroup::new(1000, true); // 1000 bytes/sec
    
    // Consume all tokens
    throttle.try_consume(1000).await;
    
    // Wait a bit and tokens should replenish
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Should be able to consume some tokens now
    assert!(throttle.try_consume(100).await);
}

#[tokio::test]
async fn test_throttle_group_set_rate() {
    let throttle = ThrottleGroup::new(1000, true);
    
    // Change rate
    throttle.set_rate(2000).await;
    
    // Should be able to consume more
    assert!(throttle.try_consume(2000).await);
}

#[tokio::test]
async fn test_throttle_group_enable_disable() {
    let throttle = ThrottleGroup::new(1000, true);
    
    // Disable
    throttle.set_enabled(false).await;
    assert!(throttle.try_consume(10000).await);
    
    // Re-enable
    throttle.set_enabled(true).await;
    throttle.try_consume(1000).await;
    assert!(!throttle.try_consume(1).await);
}

#[tokio::test]
async fn test_throttle_group_destroy() {
    let throttle = ThrottleGroup::new(1000, true);
    
    throttle.destroy().await;
    
    // After destroy, should not allow consumption
    assert!(!throttle.try_consume(1).await);
}

