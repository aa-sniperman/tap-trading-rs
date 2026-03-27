use axum::{extract::State, Json};
use serde::Serialize;

use crate::app::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

pub async fn health_check(State(_state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "tap-trading-api",
    })
}
