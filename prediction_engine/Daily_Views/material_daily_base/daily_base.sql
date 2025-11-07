--------------------------------
-- Materialized View
-------------------------------

-- 🛠️ Layer 1: Base Daily OHLC Aggregation
-- This should be a standard Materialized View (MV) refreshed frequently.

DROP  MATERIALIZED VIEW daily_base;

CREATE MATERIALIZED VIEW daily_base AS
WITH base AS (
  -- 1. Original Base: Assign session and trading date to raw data (from market_data)
  SELECT
    time, asset_id, open, high, low, close, volume,
    custom_session_group(time) AS session_name,
    get_trading_date(time) AS trading_date 
  FROM market_data 
)
SELECT
    -- 2. Group by the Trading Day and Asset
    p.trading_date,
    p.asset_id,
    -- ✅ FIX: OPEN Price - Get the 'open' value of the record with the minimum time (first bar).
    (array_agg(p.open ORDER BY p.time ASC))[1] AS open,
    MAX(p.high) AS high,
    MIN(p.low) AS low,
    -- ✅ FIX: CLOSE Price - Get the 'close' value of the record with the maximum time (last bar).
    (array_agg(p.close ORDER BY p.time DESC))[1] AS close,
    SUM(p.volume) AS volume,
    COUNT(*) AS bars,
    MIN(p.time) AS start_ts, 
    MAX(p.time) AS end_ts, 
    -- High/Low Metadata (Your original correct logic for these)
    (array_agg(p.time ORDER BY p.high DESC, p.time ASC))[1] AS high_ts,
    (array_agg(p.time ORDER BY p.low ASC, p.time ASC))[1] AS low_ts,
    (array_agg(p.session_name ORDER BY p.high DESC, p.time ASC))[1] AS high_session,
    (array_agg(p.session_name ORDER BY p.low ASC, p.time ASC))[1] AS low_session
    
FROM base p
GROUP BY p.trading_date, p.asset_id;

REFRESH MATERIALIZED VIEW daily_base;