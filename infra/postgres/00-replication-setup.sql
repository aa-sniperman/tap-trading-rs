-- Run as a superuser or an admin role on the source PostgreSQL instance.
-- This script creates a dedicated Debezium replication user.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'debezium') THEN
        CREATE ROLE debezium WITH LOGIN PASSWORD 'debezium';
    END IF;
END $$;

ALTER ROLE debezium WITH REPLICATION;

GRANT CONNECT ON DATABASE tap_trading TO debezium;
GRANT USAGE ON SCHEMA public TO debezium;
GRANT SELECT ON TABLE public.account_balances TO debezium;
GRANT SELECT ON TABLE public.ledger_entries TO debezium;
GRANT SELECT ON TABLE public.orders TO debezium;
GRANT SELECT ON TABLE public.payments TO debezium;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO debezium;
