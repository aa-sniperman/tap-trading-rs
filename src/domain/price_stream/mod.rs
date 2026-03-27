use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceTick {
    pub symbol: String,
    pub price: f64,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum PriceStreamError {
    #[error("cache error: {0}")]
    Cache(String),
    #[error("history error: {0}")]
    History(String),
}

#[async_trait]
pub trait PriceCacheRepository: Send + Sync {
    async fn get_latest_price(&self, symbol: &str) -> Result<Option<PriceTick>, PriceStreamError>;
    async fn set_latest_price(&self, tick: &PriceTick) -> Result<(), PriceStreamError>;
}

#[async_trait]
pub trait PriceHistoryRepository: Send + Sync {
    async fn append_tick(&self, tick: &PriceTick) -> Result<(), PriceStreamError>;
    async fn find_range(
        &self,
        symbol: &str,
        start_at: DateTime<Utc>,
        end_at: DateTime<Utc>,
    ) -> Result<Vec<PriceTick>, PriceStreamError>;
}

#[derive(Clone)]
pub struct PriceStreamService {
    cache: Arc<dyn PriceCacheRepository>,
    history: Arc<dyn PriceHistoryRepository>,
}

impl PriceStreamService {
    pub fn new(
        cache: impl PriceCacheRepository + 'static,
        history: impl PriceHistoryRepository + 'static,
    ) -> Self {
        Self {
            cache: Arc::new(cache),
            history: Arc::new(history),
        }
    }

    pub async fn ingest_tick(&self, tick: PriceTick) -> Result<(), PriceStreamError> {
        self.cache.set_latest_price(&tick).await?;
        self.history.append_tick(&tick).await
    }

    pub async fn latest_price(&self, symbol: &str) -> Result<Option<PriceTick>, PriceStreamError> {
        self.cache.get_latest_price(symbol).await
    }
}
