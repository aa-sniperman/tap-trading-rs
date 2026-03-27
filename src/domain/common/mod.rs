use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type UserId = Uuid;
pub type OrderId = Uuid;
pub type PaymentId = Uuid;
pub type GridCellId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Money {
    pub asset: String,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditMetadata {
    pub request_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}
