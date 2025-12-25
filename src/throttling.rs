use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

/// Throttle group for rate limiting
pub struct ThrottleGroup {
    rate: Arc<RwLock<u64>>, // bytes per second
    enabled: Arc<RwLock<bool>>,
    tokens: Arc<RwLock<u64>>,
    last_update: Arc<RwLock<Instant>>,
}

impl ThrottleGroup {
    pub fn new(rate: u64, enabled: bool) -> Self {
        Self {
            rate: Arc::new(RwLock::new(rate)),
            enabled: Arc::new(RwLock::new(enabled)),
            tokens: Arc::new(RwLock::new(if enabled { rate } else { u64::MAX })),
            last_update: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Set the rate limit (bytes per second)
    pub async fn set_rate(&self, rate: u64) {
        *self.rate.write().await = rate;
        self.update_tokens().await;
    }

    /// Enable or disable throttling
    pub async fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().await = enabled;
        if enabled {
            self.update_tokens().await;
        } else {
            *self.tokens.write().await = u64::MAX;
        }
    }

    /// Update tokens based on elapsed time
    async fn update_tokens(&self) {
        if !*self.enabled.read().await {
            return;
        }

        let mut last_update = self.last_update.write().await;
        let mut tokens = self.tokens.write().await;
        let rate = *self.rate.read().await;

        let now = Instant::now();
        let elapsed = now.duration_since(*last_update);
        let new_tokens = (elapsed.as_secs_f64() * rate as f64) as u64;

        *tokens = (*tokens + new_tokens).min(rate);
        *last_update = now;
    }

    /// Try to consume tokens for the given amount of data
    pub async fn try_consume(&self, amount: u64) -> bool {
        self.update_tokens().await;

        if !*self.enabled.read().await {
            return true;
        }

        let mut tokens = self.tokens.write().await;
        if *tokens >= amount {
            *tokens -= amount;
            true
        } else {
            false
        }
    }

    /// Wait until enough tokens are available
    pub async fn wait_for_tokens(&self, amount: u64) {
        while !self.try_consume(amount).await {
            let rate = *self.rate.read().await;
            let tokens = *self.tokens.read().await;
            let needed = amount.saturating_sub(tokens);
            let wait_time = if rate > 0 {
                Duration::from_secs_f64(needed as f64 / rate as f64)
            } else {
                Duration::from_secs(1)
            };
            tokio::time::sleep(wait_time).await;
        }
    }

    pub async fn destroy(&self) {
        *self.enabled.write().await = false;
        *self.tokens.write().await = 0;
    }
}

