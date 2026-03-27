pub mod state;
pub mod telemetry;

use anyhow::Context;
pub use state::AppState;
use state::AppServices;

use crate::{config::Settings, infra};

pub async fn build_state(settings: &Settings) -> anyhow::Result<AppState> {
    let postgres = infra::postgres::connect(&settings.postgres)
        .await
        .context("connect postgres")?;
    let redis = infra::redis::connect(&settings.redis)
        .await
        .context("connect redis")?;
    let clickhouse = infra::clickhouse::connect(&settings.clickhouse)
        .await
        .context("connect clickhouse")?;

    let services = AppServices::new(settings.clone(), postgres, redis, clickhouse);
    Ok(AppState::new(services))
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        signal(SignalKind::terminate())
            .expect("install terminate handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
