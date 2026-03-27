use axum::{extract::State, Json};
use serde::Serialize;

use crate::app::AppState;

#[derive(Serialize)]
pub struct PriceStreamStatus {
    stream_key: &'static str,
    status: &'static str,
    retention_days: u8,
}

pub async fn stream_status(State(state): State<AppState>) -> Json<PriceStreamStatus> {
    let _ = state.price_stream_service();

    Json(PriceStreamStatus {
        stream_key: "btc-usdt",
        status: "configured",
        retention_days: 7,
    })
}
