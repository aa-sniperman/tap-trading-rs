use axum::{extract::{Path, State}, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::AppState,
    domain::order::{OrderIntent, SettlementBatchResult, SettlementTrigger},
};

#[derive(Deserialize)]
pub struct PlaceOrderRequest {
    pub user_id: Uuid,
    pub bet_amount_minor: i64,
    pub payout_amount_minor: i64,
    pub bet_asset: String,
    pub bet_price_lower_bound: f64,
    pub bet_price_upper_bound: f64,
}

#[derive(Serialize)]
pub struct OrderResponse {
    pub order_id: Uuid,
    pub status: &'static str,
}

#[derive(Deserialize)]
pub struct TriggerSettlementRequest {
    pub observed_at: DateTime<Utc>,
    pub observed_price: f64,
    pub limit: usize,
}

pub async fn list_orders(State(state): State<AppState>) -> Json<Vec<OrderResponse>> {
    let _ = state.order_service();
    Json(vec![])
}

pub async fn place_order(
    State(state): State<AppState>,
    Json(request): Json<PlaceOrderRequest>,
) -> Json<OrderResponse> {
    let order_id = Uuid::new_v4();
    let status = if state
        .order_service()
        .place_order_async(OrderIntent {
            order_id,
            user_id: request.user_id,
            bet_amount_minor: request.bet_amount_minor,
            payout_amount_minor: request.payout_amount_minor,
            bet_asset: request.bet_asset,
            bet_price_lower_bound: request.bet_price_lower_bound,
            bet_price_upper_bound: request.bet_price_upper_bound,
            bet_time: Utc::now(),
        })
        .await
        .is_ok()
    {
        "accepted"
    } else {
        "rejected"
    };

    Json(OrderResponse {
        order_id,
        status,
    })
}

pub async fn settlement_preview(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> Json<OrderResponse> {
    let status = match state.order_service().get_cached(order_id).await {
        Ok(Some(order)) => match order.status {
            crate::domain::order::OrderCacheStatus::Accepted => "accepted",
            crate::domain::order::OrderCacheStatus::Confirmed => "confirmed",
        },
        Ok(None) => "not_found",
        Err(_) => "error",
    };

    Json(OrderResponse { order_id, status })
}

pub async fn settle_order(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> Json<OrderResponse> {
    let _ = state.order_service();
    Json(OrderResponse {
        order_id,
        status: "queued",
    })
}

pub async fn settlement_batch_preview(State(state): State<AppState>) -> Json<SettlementBatchResult> {
    let _ = state.order_service();
    Json(SettlementBatchResult {
        observed_at: Utc::now(),
        observed_price: 0.0,
        scanned: 0,
        confirmed: 0,
        won: 0,
        lost: 0,
    })
}

pub async fn trigger_settlement(
    State(state): State<AppState>,
    Json(request): Json<TriggerSettlementRequest>,
) -> Json<SettlementBatchResult> {
    let result = state
        .order_service()
        .trigger_settlement(SettlementTrigger {
            observed_at: request.observed_at,
            observed_price: request.observed_price,
            limit: request.limit,
        })
        .await
        .unwrap_or(SettlementBatchResult {
            observed_at: request.observed_at,
            observed_price: request.observed_price,
            scanned: 0,
            confirmed: 0,
            won: 0,
            lost: 0,
        });

    Json(result)
}
