use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::AppState;

#[derive(Deserialize)]
pub struct PaymentRequest {
    pub user_id: Uuid,
    pub asset: String,
    pub amount_minor: i64,
}

#[derive(Serialize)]
pub struct PaymentResponse {
    pub status: &'static str,
}

pub async fn list_deposits(State(state): State<AppState>) -> Json<Vec<PaymentResponse>> {
    let _ = state.payment_service();
    Json(vec![])
}

pub async fn create_deposit(
    State(state): State<AppState>,
    Json(_request): Json<PaymentRequest>,
) -> Json<PaymentResponse> {
    let _ = state.payment_service();
    Json(PaymentResponse { status: "accepted" })
}

pub async fn list_withdrawals(State(state): State<AppState>) -> Json<Vec<PaymentResponse>> {
    let _ = state.payment_service();
    Json(vec![])
}

pub async fn create_withdrawal(
    State(state): State<AppState>,
    Json(_request): Json<PaymentRequest>,
) -> Json<PaymentResponse> {
    let _ = state.payment_service();
    Json(PaymentResponse { status: "accepted" })
}
