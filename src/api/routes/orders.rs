use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::AppState;

#[derive(Deserialize)]
pub struct PlaceOrderRequest {
    pub user_id: Uuid,
    pub grid_cell_id: Uuid,
    pub stake_minor: i64,
}

#[derive(Serialize)]
pub struct OrderResponse {
    pub status: &'static str,
}

pub async fn list_orders(State(state): State<AppState>) -> Json<Vec<OrderResponse>> {
    let _ = state.order_service();
    Json(vec![])
}

pub async fn place_order(
    State(state): State<AppState>,
    Json(_request): Json<PlaceOrderRequest>,
) -> Json<OrderResponse> {
    let _ = state.order_service();
    Json(OrderResponse { status: "accepted" })
}

pub async fn settlement_preview(
    State(state): State<AppState>,
    Path(_order_id): Path<Uuid>,
) -> Json<OrderResponse> {
    let _ = state.order_service();
    Json(OrderResponse { status: "pending" })
}

pub async fn settle_order(
    State(state): State<AppState>,
    Path(_order_id): Path<Uuid>,
) -> Json<OrderResponse> {
    let _ = state.order_service();
    Json(OrderResponse { status: "queued" })
}
