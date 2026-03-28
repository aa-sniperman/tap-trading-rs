use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Semaphore;
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
    SettledWinPendingEffect,
    SettledLosePendingEffect,
    SettledWin,
    SettledLose,
    SettlementReverted,
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
    async fn publish_balance_update(&self, user_id: UserId) -> Result<(), OrderError>;
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

    async fn publish_balance_update(&self, _user_id: UserId) -> Result<(), OrderError> {
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
    side_effect_limit: Arc<Semaphore>,
}

impl OrderService {
    pub fn new(
        events: impl OrderEventRepository + 'static,
        cache: impl OrderCacheRepository + 'static,
        fanout: impl OrderFanout + 'static,
        ledger: LedgerService,
        side_effect_max_concurrency: usize,
    ) -> Self {
        Self {
            events: Arc::new(events),
            cache: Arc::new(cache),
            fanout: Arc::new(fanout),
            ledger,
            side_effect_limit: Arc::new(Semaphore::new(side_effect_max_concurrency.max(1))),
        }
    }

    pub async fn place_order_async(
        &self,
        intent: OrderIntent,
    ) -> Result<CachedOrder, OrderError> {
        let now = Utc::now();
        let placed_event = Self::placed_event(&intent, now);

        self.events.append_event(&placed_event).await?;

        let cached = Self::accepted_order(&intent, now);

        self.cache.set_order(&cached).await?;

        let worker = self.clone();
        let accepted_order = cached.clone();
        tokio::spawn(async move {
            if let Err(error) = worker.spawn_place_side_effects(intent, accepted_order).await {
                error!(error = %error, "order saga worker failed");
            }
        });

        Ok(cached)
    }

