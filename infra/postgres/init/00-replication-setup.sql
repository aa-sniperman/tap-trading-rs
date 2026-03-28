-- Docker init script for local CDC development.
-- Runs once on first database initialization.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'debezium') THEN
        CREATE ROLE debezium WITH LOGIN PASSWORD 'debezium';
    END IF;
END $$;

ALTER ROLE debezium WITH REPLICATION;

GRANT CONNECT ON DATABASE tap_trading TO debezium;
