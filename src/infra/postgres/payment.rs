use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    common::UserId,
    payment::{Payment, PaymentError},
};

pub struct PaymentRepository {
    pool: PgPool,
}

impl PaymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl crate::domain::payment::PaymentRepository for PaymentRepository {
    async fn insert(&self, _payment: &Payment) -> Result<(), PaymentError> {
        let _ = &self.pool;
        Ok(())
    }

    async fn list_by_user(&self, _user_id: UserId) -> Result<Vec<Payment>, PaymentError> {
        let _ = &self.pool;
        Ok(vec![])
    }
}
