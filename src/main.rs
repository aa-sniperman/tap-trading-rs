mod api;
mod app;
mod config;
mod domain;
mod infra;

use std::net::SocketAddr;

use anyhow::Context;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::telemetry::init()?;

    let settings = config::Settings::load().context("load settings")?;
    let state = app::build_state(&settings).await.context("build app state")?;
    let app = api::router(state);

    let addr: SocketAddr = settings.server.bind_address.parse().context("parse bind address")?;
    let listener = TcpListener::bind(addr).await.context("bind tcp listener")?;

    info!(address = %addr, "tap-trading backend started");

    axum::serve(listener, app)
        .with_graceful_shutdown(app::shutdown_signal())
        .await
        .context("run axum server")?;

    Ok(())
}
