# Debezium CDC Runbook

## 1. Configure PostgreSQL

Apply PostgreSQL logical replication settings:

- `wal_level = logical`
- `max_replication_slots >= number_of_connectors`
- `max_wal_senders >= number_of_connectors`

Then run:

```sql
\i infra/postgres/00-replication-setup.sql
\i infra/postgres/01-publication.sql
```

## 2. Deploy Debezium Connector

Register the connector using:

- [account-balances-connector.json](./account-balances-connector.json)

Example with Kafka Connect:

```bash
curl -X POST http://localhost:8083/connectors \
  -H 'Content-Type: application/json' \
  --data @infra/debezium/account-balances-connector.json
```

## 3. Sync Kafka -> Redis

Choose one path:

1. Redis Data Integration (RDI)
2. Custom consumer

For a custom consumer, map topic `tap_trading.cdc.account_balances` using the rules in:

- [redis-sync-spec.md](./redis-sync-spec.md)

## 4. Verify

Check:

1. Replication slot exists
2. Publication contains `account_balances`
3. Kafka topic receives events
4. Redis key `ledger:balance:{user_id}:{asset}` updates after DB writes

Useful PostgreSQL checks:

```sql
SELECT * FROM pg_publication WHERE pubname = 'tap_trading_cdc';
SELECT * FROM pg_replication_slots WHERE slot_name = 'tap_trading_cdc_slot';
```

## 5. Important App Change

If CDC becomes the official Redis sync path, remove or disable the app-side write-through balance cache refresh to avoid dual writers.
