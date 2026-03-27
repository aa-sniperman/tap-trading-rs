pub mod routes;

use axum::{routing::get, Router};

use crate::app::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(routes::health::health_check))
        .route("/v1/prices/stream", get(routes::prices::stream_status))
        .route("/v1/grid/snapshot", get(routes::grid::snapshot))
        .route("/v1/orders", get(routes::orders::list_orders).post(routes::orders::place_order))
        .route("/v1/orders/:order_id/settle", get(routes::orders::settlement_preview).post(routes::orders::settle_order))
        .route("/v1/payments/deposits", get(routes::payments::list_deposits).post(routes::payments::create_deposit))
        .route("/v1/payments/withdrawals", get(routes::payments::list_withdrawals).post(routes::payments::create_withdrawal))
        .with_state(state)
}
