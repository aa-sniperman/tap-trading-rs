-- Run on the source PostgreSQL database after the ledger schema exists.
-- Restrict the publication to the tables that should be CDC-synced.

DROP PUBLICATION IF EXISTS tap_trading_cdc;

CREATE PUBLICATION tap_trading_cdc
FOR TABLE
    public.account_balances,
    public.ledger_entries,
    public.orders,
    public.payments;
