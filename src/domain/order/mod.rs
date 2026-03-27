use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    common::{GridCellId, Money, OrderId, UserId},
    grid::GridService,
    ledger::LedgerService,
    price_stream::PriceStreamService,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: OrderId,
    pub user_id: UserId,
    pub grid_cell_id: GridCellId,
    pub stake: Money,
    pub reward_rate_bps: i32,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Won,
    Lost,
    Cancelled,
}

#[derive(Debug, Error)]
pub enum OrderError {
    #[error("duplicate user-cell order")]
    DuplicateOrder,
    #[error("invalid placement window")]
    InvalidPlacementWindow,
    #[error("repository error: {0}")]
    Repository(String),
}

#[async_trait]
pub trait OrderRepository: Send + Sync {
    async fn insert(&self, order: &Order) -> Result<(), OrderError>;
    async fn get_by_id(&self, order_id: Uuid) -> Result<Option<Order>, OrderError>;
}

#[derive(Clone)]
pub struct OrderService {
    repository: Arc<dyn OrderRepository>,
    #[allow(dead_code)]
    ledger: LedgerService,
    #[allow(dead_code)]
    prices: PriceStreamService,
    #[allow(dead_code)]
    grid: GridService,
}

impl OrderService {
    pub fn new(
        repository: impl OrderRepository + 'static,
        ledger: LedgerService,
        prices: PriceStreamService,
        grid: GridService,
    ) -> Self {
        Self {
            repository: Arc::new(repository),
            ledger,
            prices,
            grid,
        }
    }

    pub async fn place(&self, order: &Order) -> Result<(), OrderError> {
        self.repository.insert(order).await
    }

    pub async fn get(&self, order_id: Uuid) -> Result<Option<Order>, OrderError> {
        self.repository.get_by_id(order_id).await
    }
}
