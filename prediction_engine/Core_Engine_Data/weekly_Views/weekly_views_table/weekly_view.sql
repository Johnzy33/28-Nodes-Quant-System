-------------------------------------
---- Weekly Main View
------------------------------------

-- 1. Ensure the correct table is dropped
DROP TABLE IF EXISTS weekly_views CASCADE;

-- 2. Create the target table (as a Hypertable for future partitioning)
CREATE TABLE weekly_views (
    week_start DATE NOT NULL,
    asset_id TEXT NOT NULL,
    Month_of_Year INTEGER NOT NULL,
    Weekly_Type TEXT,
    Consolidation_Subtype TEXT,
    open DOUBLE PRECISION,
    high DOUBLE PRECISION,
    low DOUBLE PRECISION,
    close DOUBLE PRECISION,
    volume BIGINT,
    bars BIGINT,
    high_trading_date DATE,
    high_ts TIMESTAMP WITH TIME ZONE,
    high_session TEXT,
    low_trading_date DATE,
    low_ts TIMESTAMP WITH TIME ZONE,
    low_session TEXT,
    PRIMARY KEY(week_start, asset_id)
);

-- 3. Initialize as a TimescaleDB Hypertable
SELECT create_hypertable('weekly_views', 'week_start', if_not_exists => TRUE);
CALL refresh_weekly_views();

CREATE OR REPLACE PROCEDURE refresh_weekly_views()
LANGUAGE sql
AS $$
INSERT INTO weekly_views (
    week_start, asset_id, Month_of_Year, open, high, low, close, volume, bars, 
    Weekly_Type, Consolidation_Subtype,
    high_trading_date, high_ts, high_session, low_trading_date, low_ts, low_session
)
WITH weekly_aggregate AS (
    -- 1. Aggregate Daily Views data into weekly buckets (with mandatory casts)
    SELECT
        DATE_TRUNC('week', trading_date) AS week_start,
        asset_id,
        (ARRAY_AGG(open ORDER BY trading_date ASC))[1]::DOUBLE PRECISION AS open_price,
        MAX(high) AS high_price,
        MIN(low) AS low_price,
        (ARRAY_AGG(close ORDER BY trading_date DESC))[1]::DOUBLE PRECISION AS close_price,
        SUM(volume) AS volume_sum,
        SUM(bars) AS bars_sum
    FROM daily_views
    GROUP BY 1, 2
),
high_low_metadata AS (
    -- 2. Find the exact high/low timestamps and sessions
    SELECT
        w.week_start, w.asset_id,
        (ARRAY_AGG(d.trading_date ORDER BY d.high DESC))[1] AS high_trading_date,
        (ARRAY_AGG(d.high_ts ORDER BY d.high DESC))[1] AS high_ts,
        (ARRAY_AGG(d.high_session ORDER BY d.high DESC))[1] AS high_session,
        (ARRAY_AGG(d.trading_date ORDER BY d.low ASC))[1] AS low_trading_date,
        (ARRAY_AGG(d.low_ts ORDER BY d.low ASC))[1] AS low_ts,
        (ARRAY_AGG(d.low_session ORDER BY d.low ASC))[1] AS low_session
    FROM weekly_aggregate w
    JOIN daily_views d ON d.asset_id = w.asset_id AND DATE_TRUNC('week', d.trading_date) = w.week_start
    GROUP BY 1, 2
),
classified AS (
    -- 3. Classify the weekly candle type
    SELECT
        w.*,
        get_market_type(w.open_price, w.high_price, w.low_price, w.close_price) AS classification
    FROM weekly_aggregate w
)
-- Final Insertion into the new weekly table
SELECT
    c.week_start,
    c.asset_id,
    DATE_PART('month', c.week_start)::INTEGER AS Month_of_Year,
    c.open_price AS open,  -- Alias for insertion
    c.high_price AS high,  -- Alias for insertion
    c.low_price AS low,    -- Alias for insertion
    c.close_price AS close, -- Alias for insertion
    c.volume_sum AS volume, -- Alias for insertion
    c.bars_sum AS bars,     -- Alias for insertion
    (c.classification).session_type AS Weekly_Type,
    (c.classification).consolidation_subtype AS Consolidation_Subtype,
    m.high_trading_date, m.high_ts, m.high_session,
    m.low_trading_date, m.low_ts, m.low_session
FROM classified c
JOIN high_low_metadata m USING (week_start, asset_id)
ON CONFLICT (week_start, asset_id) DO UPDATE SET
    Month_of_Year = EXCLUDED.Month_of_Year,
    -- **FIX HERE: Using Target Column Names**
    open = EXCLUDED.open,
    high = EXCLUDED.high,
    low = EXCLUDED.low,
    close = EXCLUDED.close,
    volume = EXCLUDED.volume,
    bars = EXCLUDED.bars,
    Weekly_Type = EXCLUDED.Weekly_Type,
    Consolidation_Subtype = EXCLUDED.Consolidation_Subtype,
    high_trading_date = EXCLUDED.high_trading_date,
    high_ts = EXCLUDED.high_ts,
    high_session = EXCLUDED.high_session,
    low_trading_date = EXCLUDED.low_trading_date,
    low_ts = EXCLUDED.low_ts,
    low_session = EXCLUDED.low_session;
$$;
