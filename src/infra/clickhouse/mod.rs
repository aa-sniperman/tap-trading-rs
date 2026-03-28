pub mod order;
pub mod price_history;

use crate::config::ClickHouseSettings;

pub type ClickHouseClient = clickhouse::Client;

pub async fn connect(settings: &ClickHouseSettings) -> Result<ClickHouseClient, clickhouse::error::Error> {
    let client = clickhouse::Client::default()
        .with_url(&settings.url)
        .with_database(&settings.database)
        .with_user(&settings.username)
        .with_password(&settings.password);

    Ok(client)
}
