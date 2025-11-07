-----------------------------
-- daily view table
----------------------------

DROP TABLE IF EXISTS daily_views;
CREATE TABLE daily_views (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    -- Temporal/Categorical D.Mn Inputs
    DOW TEXT NOT NULL,                  -- D.Mn: Day-of-Week filter
    Day_Type TEXT,                      -- D.M1, D.M3: Prior Day Type (PD)
    Day_Close_Type TEXT,                -- D.M4: Follow-Through target classification
   -- NYAM_Bias TEXT,                     -- D.M1: Current Session (CS) target (Bullish/Bearish/Neutral)
    Consolidation_Subtype TEXT,         
    -- Core OHLC Data
    open DOUBLE PRECISION,
    high DOUBLE PRECISION,
    low DOUBLE PRECISION,
    close DOUBLE PRECISION,
    volume BIGINT,
    bars BIGINT,
    -- NEW: Aggregation Range Timestamps for Debugging
    start_ts TIMESTAMP WITH TIME ZONE,  -- Start of the Trading Day aggregation
    end_ts TIMESTAMP WITH TIME ZONE,    -- End of the Trading Day aggregation
    -- High/Low Metadata
    high_ts TIMESTAMP WITH TIME ZONE,
    high_session TEXT,
    low_ts TIMESTAMP WITH TIME ZONE,
    low_session TEXT,
    -- Relational/Lagged Fields (Calculated internally)
    Prior_Day_High DOUBLE PRECISION,
    Prior_Day_Low DOUBLE PRECISION,
    Prior_Week_High DOUBLE PRECISION,  
    Prior_Week_Low DOUBLE PRECISION,   
    PRIMARY KEY(trading_date, asset_id)
);

SELECT create_hypertable('daily_views', 'trading_date', if_not_exists => TRUE);

-- Execute the stored procedure to run the final calculation and INSERT data
CALL refresh_daily_views();

CREATE OR REPLACE PROCEDURE refresh_daily_views()
LANGUAGE sql
AS $$
WITH classified AS (
    -- Get Classification from Layer 1 MV (daily_base_mv)
    SELECT 
        *, 
        get_market_type(open, high, low, close) AS classification
    FROM daily_base
),
enriched AS (
    -- Calculate DOW and Lagged Day Data
    SELECT
        cld.*,
        TO_CHAR(cld.trading_date, 'Dy') AS DOW,
        (cld.classification).session_type AS Day_Type,
        (cld.classification).consolidation_subtype AS Consolidation_Subtype,
        get_close_type(cld.open, cld.high, cld.low, cld.close) AS Day_Close_Type,
        LAG(cld.high, 1, NULL) OVER (PARTITION BY cld.asset_id ORDER BY cld.trading_date) AS Prior_Day_High,
        LAG(cld.low, 1, NULL) OVER (PARTITION BY cld.asset_id ORDER BY cld.trading_date) AS Prior_Day_Low
    FROM classified cld
),
WeeklyAgg AS (
    -- Calculate the Weekly High/Low using window functions on the enriched set
    SELECT
        trading_date,
        asset_id,
        -- Calculate the max/min of the PREVIOUS 5 (trading) days
        MAX(high) OVER (PARTITION BY asset_id ORDER BY trading_date 
                        RANGE BETWEEN INTERVAL '6 days' PRECEDING AND INTERVAL '1 day' PRECEDING) AS Prior_Week_High,
        MIN(low) OVER (PARTITION BY asset_id ORDER BY trading_date 
                        RANGE BETWEEN INTERVAL '6 days' PRECEDING AND INTERVAL '1 day' PRECEDING) AS Prior_Week_Low
    FROM enriched
    -- NOTE: This logic assumes a calendar week calculation based on the trading_date column.
    -- For a precise M-F trading week, the logic is slightly more complex but achievable.
)
-- Insert/Update into the final daily_views Hypertable
INSERT INTO daily_views (
    trading_date, asset_id, DOW, open, high, start_ts, end_ts, high_ts, high_session, 
    low, low_ts, low_session, close, volume, bars, 
    Day_Type, Consolidation_Subtype, Day_Close_Type,
    Prior_Day_High, Prior_Day_Low, Prior_Week_High, Prior_Week_Low
)
SELECT
    e.trading_date, e.asset_id, e.DOW, e.open, e.high, e.start_ts, e.end_ts, e.high_ts, e.high_session, 
    e.low, e.low_ts, e.low_session, e.close, e.volume, e.bars, 
    e.Day_Type, e.Consolidation_Subtype, e.Day_Close_Type,
    e.Prior_Day_High, e.Prior_Day_Low,
    wa.Prior_Week_High,  
    wa.Prior_Week_Low      
FROM enriched e
LEFT JOIN WeeklyAgg wa USING (trading_date, asset_id)
ON CONFLICT (trading_date, asset_id) DO UPDATE SET
    open = EXCLUDED.open, high = EXCLUDED.high, low = EXCLUDED.low, close = EXCLUDED.close,
    volume = EXCLUDED.volume, bars = EXCLUDED.bars, Day_Type = EXCLUDED.Day_Type,
    -- ... update all other non-key fields
    Prior_Day_High = EXCLUDED.Prior_Day_High, Prior_Day_Low = EXCLUDED.Prior_Day_Low,
    Prior_Week_High = EXCLUDED.Prior_Week_High, Prior_Week_Low = EXCLUDED.Prior_Week_Low;
$$;