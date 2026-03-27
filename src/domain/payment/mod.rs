use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{
    common::{Money, PaymentId, UserId},
    ledger::LedgerService,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub payment_id: PaymentId,
    pub user_id: UserId,
    pub kind: PaymentKind,
    pub amount: Money,
    pub status: PaymentStatus,
    pub external_reference: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentKind {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentStatus {
    Pending,
    Confirmed,
    Failed,
}

#[derive(Debug, Error)]
pub enum PaymentError {
    #[error("repository error: {0}")]
    Repository(String),
}

#[async_trait]
pub trait PaymentRepository: Send + Sync {
    async fn insert(&self, payment: &Payment) -> Result<(), PaymentError>;
    async fn list_by_user(&self, user_id: UserId) -> Result<Vec<Payment>, PaymentError>;
}

#[derive(Clone)]
pub struct PaymentService {
    repository: Arc<dyn PaymentRepository>,
    #[allow(dead_code)]
    ledger: LedgerService,
}

impl PaymentService {
    pub fn new(repository: impl PaymentRepository + 'static, ledger: LedgerService) -> Self {
        Self {
            repository: Arc::new(repository),
            ledger,
        }
    }

    pub async fn create(&self, payment: &Payment) -> Result<(), PaymentError> {
        self.repository.insert(payment).await
    }

    pub async fn list_by_user(&self, user_id: UserId) -> Result<Vec<Payment>, PaymentError> {
        self.repository.list_by_user(user_id).await
    }
}
