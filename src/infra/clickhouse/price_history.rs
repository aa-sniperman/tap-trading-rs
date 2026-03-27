use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    domain::price_stream::{PriceStreamError, PriceTick},
    infra::clickhouse::ClickHouseClient,
};

#[derive(Clone)]
pub struct PriceHistoryRepository {
    client: ClickHouseClient,
}

impl PriceHistoryRepository {
    pub fn new(client: ClickHouseClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl crate::domain::price_stream::PriceHistoryRepository for PriceHistoryRepository {
    async fn append_tick(&self, _tick: &PriceTick) -> Result<(), PriceStreamError> {
        let _ = &self.client;
        Ok(())
    }

    async fn find_range(
        &self,
        _symbol: &str,
        _start_at: DateTime<Utc>,
        _end_at: DateTime<Utc>,
    ) -> Result<Vec<PriceTick>, PriceStreamError> {
        let _ = &self.client;
        Ok(vec![])
    }
}
