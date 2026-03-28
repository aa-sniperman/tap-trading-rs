# Redis Data Integration Runbook

This setup assumes PostgreSQL logical replication plus publication/slot have been prepared.

## Files

- `infra/rdi/config.yaml.example`
- `infra/rdi/jobs/account_balances.yaml`
- `infra/rdi/runtime/config.yaml`
- `infra/rdi/runtime/jobs/account_balances.yaml`
- `infra/rdi/package-pipeline.sh`
- `infra/debezium/cdc-runbook.md`

## What This RDI Setup Does

- Uses Debezium inside RDI to read PostgreSQL WAL through logical replication.
- Captures only `public.account_balances`.
- Writes each row as a Redis JSON document.
- Uses key format:
  - `ledger:balance:{user_id}:{asset}`

## Deployment Steps

This repo mirrors the RDI pipeline layout locally under:

- `infra/rdi/runtime/config.yaml`
- `infra/rdi/runtime/jobs/`

Redis documents the runtime pipeline path on the RDI host as:

- `/opt/rdi/config/config.yaml`
- `/opt/rdi/config/jobs/`

1. Edit the local runtime files in:
   - `infra/rdi/runtime/config.yaml`
   - `infra/rdi/runtime/jobs/account_balances.yaml`
2. Copy them to the RDI host:
   - `infra/rdi/runtime/config.yaml -> /opt/rdi/config/config.yaml`
   - `infra/rdi/runtime/jobs/account_balances.yaml -> /opt/rdi/config/jobs/account_balances.yaml`
3. Replace placeholder connection values as needed:
   - PostgreSQL host/user/password
   - Redis host/port
4. Start or reload the RDI pipeline.
5. Verify that updates to `account_balances` create/update Redis keys.
6. Run the Rust app with:
   - `APP__REDIS__URL=redis://127.0.0.1:12000`
   - `APP__REDIS__BALANCE_CACHE_FORMAT=redis_json`

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

1. `plain_json_string`: app reads plain Redis string values from an external writer.
2. `redis_json`: RDI writes RedisJSON documents and the app reads them.

For this RDI setup, use path `2`.

## Operational Notes

- PostgreSQL remains the source of truth.
- The app does not write balance cache entries anymore.
- If out-of-order CDC delivery is a concern, compare `account_version` in consumers or downstream readers.
- `infra/rdi/package-pipeline.sh` packages the local runtime layout into `infra/rdi/pipeline.zip` if you prefer to move one artifact.
