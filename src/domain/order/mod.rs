use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, warn};
use uuid::Uuid;

use crate::domain::{
    common::{Money, OrderId, UserId},
    ledger::{LedgerService, OrderHoldParams, OrderSettleLoseParams, OrderSettleWinParams},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderIntent {
    pub order_id: OrderId,
    pub user_id: UserId,
    pub bet_amount_minor: i64,
    pub payout_amount_minor: i64,
    pub bet_asset: String,
    pub bet_price_lower_bound: f64,
    pub bet_price_upper_bound: f64,
    pub bet_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderEventType {
    Placed,
    Confirmed,
    Reverted,
    SettledWin,
    SettledLose,
    SettlementReverted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    pub event_id: Uuid,
    pub order_id: OrderId,
    pub user_id: UserId,
    pub bet_amount_minor: i64,
    pub payout_amount_minor: i64,
    pub bet_asset: String,
    pub bet_price_lower_bound: f64,
    pub bet_price_upper_bound: f64,
    pub bet_time: DateTime<Utc>,
    pub event_type: OrderEventType,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderCacheStatus {
    Accepted,
    Confirmed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedOrder {
    pub order_id: OrderId,
    pub user_id: UserId,
    pub bet_amount_minor: i64,
    pub payout_amount_minor: i64,
    pub bet_asset: String,
    pub bet_price_lower_bound: f64,
    pub bet_price_upper_bound: f64,
    pub bet_time: DateTime<Utc>,
    pub status: OrderCacheStatus,
    pub reason: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementOutcome {
    Win,
    Lose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementTrigger {
    pub observed_at: DateTime<Utc>,
    pub observed_price: f64,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementBatchResult {
    pub observed_at: DateTime<Utc>,
    pub observed_price: f64,
    pub scanned: usize,
    pub confirmed: usize,
    pub won: usize,
    pub lost: usize,
}

#[derive(Debug, Error)]
pub enum OrderError {
    #[error("ledger error: {0}")]
    Ledger(String),
    #[error("repository error: {0}")]
    Repository(String),
    #[error("cache error: {0}")]
    Cache(String),
}

#[async_trait]
pub trait OrderEventRepository: Send + Sync {
    async fn append_event(&self, event: &OrderEvent) -> Result<(), OrderError>;
    async fn append_events(&self, events: &[OrderEvent]) -> Result<(), OrderError>;
}

#[async_trait]
pub trait OrderCacheRepository: Send + Sync {
    async fn set_order(&self, order: &CachedOrder) -> Result<(), OrderError>;
    async fn delete_order(&self, order_id: OrderId) -> Result<(), OrderError>;
    async fn get_order(&self, order_id: OrderId) -> Result<Option<CachedOrder>, OrderError>;
    async fn list_active_orders_up_to(
        &self,
        observed_at: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<CachedOrder>, OrderError>;
}

#[async_trait]
pub trait OrderFanout: Send + Sync {
    async fn publish_status(&self, order: &CachedOrder) -> Result<(), OrderError>;
    async fn publish_settlement(
        &self,
        order_id: OrderId,
        outcome: SettlementOutcome,
    ) -> Result<(), OrderError>;
    async fn publish_revert(
        &self,
        order_id: OrderId,
        reason: &str,
    ) -> Result<(), OrderError>;
}

#[derive(Clone)]
pub struct NoopOrderFanout;

#[async_trait]
impl OrderFanout for NoopOrderFanout {
    async fn publish_status(&self, _order: &CachedOrder) -> Result<(), OrderError> {
        Ok(())
    }

    async fn publish_settlement(
        &self,
        _order_id: OrderId,
        _outcome: SettlementOutcome,
    ) -> Result<(), OrderError> {
        Ok(())
    }

    async fn publish_revert(
        &self,
        _order_id: OrderId,
        _reason: &str,
    ) -> Result<(), OrderError> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct OrderService {
    events: Arc<dyn OrderEventRepository>,
    cache: Arc<dyn OrderCacheRepository>,
    fanout: Arc<dyn OrderFanout>,
    ledger: LedgerService,
}

impl OrderService {
    pub fn new(
        events: impl OrderEventRepository + 'static,
        cache: impl OrderCacheRepository + 'static,
        fanout: impl OrderFanout + 'static,
        ledger: LedgerService,
    ) -> Self {
        Self {
            events: Arc::new(events),
            cache: Arc::new(cache),
            fanout: Arc::new(fanout),
            ledger,
        }
    }

    pub async fn place_order_async(
        &self,
        intent: OrderIntent,
    ) -> Result<CachedOrder, OrderError> {
        let now = Utc::now();
        let placed_event = OrderEvent {
            event_id: Uuid::new_v4(),
            order_id: intent.order_id,
            user_id: intent.user_id,
            bet_amount_minor: intent.bet_amount_minor,
            payout_amount_minor: intent.payout_amount_minor,
            bet_asset: intent.bet_asset.clone(),
            bet_price_lower_bound: intent.bet_price_lower_bound,
            bet_price_upper_bound: intent.bet_price_upper_bound,
            bet_time: intent.bet_time,
            event_type: OrderEventType::Placed,
            reason: None,
            created_at: now,
        };

        self.events.append_event(&placed_event).await?;

        let cached = CachedOrder {
            order_id: intent.order_id,
            user_id: intent.user_id,
            bet_amount_minor: intent.bet_amount_minor,
            payout_amount_minor: intent.payout_amount_minor,
            bet_asset: intent.bet_asset.clone(),
            bet_price_lower_bound: intent.bet_price_lower_bound,
            bet_price_upper_bound: intent.bet_price_upper_bound,
            bet_time: intent.bet_time,
            status: OrderCacheStatus::Accepted,
            reason: None,
            updated_at: now,
        };

        self.cache.set_order(&cached).await?;
        self.fanout.publish_status(&cached).await?;

        let worker = self.clone();
        tokio::spawn(async move {
            if let Err(error) = worker.confirm_or_revert_order(intent).await {
                error!(error = %error, "order saga worker failed");
            }
        });

        Ok(cached)
    }

    pub async fn confirm_or_revert_order(&self, intent: OrderIntent) -> Result<(), OrderError> {
        let hold_result = self
            .ledger
            .order_hold(OrderHoldParams {
                user_id: intent.user_id,
                amount: Money {
                    asset: intent.bet_asset.clone(),
                    amount_minor: intent.bet_amount_minor,
                },
                order_id: intent.order_id,
            })
            .await;

        match hold_result {
            Ok(()) => {
                let now = Utc::now();
                self.events
                    .append_event(&OrderEvent {
                        event_id: Uuid::new_v4(),
                        order_id: intent.order_id,
                        user_id: intent.user_id,
                        bet_amount_minor: intent.bet_amount_minor,
                        payout_amount_minor: intent.payout_amount_minor,
                        bet_asset: intent.bet_asset.clone(),
                        bet_price_lower_bound: intent.bet_price_lower_bound,
                        bet_price_upper_bound: intent.bet_price_upper_bound,
                        bet_time: intent.bet_time,
                        event_type: OrderEventType::Confirmed,
                        reason: None,
                        created_at: now,
                    })
                    .await?;

                let confirmed_order = CachedOrder {
                    order_id: intent.order_id,
                    user_id: intent.user_id,
                    bet_amount_minor: intent.bet_amount_minor,
                    payout_amount_minor: intent.payout_amount_minor,
                    bet_asset: intent.bet_asset,
                    bet_price_lower_bound: intent.bet_price_lower_bound,
                    bet_price_upper_bound: intent.bet_price_upper_bound,
                    bet_time: intent.bet_time,
                    status: OrderCacheStatus::Confirmed,
                    reason: None,
                    updated_at: now,
                };
                self.cache.set_order(&confirmed_order).await?;
                self.fanout.publish_status(&confirmed_order).await?;
            }
            Err(error) => {
                let now = Utc::now();
                let reason = error.to_string();
                self.events
                    .append_event(&OrderEvent {
                        event_id: Uuid::new_v4(),
                        order_id: intent.order_id,
                        user_id: intent.user_id,
                        bet_amount_minor: intent.bet_amount_minor,
                        payout_amount_minor: intent.payout_amount_minor,
                        bet_asset: intent.bet_asset.clone(),
                        bet_price_lower_bound: intent.bet_price_lower_bound,
                        bet_price_upper_bound: intent.bet_price_upper_bound,
                        bet_time: intent.bet_time,
                        event_type: OrderEventType::Reverted,
                        reason: Some(reason.clone()),
                        created_at: now,
                    })
                    .await?;

                if let Err(cache_error) = self.cache.delete_order(intent.order_id).await {
                    warn!(error = %cache_error, order_id = %intent.order_id, reason, "failed to evict reverted order from cache");
                }
            }
        }

        Ok(())
    }

    pub async fn get_cached(&self, order_id: OrderId) -> Result<Option<CachedOrder>, OrderError> {
        self.cache.get_order(order_id).await
    }

    pub async fn trigger_settlement(
        &self,
        trigger: SettlementTrigger,
    ) -> Result<SettlementBatchResult, OrderError> {
        let active_orders = self
            .cache
            .list_active_orders_up_to(trigger.observed_at, trigger.limit)
            .await?;

        let mut confirmed = 0usize;
        let mut won = 0usize;
        let mut lost = 0usize;
        let mut settled_orders = Vec::new();
        let mut settlement_events = Vec::new();

        for order in active_orders.iter() {
            if !matches!(order.status, OrderCacheStatus::Confirmed) {
                continue;
            }

            confirmed += 1;

            let in_range = trigger.observed_price >= order.bet_price_lower_bound
                && trigger.observed_price <= order.bet_price_upper_bound;
            let timed_out = order.bet_time + Duration::seconds(5) <= trigger.observed_at;

            if in_range {
                won += 1;
                settlement_events.push(self.build_settlement_event(order, SettlementOutcome::Win, None));
                settled_orders.push((order.clone(), SettlementOutcome::Win));
            } else if timed_out {
                lost += 1;
                settlement_events.push(self.build_settlement_event(order, SettlementOutcome::Lose, None));
                settled_orders.push((order.clone(), SettlementOutcome::Lose));
            }
        }

        if !settlement_events.is_empty() {
            self.events.append_events(&settlement_events).await?;
        }

        for (order, outcome) in settled_orders {
            self.spawn_settlement_side_effects(order, outcome);
        }

        Ok(SettlementBatchResult {
            observed_at: trigger.observed_at,
            observed_price: trigger.observed_price,
            scanned: active_orders.len(),
            confirmed,
            won,
            lost,
        })
    }

    fn build_settlement_event(
        &self,
        order: &CachedOrder,
        outcome: SettlementOutcome,
        reason: Option<String>,
    ) -> OrderEvent {
        OrderEvent {
            event_id: Uuid::new_v4(),
            order_id: order.order_id,
            user_id: order.user_id,
            bet_amount_minor: order.bet_amount_minor,
            payout_amount_minor: order.payout_amount_minor,
            bet_asset: order.bet_asset.clone(),
            bet_price_lower_bound: order.bet_price_lower_bound,
            bet_price_upper_bound: order.bet_price_upper_bound,
            bet_time: order.bet_time,
            event_type: match outcome {
                SettlementOutcome::Win => OrderEventType::SettledWin,
                SettlementOutcome::Lose => OrderEventType::SettledLose,
            },
            reason,
            created_at: Utc::now(),
        }
    }

    fn spawn_settlement_side_effects(&self, order: CachedOrder, outcome: SettlementOutcome) {
        let ledger = self.ledger.clone();
        let cache = self.cache.clone();
        let fanout = self.fanout.clone();
        let events = self.events.clone();
        tokio::spawn(async move {
            if let Err(error) = fanout.publish_settlement(order.order_id, outcome.clone()).await {
                warn!(order_id = %order.order_id, error = %error, "failed to publish provisional settlement");
            }

            let ledger_result = match outcome {
                SettlementOutcome::Win => {
                    ledger
                        .order_settle_win(OrderSettleWinParams {
                            user_id: order.user_id,
                            bet_amount: Money {
                                asset: order.bet_asset.clone(),
                                amount_minor: order.bet_amount_minor,
                            },
                            payout_amount: Money {
                                asset: order.bet_asset.clone(),
                                amount_minor: order.payout_amount_minor,
                            },
                            order_id: order.order_id,
                        })
                        .await
                }
                SettlementOutcome::Lose => {
                    ledger
                        .order_settle_lose(OrderSettleLoseParams {
                            user_id: order.user_id,
                            bet_amount: Money {
                                asset: order.bet_asset.clone(),
                                amount_minor: order.bet_amount_minor,
                            },
                            order_id: order.order_id,
                        })
                        .await
                }
            };

            match ledger_result {
                Ok(()) => {
                    if let Err(error) = cache.delete_order(order.order_id).await {
                        warn!(order_id = %order.order_id, error = %error, "failed to evict settled order from cache");
                    }
                }
                Err(error) => {
                    let reason = error.to_string();
                    let revert_event = OrderEvent {
                        event_id: Uuid::new_v4(),
                        order_id: order.order_id,
                        user_id: order.user_id,
                        bet_amount_minor: order.bet_amount_minor,
                        payout_amount_minor: order.payout_amount_minor,
                        bet_asset: order.bet_asset.clone(),
                        bet_price_lower_bound: order.bet_price_lower_bound,
                        bet_price_upper_bound: order.bet_price_upper_bound,
                        bet_time: order.bet_time,
                        event_type: OrderEventType::SettlementReverted,
                        reason: Some(reason.clone()),
                        created_at: Utc::now(),
                    };
                    if let Err(event_error) = events.append_event(&revert_event).await {
                        warn!(order_id = %order.order_id, error = %event_error, "failed to append settlement revert event");
                    }
                    if let Err(fanout_error) = fanout.publish_revert(order.order_id, &reason).await {
                        warn!(order_id = %order.order_id, error = %fanout_error, "failed to publish settlement revert");
                    }
                }
            }
        });
    }
}
