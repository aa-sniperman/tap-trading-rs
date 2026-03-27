use anyhow::Context;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tap_trading_rs=debug"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .try_init()
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("initialize tracing subscriber")?;

    Ok(())
}
