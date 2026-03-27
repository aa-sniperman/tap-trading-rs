# tap-trading-rs

Rust backend scaffold for Tap Trading phase 1.

## Scope from PRD

- Custodial single-entry ledger with append-only entries and balance control on Postgres.
- Real-time BTC price stream cache on Redis.
- Historical price retention for deterministic settlement on ClickHouse.
- Modules for grid, order placement and settlement, payment deposit and withdrawal sync.
- Simple Axum HTTP surface to anchor future handlers.

## Layout

- `src/api`: HTTP routes and transport DTOs.
- `src/app`: bootstrap, shared state, telemetry.
- `src/config`: typed config loading.
- `src/domain`: business modules.
- `src/infra/postgres`: ledger, order, payment repositories.
- `src/infra/redis`: price cache adapter.
- `src/infra/clickhouse`: price history adapter.
- `migrations`: Postgres schema plus ClickHouse DDL.

## Core modules

- `ledger`: append-only single-entry ledger, discard only for pending entries, and materialized `account_balances`.
  `economic_type` is constrained to `order_hold`, `settle_win`, `settle_lose`, `deposit`, `withdraw_hold`, `withdraw_confirm`.
  `ledger_entries` and `account_balances` keep `user_id` as a plain business key, without foreign key references to `users`.
  One account is queried by `(user_id, asset)`. Each account keeps an `account_version` starting at `0` and increments by `1` for every appended entry on that account. Entries are immutable.
- `price_stream`: latest price cache plus immutable tick history.
- `grid`: cell definition and placement rules from the PRD.
- `order`: placement and settlement boundary for user bets.
- `payment`: deposit and withdrawal intake, then sync into ledger.

## Run infra

```bash
docker compose up -d
```

For a local Redis Software target suitable for RDI development/testing:

```bash
docker compose --profile redis-software up -d redis-software
```

## Config

Defaults live in `config/default.toml`.
Override with env vars like:

```bash
APP__SERVER__BIND_ADDRESS=0.0.0.0:8080
APP__POSTGRES__URL=postgres://postgres:postgres@localhost:5432/tap_trading
APP__REDIS__URL=redis://127.0.0.1:6379
APP__CLICKHOUSE__URL=http://localhost:8123
```


## CDC Setup

Prepared infra artifacts for PostgreSQL logical replication and RDI live in `infra/`:

- `infra/postgres/postgresql-cdc.conf.example`
- `infra/postgres/00-replication-setup.sql`
- `infra/postgres/01-publication.sql`
- `infra/debezium/cdc-runbook.md`
- `infra/rdi/config.yaml.example`
- `infra/rdi/jobs/account_balances.yaml`
- `infra/rdi/rdi-runbook.md`

Flow:

1. PostgreSQL writes balance changes into WAL.
2. RDI/Debezium reads WAL via logical replication.
3. The change flows through the RDI pipeline.
4. RDI transforms the row and writes RedisJSON balance snapshots.

For RDI-backed balance cache, run the app with:

```bash
APP__REDIS__URL=redis://127.0.0.1:12000
APP__REDIS__BALANCE_CACHE_FORMAT=redis_json
```

## Run app

```bash
cargo run
```

## Next implementation steps

1. Replace repository stubs with transactional SQL, idempotent `economic_type/economic_key`, and balance projection updates.
2. Add Web3 auth challenge, signature verification, and replay protection.
3. Add a market data ingestor pushing ticks into Redis and ClickHouse.
4. Implement settlement worker reading price history for deterministic resolution.
5. Add integration tests around ledger integrity, duplicate order prevention, and payment sync.
