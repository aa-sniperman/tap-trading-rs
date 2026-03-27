use async_trait::async_trait;
use sqlx::{PgPool, Row};
use tracing::warn;

use crate::{
    domain::{
        common::UserId,
        ledger::{AccountBalance, EconomicType, LedgerEntry, LedgerError},
    },
    infra::redis::balance_cache::BalanceCache,
};

pub struct LedgerRepository {
    pool: PgPool,
    balance_cache: BalanceCache,
}

impl LedgerRepository {
    pub fn new(pool: PgPool, balance_cache: BalanceCache) -> Self {
        Self {
            pool,
            balance_cache,
        }
    }

    async fn append_entry(
        &self,
        entry: &LedgerEntry,
        authorize_mode: bool,
    ) -> Result<(), LedgerError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| LedgerError::Repository(error.to_string()))?;

        let next_version = if authorize_mode {
            self.bump_balances_authorizing_mode(&mut tx, entry).await?
        } else {
            self.bump_balances_recording_mode(&mut tx, entry).await?
        };

        sqlx::query(
            r#"
            INSERT INTO ledger_entries (
                entry_id,
                user_id,
                account_version,
                asset,
                delta_posted_minor,
                delta_locked_minor,
                economic_type,
                economic_key
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(entry.entry_id)
        .bind(entry.user_id)
        .bind(next_version)
        .bind(&entry.asset)
        .bind(entry.delta_posted_minor)
        .bind(entry.delta_locked_minor)
        .bind(economic_type_to_db(entry.economic_type.clone()))
        .bind(&entry.economic_key)
        .execute(&mut *tx)
        .await
        .map_err(|error| LedgerError::Repository(error.to_string()))?;

        tx.commit()
            .await
            .map_err(|error| LedgerError::Repository(error.to_string()))?;

        self.sync_balance_cache(entry.user_id, &entry.asset).await;

        Ok(())
    }

    async fn bump_balances_recording_mode(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry: &LedgerEntry,
    ) -> Result<i64, LedgerError> {
        let maybe_row = sqlx::query(
            r#"
            UPDATE account_balances
            SET
                account_version = account_version + 1,
                posted_balance_minor = posted_balance_minor + $3,
                locked_balance_minor = locked_balance_minor + $4,
                updated_at = NOW()
            WHERE user_id = $1 AND asset = $2
            RETURNING account_version
            "#,
        )
        .bind(entry.user_id)
        .bind(&entry.asset)
        .bind(entry.delta_posted_minor)
        .bind(entry.delta_locked_minor)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| LedgerError::Repository(error.to_string()))?;

        let row = maybe_row.ok_or_else(|| {
            LedgerError::Repository("account balance row not found for record update".to_owned())
        })?;

        row.try_get("account_version")
            .map_err(|error| LedgerError::Repository(error.to_string()))
    }

    async fn bump_balances_authorizing_mode(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry: &LedgerEntry,
    ) -> Result<i64, LedgerError> {
        let should_lock_available = entry.delta_posted_minor < 0;

        let maybe_row = if should_lock_available {
            sqlx::query(
                r#"
                UPDATE account_balances
                SET
                    account_version = account_version + 1,
                    posted_balance_minor = posted_balance_minor + $3,
                    locked_balance_minor = locked_balance_minor + $4,
                    updated_at = NOW()
                WHERE user_id = $1
                  AND asset = $2
                  AND posted_balance_minor + $3 >= 0
                RETURNING account_version
                "#,
            )
            .bind(entry.user_id)
            .bind(&entry.asset)
            .bind(entry.delta_posted_minor)
            .bind(entry.delta_locked_minor)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|error| LedgerError::Repository(error.to_string()))?
        } else {
            sqlx::query(
                r#"
                UPDATE account_balances
                SET
                    account_version = account_version + 1,
                    posted_balance_minor = posted_balance_minor + $3,
                    locked_balance_minor = locked_balance_minor + $4,
                    updated_at = NOW()
                WHERE user_id = $1 AND asset = $2
                RETURNING account_version
                "#,
            )
            .bind(entry.user_id)
            .bind(&entry.asset)
            .bind(entry.delta_posted_minor)
            .bind(entry.delta_locked_minor)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|error| LedgerError::Repository(error.to_string()))?
        };

        let row = maybe_row.ok_or_else(|| {
            if should_lock_available {
                LedgerError::InsufficientBalance
            } else {
                LedgerError::Repository(
                    "account balance row not found for authorize update".to_owned(),
                )
            }
        })?;

        row.try_get("account_version")
            .map_err(|error| LedgerError::Repository(error.to_string()))
    }

    async fn fetch_balance_from_db(
        &self,
        user_id: UserId,
        asset: &str,
    ) -> Result<Option<AccountBalance>, LedgerError> {
        let row = sqlx::query(
            r#"
            SELECT
                user_id,
                asset,
                account_version,
                locked_balance_minor,
                posted_balance_minor
            FROM account_balances
            WHERE user_id = $1 AND asset = $2
            "#,
        )
        .bind(user_id)
        .bind(asset)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| LedgerError::Repository(error.to_string()))?;

        row.map(|row| {
            Ok(AccountBalance {
                user_id: row
                    .try_get("user_id")
                    .map_err(|error| LedgerError::Repository(error.to_string()))?,
                asset: row
                    .try_get("asset")
                    .map_err(|error| LedgerError::Repository(error.to_string()))?,
                account_version: row
                    .try_get("account_version")
                    .map_err(|error| LedgerError::Repository(error.to_string()))?,
                locked_balance_minor: row
                    .try_get("locked_balance_minor")
                    .map_err(|error| LedgerError::Repository(error.to_string()))?,
                posted_balance_minor: row
                    .try_get("posted_balance_minor")
                    .map_err(|error| LedgerError::Repository(error.to_string()))?,
            })
        })
        .transpose()
    }

    async fn sync_balance_cache(&self, user_id: UserId, asset: &str) {
        if !self.balance_cache.write_through_enabled() {
            return;
        }

        match self.fetch_balance_from_db(user_id, asset).await {
            Ok(Some(balance)) => {
                if let Err(error) = self.balance_cache.set_balance(&balance).await {
                    warn!(user_id = %user_id, asset, error = %error, "failed to refresh balance cache after db commit");
                }
            }
            Ok(None) => {
                warn!(user_id = %user_id, asset, "balance row missing after db commit; cache not updated");
            }
            Err(error) => {
                warn!(user_id = %user_id, asset, error = %error, "failed to fetch balance for cache refresh");
            }
        }
    }
}

