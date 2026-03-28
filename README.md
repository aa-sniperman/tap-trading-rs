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
- `src/infra/postgres`: ledger and payment repositories.
- `src/infra/redis`: price cache adapter.
- `src/infra/clickhouse`: price history and order adapters.
- `migrations`: Postgres schema plus ClickHouse DDL.

## Core modules

- `ledger`: append-only single-entry ledger, discard only for pending entries, and materialized `account_balances`.
  `economic_type` is constrained to `order_hold`, `settle_win`, `settle_lose`, `deposit`, `withdraw_hold`, `withdraw_confirm`.
  `ledger_entries` and `account_balances` keep `user_id` as a plain business key, without foreign key references to `users`.
  One account is queried by `(user_id, asset)`. Each account keeps an `account_version` starting at `0` and increments by `1` for every appended entry on that account. Entries are immutable.
- `price_stream`: latest price cache plus immutable tick history.
- `grid`: cell definition and placement rules from the PRD.
- `order`: minimal bet saga for fast placement experiments.
  The accepted order intent is intentionally small: `order_id`, `user_id`, `bet_amount_minor`, `bet_asset`, `bet_time`.
  Order lifecycle is append-only in ClickHouse `order_events` with event types `placed`, `confirmed`, `reverted`, while Redis keeps the active cache view for fast fanout/read-after-write.
- `payment`: deposit and withdrawal intake, then sync into ledger.

## Run infra

```bash
docker compose up -d
```

The local `postgres` container is started with CDC settings enabled:

- `wal_level=logical`
- `max_replication_slots=8`
- `max_wal_senders=8`
- `max_slot_wal_keep_size=4096`
- `wal_sender_timeout=60s`

It also runs [00-replication-setup.sql](/Users/sniperman/code/tap-trading-rs/infra/postgres/init/00-replication-setup.sql) on first init to create the `debezium` replication user for local RDI development.

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

## Balance cache

The current app path uses Redis as a write-through balance cache:

1. The ledger transaction commits in PostgreSQL.
2. The repository reads the latest `account_balances` snapshot.
3. The snapshot is written into Redis under `ledger:balance:{user_id}:{asset}`.
4. Later balance reads hit Redis first and fall back to PostgreSQL on cache miss.

Default local config uses:

```bash
APP__REDIS__URL=redis://127.0.0.1:6379
APP__REDIS__BALANCE_CACHE_FORMAT=plain_json_string
```

## CDC Setup

CDC/RDI artifacts are kept in `infra/` as awareness for a future ops rollout:

- `infra/postgres/postgresql-cdc.conf.example`
- `infra/postgres/00-replication-setup.sql`
- `infra/postgres/01-publication.sql`
- `infra/debezium/cdc-runbook.md`
- `infra/rdi/config.yaml.example`
- `infra/rdi/jobs/account_balances.yaml`
- `infra/rdi/runtime/config.yaml`
- `infra/rdi/runtime/jobs/account_balances.yaml`
- `infra/rdi/rdi-runbook.md`

If you revisit CDC later, after the application schema and migrations exist, apply the publication:

```bash
psql postgres://postgres:postgres@localhost:5432/tap_trading -f infra/postgres/01-publication.sql
```

If you revisit an RDI-backed cache later, run the app with:

```bash
APP__REDIS__URL=redis://127.0.0.1:12000
APP__REDIS__BALANCE_CACHE_FORMAT=redis_json
```

RDI deployment is not part of this `docker-compose` file. The repo only prepares awareness artifacts and a local pipeline layout under `infra/rdi/runtime/`.

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
