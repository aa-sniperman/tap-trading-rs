use async_trait::async_trait;
use redis::aio::ConnectionManager;

use crate::domain::price_stream::{PriceCacheRepository, PriceStreamError, PriceTick};

#[derive(Clone)]
pub struct PriceCache {
    connection_manager: ConnectionManager,
}

impl PriceCache {
    pub fn new(connection_manager: ConnectionManager) -> Self {
        Self { connection_manager }
    }
}

#[async_trait]
impl PriceCacheRepository for PriceCache {
    async fn get_latest_price(&self, _symbol: &str) -> Result<Option<PriceTick>, PriceStreamError> {
        let _ = &self.connection_manager;
        Ok(None)
    }

    async fn set_latest_price(&self, _tick: &PriceTick) -> Result<(), PriceStreamError> {
        let _ = &self.connection_manager;
        Ok(())
    }
}
