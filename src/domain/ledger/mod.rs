use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::common::{Money, UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub entry_id: Uuid,
    pub user_id: UserId,
    pub account_version: i64,
    pub asset: String,
    pub delta_posted_minor: i64,
    pub delta_locked_minor: i64,
    pub economic_type: EconomicType,
    pub economic_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositParams {
    pub user_id: UserId,
    pub amount: Money,
    pub tx_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawHoldParams {
    pub user_id: UserId,
    pub amount: Money,
    pub withdrawal_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawConfirmParams {
    pub user_id: UserId,
    pub amount: Money,
    pub withdrawal_id: Uuid,
    pub tx_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawCancelParams {
    pub user_id: UserId,
    pub amount: Money,
    pub withdrawal_id: Uuid,
    pub tx_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderHoldParams {
    pub user_id: UserId,
    pub amount: Money,
    pub order_id: Uuid,
    pub grid_cell_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSettleWinParams {
    pub user_id: UserId,
    pub bet_amount: Money,
    pub payout_amount: Money,
    pub order_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSettleLoseParams {
    pub user_id: UserId,
    pub bet_amount: Money,
    pub order_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EconomicType {
    OrderHold,
    SettleWin,
    SettleLose,
    Deposit,
    WithdrawHold,
    WithdrawConfirm,
    WithdrawCancel,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub user_id: UserId,
    pub asset: String,
    pub account_version: i64,
    pub locked_balance_minor: i64,
    pub posted_balance_minor: i64,
}

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("insufficient available balance")]
    InsufficientBalance,
    #[error("repository error: {0}")]
    Repository(String),
}

#[async_trait]
pub trait LedgerRepository: Send + Sync {
    async fn ensure_account_balance(&self, user_id: UserId, asset: &str)
        -> Result<(), LedgerError>;
    async fn record_entry(&self, entry: &LedgerEntry) -> Result<(), LedgerError>;
    async fn authorize_entry(&self, entry: &LedgerEntry) -> Result<(), LedgerError>;
    async fn get_balance(
        &self,
        user_id: UserId,
        asset: &str,
    ) -> Result<AccountBalance, LedgerError>;
}

#[derive(Clone)]
pub struct LedgerService {
    repository: Arc<dyn LedgerRepository>,
}

impl LedgerService {
    pub fn new(repository: impl LedgerRepository + 'static) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }

    pub async fn reserve_funds(
        &self,
        user_id: UserId,
        amount: Money,
    ) -> Result<AccountBalance, LedgerError> {
        self.repository.get_balance(user_id, &amount.asset).await
    }

    pub async fn ensure_account_balance(
        &self,
        user_id: UserId,
        asset: &str,
    ) -> Result<(), LedgerError> {
        self.repository.ensure_account_balance(user_id, asset).await
    }

    pub async fn settle_entry(&self, entry: &LedgerEntry) -> Result<(), LedgerError> {
        self.repository.record_entry(entry).await
    }

    pub async fn deposit(&self, params: DepositParams) -> Result<(), LedgerError> {
        let economic_key = Self::build_deposit_key(&params);
        let entry = LedgerEntry {
            entry_id: Uuid::new_v4(),
            user_id: params.user_id,
            account_version: 0,
            asset: params.amount.asset,
            delta_posted_minor: params.amount.amount_minor,
            delta_locked_minor: 0,
            economic_type: EconomicType::Deposit,
            economic_key,
        };

        self.repository.record_entry(&entry).await
    }

    pub async fn withdraw_hold(&self, params: WithdrawHoldParams) -> Result<(), LedgerError> {
        let economic_key = Self::build_withdraw_hold_key(&params);
        let entry = LedgerEntry {
            entry_id: Uuid::new_v4(),
            user_id: params.user_id,
            account_version: 0,
            asset: params.amount.asset,
            delta_posted_minor: -params.amount.amount_minor,
            delta_locked_minor: params.amount.amount_minor,
            economic_type: EconomicType::WithdrawHold,
            economic_key,
        };

        self.repository.authorize_entry(&entry).await
    }

    pub async fn withdraw_cancel(&self, params: WithdrawCancelParams) -> Result<(), LedgerError> {
        let economic_key = Self::build_withdraw_cancel_key(&params);
        let entry = LedgerEntry {
            entry_id: Uuid::new_v4(),
            user_id: params.user_id,
            account_version: 0,
            asset: params.amount.asset,
            delta_posted_minor: params.amount.amount_minor,
            delta_locked_minor: -params.amount.amount_minor,
            economic_key: economic_key,
            economic_type: EconomicType::WithdrawCancel,
        };

        self.repository.record_entry(&entry).await
    }

    pub async fn withdraw_confirm(&self, params: WithdrawConfirmParams) -> Result<(), LedgerError> {
        let economic_key = Self::build_withdraw_confirm_key(&params);
        let entry = LedgerEntry {
            entry_id: Uuid::new_v4(),
            user_id: params.user_id,
            account_version: 0,
            asset: params.amount.asset,
            delta_posted_minor: 0,
            delta_locked_minor: -params.amount.amount_minor,
            economic_type: EconomicType::WithdrawConfirm,
            economic_key,
        };

        self.repository.record_entry(&entry).await
    }

    pub async fn order_hold(&self, params: OrderHoldParams) -> Result<(), LedgerError> {
        let economic_key = Self::build_order_hold_key(&params);
        let entry = LedgerEntry {
            entry_id: Uuid::new_v4(),
            user_id: params.user_id,
            account_version: 0,
            asset: params.amount.asset,
            delta_posted_minor: -params.amount.amount_minor,
            delta_locked_minor: params.amount.amount_minor,
            economic_type: EconomicType::OrderHold,
            economic_key,
        };

        self.repository.authorize_entry(&entry).await
    }

    pub async fn order_settle_win(&self, params: OrderSettleWinParams) -> Result<(), LedgerError> {
        let economic_key = Self::build_order_settle_win_key(&params);
        let entry = LedgerEntry {
            entry_id: Uuid::new_v4(),
            user_id: params.user_id,
            account_version: 0,
            asset: params.bet_amount.asset,
            delta_posted_minor: params.payout_amount.amount_minor,
            delta_locked_minor: -params.bet_amount.amount_minor,
            economic_type: EconomicType::SettleWin,
            economic_key,
        };

        self.repository.record_entry(&entry).await
    }

    pub async fn order_settle_lose(
        &self,
        params: OrderSettleLoseParams,
    ) -> Result<(), LedgerError> {
        let economic_key = Self::build_order_settle_lose_key(&params);
        let entry = LedgerEntry {
            entry_id: Uuid::new_v4(),
            user_id: params.user_id,
            account_version: 0,
            asset: params.bet_amount.asset,
            delta_posted_minor: 0,
            delta_locked_minor: -params.bet_amount.amount_minor,
            economic_type: EconomicType::SettleLose,
            economic_key,
        };

        self.repository.record_entry(&entry).await
    }


    fn build_deposit_key(params: &DepositParams) -> String {
        format!("deposit:{}", params.tx_hash)
    }

    fn build_withdraw_hold_key(params: &WithdrawHoldParams) -> String {
        format!("withdraw_hold:{}", params.withdrawal_id)
    }

    fn build_withdraw_confirm_key(params: &WithdrawConfirmParams) -> String {
        format!(
            "withdraw_confirm:{}:{}",
            params.withdrawal_id, params.tx_hash
        )
    }

    fn build_withdraw_cancel_key(params: &WithdrawCancelParams) -> String {
        format!("withdraw_cancel:{}", params.withdrawal_id)
    }

    fn build_order_hold_key(params: &OrderHoldParams) -> String {
        format!("order_hold:{}:{}", params.order_id, params.grid_cell_id)
    }

    fn build_order_settle_win_key(params: &OrderSettleWinParams) -> String {
        format!("settle_win:{}", params.order_id)
    }

    fn build_order_settle_lose_key(params: &OrderSettleLoseParams) -> String {
        format!("settle_lose:{}", params.order_id)
    }
}
