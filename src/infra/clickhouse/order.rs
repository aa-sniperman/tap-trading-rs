use async_trait::async_trait;
use chrono::{DateTime, Utc};
use clickhouse::Row;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    domain::order::{OrderError, OrderEvent, OrderEventRepository, OrderEventType},
    infra::clickhouse::ClickHouseClient,
};

#[derive(Clone)]
pub struct OrderEventRepositoryImpl {
    client: ClickHouseClient,
}

impl OrderEventRepositoryImpl {
    pub fn new(client: ClickHouseClient) -> Self {
        Self { client }
    }
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
struct OrderEventRow {
    event_id: Uuid,
    order_id: Uuid,
    user_id: Uuid,
    bet_amount_minor: i64,
    payout_amount_minor: i64,
    bet_asset: String,
    bet_price_lower_bound: f64,
    bet_price_upper_bound: f64,
    bet_time: DateTime<Utc>,
    event_type: String,
    reason: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<OrderEvent> for OrderEventRow {
    fn from(value: OrderEvent) -> Self {
        Self {
            event_id: value.event_id,
            order_id: value.order_id,
            user_id: value.user_id,
            bet_amount_minor: value.bet_amount_minor,
            payout_amount_minor: value.payout_amount_minor,
            bet_asset: value.bet_asset,
            bet_price_lower_bound: value.bet_price_lower_bound,
            bet_price_upper_bound: value.bet_price_upper_bound,
            bet_time: value.bet_time,
            event_type: event_type_to_str(&value.event_type).to_owned(),
            reason: value.reason,
            created_at: value.created_at,
        }
    }
}

#[async_trait]
impl OrderEventRepository for OrderEventRepositoryImpl {
    async fn append_event(&self, event: &OrderEvent) -> Result<(), OrderError> {
        self.append_events(std::slice::from_ref(event)).await
    }

    async fn append_events(&self, events: &[OrderEvent]) -> Result<(), OrderError> {
        let mut insert = self
            .client
            .insert("order_events")
            .map_err(|error| OrderError::Repository(error.to_string()))?;

        for event in events {
            insert
                .write(&OrderEventRow::from(event.clone()))
                .await
                .map_err(|error| OrderError::Repository(error.to_string()))?;
        }

        insert
            .end()
            .await
            .map_err(|error| OrderError::Repository(error.to_string()))
    }
}

fn event_type_to_str(value: &OrderEventType) -> &'static str {
    match value {
        OrderEventType::Placed => "placed",
        OrderEventType::Confirmed => "confirmed",
        OrderEventType::Reverted => "reverted",
        OrderEventType::SettledWin => "settled_win",
        OrderEventType::SettledLose => "settled_lose",
        OrderEventType::SettlementReverted => "settlement_reverted",
    }
}
