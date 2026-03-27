-- Run separately on ClickHouse.
CREATE TABLE IF NOT EXISTS price_ticks (
    symbol String,
    price Float64,
    observed_at DateTime64(3, 'UTC')
)
ENGINE = MergeTree
PARTITION BY toDate(observed_at)
ORDER BY (symbol, observed_at)
TTL observed_at + toIntervalDay(7);
