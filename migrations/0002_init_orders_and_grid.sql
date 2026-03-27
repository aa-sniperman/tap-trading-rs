CREATE TABLE IF NOT EXISTS grid_cells (
    grid_cell_id UUID PRIMARY KEY,
    column_start_at TIMESTAMPTZ NOT NULL,
    column_end_at TIMESTAMPTZ NOT NULL,
    price_low DOUBLE PRECISION NOT NULL,
    price_high DOUBLE PRECISION NOT NULL,
    reward_rate_bps INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS orders (
    order_id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(user_id),
    grid_cell_id UUID NOT NULL REFERENCES grid_cells(grid_cell_id),
    asset TEXT NOT NULL,
    stake_minor BIGINT NOT NULL,
    reward_rate_bps INTEGER NOT NULL,
    status TEXT NOT NULL,
    idempotency_key TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    settled_at TIMESTAMPTZ,
    UNIQUE (user_id, grid_cell_id),
    UNIQUE (idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_orders_user_created_at
    ON orders (user_id, created_at DESC);
