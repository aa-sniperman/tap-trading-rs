pub mod balance_cache;
pub mod price_stream;

use redis::aio::ConnectionManager;

use crate::config::RedisSettings;

pub async fn connect(settings: &RedisSettings) -> Result<ConnectionManager, redis::RedisError> {
    let client = redis::Client::open(settings.url.as_str())?;
    client.get_connection_manager().await
}
