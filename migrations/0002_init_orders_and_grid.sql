CREATE TABLE IF NOT EXISTS grid_cells (
    grid_cell_id UUID PRIMARY KEY,
    column_start_at TIMESTAMPTZ NOT NULL,
    column_end_at TIMESTAMPTZ NOT NULL,
    price_low DOUBLE PRECISION NOT NULL,
    price_high DOUBLE PRECISION NOT NULL,
    reward_rate_bps INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
