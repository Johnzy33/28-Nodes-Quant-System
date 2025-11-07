------------------------------------
-- Session base
-----------------------------------
DROP MATERIALIZED VIEW session_base;

-- Create the Session Base Materialized View
CREATE MATERIALIZED VIEW session_base AS
WITH base AS (
  SELECT
    time, asset_id, open, high, low, close, volume,
    custom_session_group(time) AS session_name,
    get_trading_date(time) AS trading_date
  FROM market_data
)
SELECT
    p.trading_date,
    p.asset_id,
    p.session_name,
    MIN(p.time) AS start_ts,
    MAX(p.time) AS end_ts,
    -- ✅ FIX: OPEN Price - Get the 'open' value of the record with the minimum time (first bar)
    (array_agg(p.open ORDER BY p.time ASC))[1] AS open,
    MAX(p.high) AS high,
    -- High/Low Timestamps
    (array_agg(p.time ORDER BY p.high DESC, p.time ASC))[1] AS high_ts,
    MIN(p.low) AS low,
    
    (array_agg(p.time ORDER BY p.low ASC, p.time ASC))[1] AS low_ts,
    -- ✅ FIX: CLOSE Price - Get the 'close' value of the record with the maximum time (last bar)
    (array_agg(p.close ORDER BY p.time DESC))[1] AS close,
    SUM(p.volume) AS volume,
    COUNT(*) AS bars
FROM base p
GROUP BY p.trading_date, p.asset_id, p.session_name;

-- After running this, always run:
REFRESH MATERIALIZED VIEW session_base;