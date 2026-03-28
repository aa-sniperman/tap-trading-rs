use async_trait::async_trait;
use chrono::{DateTime, Utc};
use redis::{aio::ConnectionManager, AsyncCommands};
use uuid::Uuid;

use crate::domain::{
    common::OrderId,
    order::{CachedOrder, OrderCacheRepository, OrderError},
};

#[derive(Clone)]
pub struct OrderCache {
    connection_manager: ConnectionManager,
}

impl OrderCache {
    pub fn new(connection_manager: ConnectionManager) -> Self {
        Self { connection_manager }
    }

    fn key(order_id: OrderId) -> String {
        format!("order:{}", order_id)
    }

    fn active_index_key() -> &'static str {
        "orders:active"
    }
}

#[async_trait]
impl OrderCacheRepository for OrderCache {
    async fn set_order(&self, order: &CachedOrder) -> Result<(), OrderError> {
        let mut connection = self.connection_manager.clone();
        let payload = serde_json::to_string(order)
            .map_err(|error| OrderError::Cache(error.to_string()))?;

        connection
            .set::<_, _, ()>(Self::key(order.order_id), payload)
            .await
            .map_err(|error| OrderError::Cache(error.to_string()))?;

        connection
            .zadd(
                Self::active_index_key(),
                order.order_id.to_string(),
                order.bet_time.timestamp_millis(),
            )
            .await
            .map(|_: usize| ())
            .map_err(|error| OrderError::Cache(error.to_string()))
    }

    async fn delete_order(&self, order_id: OrderId) -> Result<(), OrderError> {
        let mut connection = self.connection_manager.clone();
        let member = order_id.to_string();
        connection
            .del::<_, i32>(Self::key(order_id))
            .await
            .map_err(|error| OrderError::Cache(error.to_string()))?;

        connection
            .zrem::<_, _, i32>(Self::active_index_key(), member)
            .await
            .map(|_| ())
            .map_err(|error| OrderError::Cache(error.to_string()))
    }

    async fn get_order(&self, order_id: Uuid) -> Result<Option<CachedOrder>, OrderError> {
        let mut connection = self.connection_manager.clone();
        let payload: Option<String> = connection
            .get(Self::key(order_id))
            .await
            .map_err(|error| OrderError::Cache(error.to_string()))?;

        payload
            .map(|value| serde_json::from_str::<CachedOrder>(&value))
            .transpose()
            .map_err(|error| OrderError::Cache(error.to_string()))
    }

    async fn list_active_orders_up_to(
        &self,
        observed_at: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<CachedOrder>, OrderError> {
        let mut connection = self.connection_manager.clone();
        let order_ids: Vec<String> = redis::cmd("ZRANGEBYSCORE")
            .arg(Self::active_index_key())
            .arg("-inf")
            .arg(observed_at.timestamp_millis())
            .arg("LIMIT")
            .arg(0)
            .arg(limit)
            .query_async(&mut connection)
            .await
            .map_err(|error| OrderError::Cache(error.to_string()))?;

        let mut orders = Vec::with_capacity(order_ids.len());
        for order_id in order_ids {
            let parsed = Uuid::parse_str(&order_id)
                .map_err(|error| OrderError::Cache(error.to_string()))?;
            if let Some(order) = self.get_order(parsed).await? {
                orders.push(order);
            }
        }

        Ok(orders)
    }
}