    pub async fn confirm_or_revert_order(
        &self,
        intent: OrderIntent,
        accepted_order: CachedOrder,
    ) -> Result<(), OrderError> {
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
                self.events.append_event(&Self::confirmed_event(&intent, now)).await?;

                let confirmed_order = Self::confirmed_order(&intent, now);
                self.cache.set_order(&confirmed_order).await?;
                self.fanout.publish_status(&confirmed_order).await?;
            }
            Err(error) => {
                let now = Utc::now();
                let reason = error.to_string();
                self.events
                    .append_event(&Self::reverted_event(&intent, reason.clone(), now))
                    .await?;

                if let Err(cache_error) = self.cache.delete_order(accepted_order.order_id).await {
                    warn!(error = %cache_error, order_id = %intent.order_id, reason, "failed to evict reverted order from cache");
                }
            }
        }

        Ok(())
    }

    async fn spawn_place_side_effects(
        &self,
        intent: OrderIntent,
        accepted_order: CachedOrder,
    ) -> Result<(), OrderError> {
        let permit = self
            .side_effect_limit
            .clone()
            .acquire_owned()
            .await
            .map_err(|error| OrderError::Repository(error.to_string()))?;
        if let Err(error) = self.fanout.publish_status(&accepted_order).await {
            warn!(order_id = %intent.order_id, error = %error, "failed to publish accepted order status");
        }
        let result = self.confirm_or_revert_order(intent, accepted_order).await;
        drop(permit);
        result
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
                settlement_events.push(Self::settled_event(order, SettlementOutcome::Win, None, Utc::now()));
                settled_orders.push((order.clone(), SettlementOutcome::Win));
            } else if timed_out {
                lost += 1;
                settlement_events.push(Self::settled_event(order, SettlementOutcome::Lose, None, Utc::now()));
                settled_orders.push((order.clone(), SettlementOutcome::Lose));
            }
        }

        if !settlement_events.is_empty() {
            self.events.append_events(&settlement_events).await?;
        }

        for (order, outcome) in settled_orders {
            let pending_order = Self::pending_settlement_order(&order, outcome.clone(), Utc::now());
            self.cache.set_order(&pending_order).await?;
            self.spawn_settlement_side_effects(pending_order, outcome);
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

    fn spawn_settlement_side_effects(&self, pending_order: CachedOrder, outcome: SettlementOutcome) {
        let ledger = self.ledger.clone();
        let cache = self.cache.clone();
        let fanout = self.fanout.clone();
        let events = self.events.clone();
        let side_effect_limit = self.side_effect_limit.clone();
        tokio::spawn(async move {
            let permit = match side_effect_limit.acquire_owned().await {
                Ok(permit) => permit,
                Err(error) => {
                    warn!(order_id = %pending_order.order_id, error = %error, "failed to acquire settlement side-effect permit");
                    return;
                }
            };

            if let Err(error) = fanout
                .publish_settlement(pending_order.order_id, outcome.clone())
                .await
            {
                warn!(order_id = %pending_order.order_id, error = %error, "failed to publish provisional settlement");
            }

            let ledger_result = match outcome {
                SettlementOutcome::Win => {
                    ledger
                        .order_settle_win(OrderSettleWinParams {
                            user_id: pending_order.user_id,
                            bet_amount: Money {
                                asset: pending_order.bet_asset.clone(),
                                amount_minor: pending_order.bet_amount_minor,
                            },
                            payout_amount: Money {
                                asset: pending_order.bet_asset.clone(),
                                amount_minor: pending_order.payout_amount_minor,
                            },
                            order_id: pending_order.order_id,
                        })
                        .await
                }
                SettlementOutcome::Lose => {
                    ledger
                        .order_settle_lose(OrderSettleLoseParams {
                            user_id: pending_order.user_id,
                            bet_amount: Money {
                                asset: pending_order.bet_asset.clone(),
                                amount_minor: pending_order.bet_amount_minor,
                            },
                            order_id: pending_order.order_id,
                        })
                        .await
                }
            };

            match ledger_result {
                Ok(()) => {
                    if let Err(error) = fanout.publish_balance_update(pending_order.user_id).await {
                        warn!(order_id = %pending_order.order_id, error = %error, "failed to publish balance update");
                    }
                    let final_order =
                        OrderService::final_settlement_order(&pending_order, outcome.clone(), Utc::now());
                    if let Err(error) = cache.set_order(&final_order).await {
                        warn!(order_id = %pending_order.order_id, error = %error, "failed to persist final settled order cache");
                    }
                }
                Err(error) => {
                    let reason = error.to_string();
                    let revert_event =
                        OrderService::settlement_reverted_event(&pending_order, reason.clone(), Utc::now());
                    if let Err(event_error) = events.append_event(&revert_event).await {
                        warn!(order_id = %pending_order.order_id, error = %event_error, "failed to append settlement revert event");
                    }
                    if let Err(fanout_error) = fanout.publish_revert(pending_order.order_id, &reason).await {
                        warn!(order_id = %pending_order.order_id, error = %fanout_error, "failed to publish settlement revert");
                    }
                    let reverted_order =
                        OrderService::settlement_reverted_order(&pending_order, reason, Utc::now());
                    if let Err(cache_error) = cache.set_order(&reverted_order).await {
                        warn!(order_id = %pending_order.order_id, error = %cache_error, "failed to persist reverted settlement cache");
                    }
                }
            }

            drop(permit);
        });
    }

    fn cached_from_intent(
        intent: &OrderIntent,
        status: OrderCacheStatus,
        reason: Option<String>,
        updated_at: DateTime<Utc>,
    ) -> CachedOrder {
        CachedOrder {
            order_id: intent.order_id,
            user_id: intent.user_id,
            bet_amount_minor: intent.bet_amount_minor,
            payout_amount_minor: intent.payout_amount_minor,
            bet_asset: intent.bet_asset.clone(),
            bet_price_lower_bound: intent.bet_price_lower_bound,
            bet_price_upper_bound: intent.bet_price_upper_bound,
            bet_time: intent.bet_time,
            status,
            reason,
            updated_at,
        }
    }

    fn cached_from_order(
        order: &CachedOrder,
        status: OrderCacheStatus,
        reason: Option<String>,
        updated_at: DateTime<Utc>,
    ) -> CachedOrder {
        CachedOrder {
            order_id: order.order_id,
            user_id: order.user_id,
            bet_amount_minor: order.bet_amount_minor,
            payout_amount_minor: order.payout_amount_minor,
            bet_asset: order.bet_asset.clone(),
            bet_price_lower_bound: order.bet_price_lower_bound,
            bet_price_upper_bound: order.bet_price_upper_bound,
            bet_time: order.bet_time,
            status,
            reason,
            updated_at,
        }
    }

    fn event_from_intent(
        intent: &OrderIntent,
        event_type: OrderEventType,
        reason: Option<String>,
        created_at: DateTime<Utc>,
    ) -> OrderEvent {
        OrderEvent {
            event_id: Uuid::new_v4(),
            order_id: intent.order_id,
            user_id: intent.user_id,
            bet_amount_minor: intent.bet_amount_minor,
            payout_amount_minor: intent.payout_amount_minor,
            bet_asset: intent.bet_asset.clone(),
            bet_price_lower_bound: intent.bet_price_lower_bound,
            bet_price_upper_bound: intent.bet_price_upper_bound,
            bet_time: intent.bet_time,
            event_type,
            reason,
            created_at,
        }
    }

    fn event_from_order(
        order: &CachedOrder,
        event_type: OrderEventType,
        reason: Option<String>,
        created_at: DateTime<Utc>,
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
            event_type,
            reason,
            created_at,
        }
    }

    fn accepted_order(intent: &OrderIntent, updated_at: DateTime<Utc>) -> CachedOrder {
        Self::cached_from_intent(intent, OrderCacheStatus::Accepted, None, updated_at)
    }

    fn confirmed_order(intent: &OrderIntent, updated_at: DateTime<Utc>) -> CachedOrder {
        Self::cached_from_intent(intent, OrderCacheStatus::Confirmed, None, updated_at)
    }

    fn pending_settlement_order(
        order: &CachedOrder,
        outcome: SettlementOutcome,
        updated_at: DateTime<Utc>,
    ) -> CachedOrder {
        Self::cached_from_order(
            order,
            match outcome {
                SettlementOutcome::Win => OrderCacheStatus::SettledWinPendingEffect,
                SettlementOutcome::Lose => OrderCacheStatus::SettledLosePendingEffect,
            },
            None,
            updated_at,
        )
    }

    fn final_settlement_order(
        order: &CachedOrder,
        outcome: SettlementOutcome,
        updated_at: DateTime<Utc>,
    ) -> CachedOrder {
        Self::cached_from_order(
            order,
            match outcome {
                SettlementOutcome::Win => OrderCacheStatus::SettledWin,
                SettlementOutcome::Lose => OrderCacheStatus::SettledLose,
            },
            None,
            updated_at,
        )
    }

    fn settlement_reverted_order(
        order: &CachedOrder,
        reason: String,
        updated_at: DateTime<Utc>,
    ) -> CachedOrder {
        Self::cached_from_order(
            order,
            OrderCacheStatus::SettlementReverted,
            Some(reason),
            updated_at,
        )
    }

    fn placed_event(intent: &OrderIntent, created_at: DateTime<Utc>) -> OrderEvent {
        Self::event_from_intent(intent, OrderEventType::Placed, None, created_at)
    }

    fn confirmed_event(intent: &OrderIntent, created_at: DateTime<Utc>) -> OrderEvent {
        Self::event_from_intent(intent, OrderEventType::Confirmed, None, created_at)
    }

    fn reverted_event(intent: &OrderIntent, reason: String, created_at: DateTime<Utc>) -> OrderEvent {
        Self::event_from_intent(intent, OrderEventType::Reverted, Some(reason), created_at)
    }

    fn settled_event(
        order: &CachedOrder,
        outcome: SettlementOutcome,
        reason: Option<String>,
        created_at: DateTime<Utc>,
    ) -> OrderEvent {
        Self::event_from_order(
            order,
            match outcome {
                SettlementOutcome::Win => OrderEventType::SettledWin,
                SettlementOutcome::Lose => OrderEventType::SettledLose,
            },
            reason,
            created_at,
        )
    }

    fn settlement_reverted_event(
        order: &CachedOrder,
        reason: String,
        created_at: DateTime<Utc>,
    ) -> OrderEvent {
        Self::event_from_order(
            order,
            OrderEventType::SettlementReverted,
            Some(reason),
            created_at,
        )
    }
}
