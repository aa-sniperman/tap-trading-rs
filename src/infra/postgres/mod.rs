pub mod ledger;
pub mod payment;

use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::config::PostgresSettings;

pub async fn connect(settings: &PostgresSettings) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .connect(&settings.url)
        .await
}
