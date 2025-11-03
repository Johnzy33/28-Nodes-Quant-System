
DROP TABLE IF EXISTS asset_weekly_views;

CREATE TABLE asset_weekly_views (
    week_start DATE NOT NULL,
    asset_id TEXT NOT NULL,
    -- Temporal/Categorical W.Mn Inputs
    Month_of_Year INTEGER NOT NULL,     -- W.Mn: Primary seasonal filter
    Weekly_Type TEXT,                   -- W.M1, W.M3: Prior Week Type (PW)
    Consolidation_Subtype TEXT,
    -- Core OHLC Data
    open DOUBLE PRECISION,
    high DOUBLE PRECISION,
    low DOUBLE PRECISION,
    close DOUBLE PRECISION,
    volume BIGINT,
    bars BIGINT,
    -- High/Low Metadata
    high_trading_date DATE,
    high_ts TIMESTAMP WITH TIME ZONE,
    high_session TEXT,
    low_trading_date DATE,
    low_ts TIMESTAMP WITH TIME ZONE,
    low_session TEXT,
    PRIMARY KEY(week_start, asset_id)
);
-- NOTE: This CTE assumes the existence of the following custom functions:
-- get_weekly_type(open, high, low, close) -> returns (weekly_type, consolidation_subtype)

-- NOTE: This CTE assumes the existence of the following custom functions:
-- get_market_type(open, high, low, close) -> returns (session_type, consolidation_subtype)
TRUNCATE TABLE asset_weekly_views;
INSERT INTO asset_weekly_views (
    week_start, asset_id, Month_of_Year, open, high, low, close, volume, bars, 
    Weekly_Type, Consolidation_Subtype,
    high_trading_date, high_ts, high_session, low_trading_date, low_ts, low_session
)
WITH weekly_aggregate AS (
    -- 1. Aggregate Daily Views data into weekly buckets
    SELECT
        DATE_TRUNC('week', trading_date) AS week_start,
        asset_id,
        MIN(trading_date) AS start_trading_date,
        MAX(trading_date) AS end_trading_date,
        (ARRAY_AGG(open ORDER BY trading_date ASC))[1] AS open_price, -- Open is the first day's open
        MAX(high) AS high_price,
        MIN(low) AS low_price,
        (ARRAY_AGG(close ORDER BY trading_date DESC))[1] AS close_price, -- Close is the last day's close
        SUM(volume) AS volume_sum,
        SUM(bars) AS bars_sum
    FROM asset_daily_views -- Renamed table
    GROUP BY 1, 2
),
high_low_metadata AS (
    -- 2. Find the exact high/low timestamps and sessions from the days they occurred
    SELECT
        w.week_start,
        w.asset_id,
        (ARRAY_AGG(d.trading_date ORDER BY d.high DESC))[1] AS high_trading_date,
        (ARRAY_AGG(d.high_ts ORDER BY d.high DESC))[1] AS high_ts,
        (ARRAY_AGG(d.high_session ORDER BY d.high DESC))[1] AS high_session,
        (ARRAY_AGG(d.trading_date ORDER BY d.low ASC))[1] AS low_trading_date,
        (ARRAY_AGG(d.low_ts ORDER BY d.low ASC))[1] AS low_ts,
        (ARRAY_AGG(d.low_session ORDER BY d.low ASC))[1] AS low_session
    FROM weekly_aggregate w
    JOIN asset_daily_views d ON d.asset_id = w.asset_id AND DATE_TRUNC('week', d.trading_date) = w.week_start
    GROUP BY 1, 2
),
classified AS (
    -- 3. Classify the weekly candle type using the consistent get_market_type function
    SELECT
        w.*,
        get_market_type(w.open_price, w.high_price, w.low_price, w.close_price) AS classification
    FROM weekly_aggregate w
)
-- Final Insertion into the new weekly table
SELECT
    c.week_start,
    c.asset_id,
    DATE_PART('month', c.week_start)::INTEGER AS Month_of_Year, -- W.Mn Conditional Factor
    c.open_price,
    c.high_price,
    c.low_price,
    c.close_price,
    c.volume_sum,
    c.bars_sum,
    (c.classification).session_type AS Weekly_Type,          -- Use session_type for Weekly_Type
    (c.classification).consolidation_subtype AS Consolidation_Subtype,
    m.high_trading_date, m.high_ts, m.high_session,
    m.low_trading_date, m.low_ts, m.low_session
FROM classified c
JOIN high_low_metadata m USING (week_start, asset_id)
ON CONFLICT (week_start, asset_id) DO UPDATE SET
    Month_of_Year = EXCLUDED.Month_of_Year,
    high = EXCLUDED.high,
    -- ... (Include other columns to update if this is a recurring job)
    Weekly_Type = EXCLUDED.Weekly_Type;
