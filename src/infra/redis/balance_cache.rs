use redis::{aio::ConnectionManager, AsyncCommands};

use crate::{
    config::RedisBalanceCacheFormat,
    domain::ledger::{AccountBalance, LedgerError},
};

#[derive(Clone)]
pub struct BalanceCache {
    connection_manager: ConnectionManager,
    format: RedisBalanceCacheFormat,
}

impl BalanceCache {
    pub fn new(connection_manager: ConnectionManager, format: RedisBalanceCacheFormat) -> Self {
        Self {
            connection_manager,
            format,
        }
    }

    pub async fn get_balance(
        &self,
        user_id: uuid::Uuid,
        asset: &str,
    ) -> Result<Option<AccountBalance>, LedgerError> {
        let mut connection = self.connection_manager.clone();
        let key = Self::key(user_id, asset);
        let payload: Option<String> = match self.format {
            RedisBalanceCacheFormat::PlainJsonString => connection
                .get(key)
                .await
                .map_err(|error| LedgerError::Repository(error.to_string()))?,
            RedisBalanceCacheFormat::RedisJson => redis::cmd("JSON.GET")
                .arg(key)
                .query_async(&mut connection)
                .await
                .map_err(|error| LedgerError::Repository(error.to_string()))?,
        };

        payload
            .map(|value| serde_json::from_str::<AccountBalance>(&value))
            .transpose()
            .map_err(|error| LedgerError::Repository(error.to_string()))
    }

    pub async fn set_balance(&self, balance: &AccountBalance) -> Result<(), LedgerError> {
        let mut connection = self.connection_manager.clone();
        let key = Self::key(balance.user_id, &balance.asset);
        let payload = serde_json::to_string(balance)
            .map_err(|error| LedgerError::Repository(error.to_string()))?;

        match self.format {
            RedisBalanceCacheFormat::PlainJsonString => connection
                .set(key, payload)
                .await
                .map_err(|error| LedgerError::Repository(error.to_string())),
            RedisBalanceCacheFormat::RedisJson => redis::cmd("JSON.SET")
                .arg(key)
                .arg("$")
                .arg(payload)
                .query_async(&mut connection)
                .await
                .map_err(|error| LedgerError::Repository(error.to_string())),
        }
    }

    fn key(user_id: uuid::Uuid, asset: &str) -> String {
        format!("ledger:balance:{}:{}", user_id, asset)
    }
}
