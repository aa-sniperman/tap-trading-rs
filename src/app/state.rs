use std::sync::Arc;

use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::{
    config::Settings,
    domain::{
        grid::GridService,
        ledger::LedgerService,
        order::OrderService,
        payment::PaymentService,
        price_stream::PriceStreamService,
    },
    infra::clickhouse::ClickHouseClient,
};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppServices>,
}

impl AppState {
    pub fn new(services: AppServices) -> Self {
        Self {
            inner: Arc::new(services),
        }
    }

    pub fn ledger_service(&self) -> &LedgerService {
        &self.inner.ledger
    }

    pub fn price_stream_service(&self) -> &PriceStreamService {
        &self.inner.price_stream
    }

    pub fn grid_service(&self) -> &GridService {
        &self.inner.grid
    }

    pub fn order_service(&self) -> &OrderService {
        &self.inner.order
    }

    pub fn payment_service(&self) -> &PaymentService {
        &self.inner.payment
    }
}

pub struct AppServices {
    #[allow(dead_code)]
    pub settings: Settings,
    #[allow(dead_code)]
    pub postgres: PgPool,
    #[allow(dead_code)]
    pub redis: ConnectionManager,
    #[allow(dead_code)]
    pub clickhouse: ClickHouseClient,
    pub ledger: LedgerService,
    pub price_stream: PriceStreamService,
    pub grid: GridService,
    pub order: OrderService,
    pub payment: PaymentService,
}

impl AppServices {
    pub fn new(
        settings: Settings,
        postgres: PgPool,
        redis: ConnectionManager,
        clickhouse: ClickHouseClient,
    ) -> Self {
        let balance_cache = crate::infra::redis::balance_cache::BalanceCache::new(
            redis.clone(),
            settings.redis.balance_cache_format.clone(),
        );
        let ledger_repo = crate::infra::postgres::ledger::LedgerRepository::new(postgres.clone(), balance_cache);
        let order_events = crate::infra::clickhouse::order::OrderEventRepositoryImpl::new(clickhouse.clone());
        let order_cache = crate::infra::redis::order_cache::OrderCache::new(redis.clone());
        let payment_repo = crate::infra::postgres::payment::PaymentRepository::new(postgres.clone());
        let price_cache = crate::infra::redis::price_stream::PriceCache::new(redis.clone());
        let price_history = crate::infra::clickhouse::price_history::PriceHistoryRepository::new(clickhouse.clone());

        let ledger = LedgerService::new(ledger_repo);
        let price_stream = PriceStreamService::new(price_cache, price_history);
        let grid = GridService::new();
        let order = OrderService::new(
            order_events,
            order_cache,
            crate::domain::order::NoopOrderFanout,
            ledger.clone(),
        );
        let payment = PaymentService::new(payment_repo, ledger.clone());

        Self {
            settings,
            postgres,
            redis,
            clickhouse,
            ledger,
            price_stream,
            grid,
            order,
            payment,
        }
    }
}
