# RDI CDC Runbook

## 1. Configure PostgreSQL

Apply PostgreSQL logical replication settings:

- `wal_level = logical`
- `max_replication_slots >= number_of_rdi_collectors`
- `max_wal_senders >= number_of_rdi_collectors`

Then run:

```sql
\i infra/postgres/00-replication-setup.sql
\i infra/postgres/01-publication.sql
```

## 2. Configure RDI Collector And Pipeline

Use the prepared files:

- `infra/rdi/config.yaml.example`
- `infra/rdi/jobs/account_balances.yaml`

RDI uses Debezium internally to read PostgreSQL logical replication and push the change stream through the RDI pipeline into Redis.

Key source settings are:

- `publication.name = tap_trading_cdc`
- `slot.name = tap_trading_cdc_slot`
- `plugin.name = pgoutput`

Key Redis mapping is:

- key: `ledger:balance:{user_id}:{asset}`
- value: RedisJSON snapshot of `account_balances`

## 3. Verify

Check:

1. Replication slot exists
2. Publication contains `account_balances`
3. RDI pipeline is healthy
4. Redis key `ledger:balance:{user_id}:{asset}` updates after DB writes

Useful PostgreSQL checks:

```sql
SELECT * FROM pg_publication WHERE pubname = 'tap_trading_cdc';
SELECT * FROM pg_replication_slots WHERE slot_name = 'tap_trading_cdc_slot';
```

## 4. Flow Summary

1. PostgreSQL writes row changes into WAL.
2. RDI/Debezium reads WAL using logical replication.
3. The change event flows through the RDI pipeline.
4. RDI transforms the row and writes the balance snapshot into Redis.
