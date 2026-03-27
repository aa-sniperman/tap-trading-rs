# Redis Data Integration Runbook

This setup assumes Kafka infrastructure already exists and PostgreSQL logical replication plus Debezium publication/slot have been prepared.

## Files

- `infra/rdi/config.yaml.example`
- `infra/rdi/jobs/account_balances.yaml`
- `infra/debezium/redis-sync-spec.md`

## What This RDI Setup Does

- Reads CDC from PostgreSQL using RDI's CDC source collector.
- Captures only `public.account_balances`.
- Writes each row as a Redis JSON document.
- Uses key format:
  - `ledger:balance:{user_id}:{asset}`

## Deployment Steps

1. Copy `infra/rdi/config.yaml.example` to your RDI deployment as `config.yaml`.
2. Mount or copy `infra/rdi/jobs/account_balances.yaml` into the RDI jobs directory.
3. Replace placeholder connection values as needed:
   - PostgreSQL host/user/password
   - Redis host/port
4. Start or reload the RDI pipeline.
5. Verify that updates to `account_balances` create/update Redis keys.
6. Run the Rust app with:
   - `APP__REDIS__BALANCE_CACHE_FORMAT=redis_json`
   - `APP__REDIS__BALANCE_CACHE_SYNC_MODE=read_only`

## Verification

After updating one row in `account_balances`, verify Redis contains:

- key: `ledger:balance:{user_id}:{asset}`
- JSON payload with:
  - `user_id`
  - `asset`
  - `account_version`
  - `locked_balance_minor`
  - `posted_balance_minor`

## Important Compatibility Note

This repo supports both balance cache formats:

1. `plain_json_string + write_through`: app writes and reads plain Redis string values.
2. `redis_json + read_only`: RDI writes RedisJSON documents and the app only reads them.

For this RDI setup, use path `2`.

## Operational Notes

- PostgreSQL remains the source of truth.
- Avoid running both app-side write-through cache sync and RDI at the same time.
- If out-of-order CDC delivery is a concern, compare `account_version` in consumers or downstream readers.
