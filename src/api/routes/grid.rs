use axum::{extract::State, Json};
use serde::Serialize;

use crate::app::AppState;

#[derive(Serialize)]
pub struct GridSnapshot {
    instrument: &'static str,
    columns_ahead_locked: u8,
    placement_rule: &'static str,
}

pub async fn snapshot(State(state): State<AppState>) -> Json<GridSnapshot> {
    let _ = state.grid_service();

    Json(GridSnapshot {
        instrument: "BTC",
        columns_ahead_locked: 1,
        placement_rule: "past,current,and-next-price-columns-are-locked",
    })
}
