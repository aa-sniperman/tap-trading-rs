CREATE TABLE IF NOT EXISTS payments (
    payment_id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(user_id),
    kind TEXT NOT NULL,
    asset TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
    status TEXT NOT NULL,
    tx_hash TEXT,
    external_reference TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_payments_user_created_at
    ON payments (user_id, created_at DESC);
