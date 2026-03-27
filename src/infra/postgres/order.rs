use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::order::{Order, OrderError};

pub struct OrderRepository {
    pool: PgPool,
}

impl OrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl crate::domain::order::OrderRepository for OrderRepository {
    async fn insert(&self, _order: &Order) -> Result<(), OrderError> {
        let _ = &self.pool;
        Ok(())
    }

    async fn get_by_id(&self, _order_id: Uuid) -> Result<Option<Order>, OrderError> {
        let _ = &self.pool;
        Ok(None)
    }
}
