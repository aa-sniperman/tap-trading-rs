CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'ledger_economic_type') THEN
        CREATE TYPE ledger_economic_type AS ENUM (
            'order_hold',
            'settle_win',
            'settle_lose',
            'deposit',
            'withdraw_hold',
            'withdraw_confirm',
            'withdraw_cancel'
        );
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS users (
    user_id UUID PRIMARY KEY,
    wallet_address TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS account_balances (
    user_id UUID NOT NULL,
    asset TEXT NOT NULL,
    account_version BIGINT NOT NULL DEFAULT 0,
    locked_balance_minor BIGINT NOT NULL DEFAULT 0,
    posted_balance_minor BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, asset)
);

CREATE INDEX IF NOT EXISTS idx_account_balances_user_asset
    ON account_balances (user_id, asset);

CREATE TABLE IF NOT EXISTS ledger_entries (
    entry_id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    account_version BIGINT NOT NULL,
    asset TEXT NOT NULL,
    delta_posted_minor BIGINT NOT NULL,
    delta_locked_minor BIGINT NOT NULL,
    economic_type ledger_economic_type NOT NULL,
    economic_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ledger_entries_user_asset_created_at
    ON ledger_entries (user_id, asset, created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS uq_ledger_entries_account_version
    ON ledger_entries (user_id, asset, account_version);

CREATE UNIQUE INDEX IF NOT EXISTS uq_ledger_entries_economic_type_key
    ON ledger_entries (economic_type, economic_key);
