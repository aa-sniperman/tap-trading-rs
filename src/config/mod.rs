use config::{Environment, File};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub postgres: PostgresSettings,
    pub redis: RedisSettings,
    pub clickhouse: ClickHouseSettings,
}

impl Settings {
    pub fn load() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?
            .try_deserialize()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    pub bind_address: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostgresSettings {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisSettings {
    pub url: String,
    pub balance_cache_format: RedisBalanceCacheFormat,
    pub balance_cache_sync_mode: RedisBalanceCacheSyncMode,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedisBalanceCacheFormat {
    PlainJsonString,
    RedisJson,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedisBalanceCacheSyncMode {
    WriteThrough,
    ReadOnly,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClickHouseSettings {
    pub url: String,
    pub database: String,
    pub username: String,
    pub password: String,
}
