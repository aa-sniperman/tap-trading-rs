-- Run separately on ClickHouse.
CREATE TABLE IF NOT EXISTS order_events (
    event_id UUID,
    order_id UUID,
    user_id UUID,
    bet_amount_minor Int64,
    payout_amount_minor Int64,
    bet_asset String,
    bet_price_lower_bound Float64,
    bet_price_upper_bound Float64,
    bet_time DateTime64(3, 'UTC'),
    event_type String,
    reason Nullable(String),
    created_at DateTime64(3, 'UTC')
)
ENGINE = MergeTree
PARTITION BY toDate(created_at)
ORDER BY (order_id, created_at, event_id);
