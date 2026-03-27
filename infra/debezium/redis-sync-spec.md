# Redis Sync Spec

This document defines the Redis projection for PostgreSQL CDC events emitted by Debezium.

## Source Topic

Primary topic after the connector transforms:

- `tap_trading.cdc.account_balances`

## Redis Key

Use one Redis string key per ledger balance snapshot.

- Key format: `ledger:balance:{user_id}:{asset}`

Example:

- `ledger:balance:6f5ef3e0-4f2f-4f4f-a1fe-1fdfd46a5e11:USDT`

## Value Shape

Store the latest full snapshot as JSON.

```json
{
  "user_id": "6f5ef3e0-4f2f-4f4f-a1fe-1fdfd46a5e11",
  "asset": "USDT",
  "account_version": 42,
  "locked_balance_minor": 1500000,
  "posted_balance_minor": 92500000
}
```

## Consumer Rules

1. Consume only `public.account_balances` CDC messages.
2. Treat the message payload as the source of truth snapshot.
3. On create/update (`op = c|u|r` before unwrap, or normal record after unwrap):
   write the full JSON snapshot to the Redis key.
4. On delete:
   either delete the Redis key or write a tombstone marker. Prefer delete.
5. Writes must be idempotent. The value is a full snapshot, not a partial patch.
6. If multiple events arrive out of order, compare `account_version` and keep the highest version.

## Operational Notes

- PostgreSQL remains the source of truth.
- Redis is a read cache only.
- If you enable Debezium-based sync, disable the app-side write-through cache to avoid two Redis writers.