#[async_trait]
impl crate::domain::ledger::LedgerRepository for LedgerRepository {
    async fn ensure_account_balance(&self, user_id: UserId, asset: &str) -> Result<(), LedgerError> {
        sqlx::query(
            r#"
            INSERT INTO account_balances (
                user_id,
                asset,
                account_version,
                locked_balance_minor,
                posted_balance_minor
            )
            VALUES ($1, $2, 0, 0, 0)
            ON CONFLICT (user_id, asset) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(asset)
        .execute(&self.pool)
        .await
        .map_err(|error| LedgerError::Repository(error.to_string()))?;

        self.sync_balance_cache(user_id, asset).await;

        Ok(())
    }

    async fn record_entry(&self, entry: &LedgerEntry) -> Result<(), LedgerError> {
        self.append_entry(entry, false).await
    }

    async fn authorize_entry(&self, entry: &LedgerEntry) -> Result<(), LedgerError> {
        self.append_entry(entry, true).await
    }

    async fn get_balance(&self, user_id: UserId, asset: &str) -> Result<AccountBalance, LedgerError> {
        if let Some(balance) = self.balance_cache.get_balance(user_id, asset).await? {
            return Ok(balance);
        }

        match self.fetch_balance_from_db(user_id, asset).await? {
            Some(balance) => {
                if self.balance_cache.write_through_enabled() {
                    if let Err(error) = self.balance_cache.set_balance(&balance).await {
                        warn!(user_id = %user_id, asset, error = %error, "failed to fill balance cache from db read");
                    }
                }
                Ok(balance)
            }
            None => Ok(AccountBalance {
                user_id,
                asset: asset.to_owned(),
                account_version: 0,
                locked_balance_minor: 0,
                posted_balance_minor: 0,
            }),
        }
    }
}

fn economic_type_to_db(value: EconomicType) -> &'static str {
    match value {
        EconomicType::OrderHold => "order_hold",
        EconomicType::SettleWin => "settle_win",
        EconomicType::SettleLose => "settle_lose",
        EconomicType::Deposit => "deposit",
        EconomicType::WithdrawHold => "withdraw_hold",
        EconomicType::WithdrawConfirm => "withdraw_confirm",
        EconomicType::WithdrawCancel => "withdraw_cancel",
    }
}
