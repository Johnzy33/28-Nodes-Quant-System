----------
-- Daily Table
--------
DROP TABLE IF EXISTS daily_views;
CREATE TABLE daily_views (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    -- Temporal/Categorical D.Mn Inputs
    DOW TEXT NOT NULL,                  -- D.Mn: Day-of-Week filter
    Day_Type TEXT,                      -- D.M1, D.M3: Prior Day Type (PD)
    Day_Close_Type TEXT,                -- D.M4: Follow-Through target classification
    --NYAM_Bias TEXT,                     -- D.M1: Current Session (CS) target (Bullish/Bearish/Neutral)
    Consolidation_Subtype TEXT,         
    -- Core OHLC Data
    open DOUBLE PRECISION,
    high DOUBLE PRECISION,
    low DOUBLE PRECISION,
    close DOUBLE PRECISION,
    volume BIGINT,
    bars BIGINT,
    -- High/Low Metadata
    high_ts TIMESTAMP WITH TIME ZONE,
    high_session TEXT,
    low_ts TIMESTAMP WITH TIME ZONE,
    low_session TEXT,
    -- Relational/Lagged Fields (Required for D.M2 and D.M5)
    Prior_Day_High DOUBLE PRECISION,
    Prior_Day_Low DOUBLE PRECISION,
    Prior_Week_High DOUBLE PRECISION,  -- Joined from Weekly Track
    Prior_Week_Low DOUBLE PRECISION,   -- Joined from Weekly Track
    PRIMARY KEY(trading_date, asset_id)
);
TRUNCATE daily_views;


WITH base AS (
  -- 1. Original Base: Assign session and trading date to raw data
  SELECT
    time, asset_id, open, high, low, close, volume,
    custom_session_group(time) AS session_name,
    get_trading_date(time) AS trading_date 
  FROM market_data -- Assumes the source data table
),
perday AS (
  -- 2. Original Per Day: Aggregate daily OHLC and find min/max time stamps
  SELECT
    trading_date, asset_id,
    MIN(time) AS min_ts, MAX(time) AS max_ts,
    MAX(high) AS high_price,
    (array_agg(time ORDER BY high DESC, time ASC))[1] AS high_ts,
    MIN(low)  AS low_price,
    (array_agg(time ORDER BY low ASC, time ASC))[1] AS low_ts,
    SUM(volume) AS volume_sum,
    COUNT(*) AS bars
  FROM base
  GROUP BY trading_date, asset_id
),
ohlc_session AS (
  -- 3. Get Open/Close Prices and High/Low Session Names
  SELECT 
    p.trading_date, p.asset_id, p.high_ts, p.low_ts,
    (SELECT b.open FROM base b WHERE b.trading_date = p.trading_date AND b.asset_id = p.asset_id AND b.time = p.min_ts) AS open,
    (SELECT b.close FROM base b WHERE b.trading_date = p.trading_date AND b.asset_id = p.asset_id AND b.time = p.max_ts) AS close,
    (SELECT b.session_name FROM base b WHERE b.time = p.high_ts AND b.trading_date = p.trading_date AND b.asset_id = p.asset_id) AS high_session,
    (SELECT b.session_name FROM base b WHERE b.time = p.low_ts AND b.trading_date = p.trading_date AND b.asset_id = p.asset_id) AS low_session
  FROM perday p
),
classified AS (
    -- 4. Get the daily candle classification (Day_Type)
    SELECT
        p.trading_date, p.asset_id,
        ohlc.open, p.high_price AS high, p.low_price AS low, ohlc.close,
        get_market_type(ohlc.open, p.high_price, p.low_price, ohlc.close) AS classification
    FROM perday p
    JOIN ohlc_session ohlc USING (trading_date, asset_id)
),
-- NEW CTE: Calculate DOW, Lagged data, and Categorical Targets (D.M1, D.M4)
enriched AS (
    SELECT
        cld.trading_date,
        cld.asset_id,
        TO_CHAR(cld.trading_date, 'Dy') AS DOW, -- D.M1-D.M5
        cld.open, cld.high, cld.low, cld.close,
        p.volume_sum AS volume,
        p.bars AS bars,
        -- D.M1 & D.M3 Inputs
        (cld.classification).session_type AS Day_Type,
        (cld.classification).consolidation_subtype AS Consolidation_Subtype,
        -- D.M4 Target
        get_close_type(cld.open, cld.high, cld.low, cld.close) AS Day_Close_Type,
        -- D.M1 Target (The specific session we are forecasting)
        --calculate_nyam_bias(cld.trading_date, cld.asset_id) AS NYAM_Bias,
        -- D.M2 & D.M5 Lagged Fields
        LAG(cld.high, 1, NULL) OVER (PARTITION BY cld.asset_id ORDER BY cld.trading_date) AS Prior_Day_High,
        LAG(cld.low, 1, NULL) OVER (PARTITION BY cld.asset_id ORDER BY cld.trading_date) AS Prior_Day_Low,
        -- High/Low Metadata
        p.high_ts, ohlc.high_session,
        p.low_ts, ohlc.low_session
    FROM classified cld
    JOIN perday p USING (trading_date, asset_id)
    JOIN ohlc_session ohlc USING (trading_date, asset_id)
),
-- ... (up to enriched CTE remains the same)
--- NEW CTE: Calculate the prior week's start date
prior_week_dates AS (
    SELECT
        trading_date,
        asset_id,
        -- Calculate the start of the week for the CURRENT trading_date
        DATE_TRUNC('week', trading_date) AS current_week_start,
        -- Calculate the start of the WEEK BEFORE the CURRENT trading_date
        DATE_TRUNC('week', trading_date) - INTERVAL '7 days' AS prior_week_start
    FROM enriched
)
-- Final Insertion into the new enriched table
INSERT INTO daily_views (
    trading_date, asset_id, DOW, open, high, high_ts, high_session, 
    low, low_ts, low_session, close, volume, bars, 
    Day_Type, Consolidation_Subtype, Day_Close_Type,
    Prior_Day_High, Prior_Day_Low, Prior_Week_High, Prior_Week_Low
)
SELECT
    e.trading_date, e.asset_id, e.DOW, e.open, e.high, e.high_ts, e.high_session, 
    e.low, e.low_ts, e.low_session, e.close, e.volume, e.bars, 
    e.Day_Type, e.Consolidation_Subtype, e.Day_Close_Type,
    e.Prior_Day_High, e.Prior_Day_Low,
    awv.high AS Prior_Week_High,  -- Values from the joined weekly table
    awv.low AS Prior_Week_Low      -- Values from the joined weekly table
FROM enriched e
JOIN prior_week_dates pwd USING (trading_date, asset_id)
LEFT JOIN asset_weekly_views awv
    ON awv.asset_id = e.asset_id
    -- Join to the weekly table's entry for the prior week
    AND awv.week_start = pwd.prior_week_start;


--------------------------------------
--- moveing dependecy from weekly table
--------------------------------------

-- ----------------------------------------------------------------------
-- Daily Views Table Definition
-- Eliminates the dependency on the external 'weekly_views' table for lookback data.
-- ----------------------------------------------------------------------
DROP TABLE IF EXISTS daily_views;
CREATE TABLE daily_views (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    -- Temporal/Categorical D.Mn Inputs
    DOW TEXT NOT NULL,                  -- D.Mn: Day-of-Week filter
    Day_Type TEXT,                      -- D.M1, D.M3: Prior Day Type (PD)
    Day_Close_Type TEXT,                -- D.M4: Follow-Through target classification
    --NYAM_Bias TEXT,                     -- D.M1: Current Session (CS) target (Bullish/Bearish/Neutral)
    Consolidation_Subtype TEXT,         
    -- Core OHLC Data
    open DOUBLE PRECISION,
    high DOUBLE PRECISION,
    low DOUBLE PRECISION,
    close DOUBLE PRECISION,
    volume BIGINT,
    bars BIGINT,
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
TRUNCATE daily_views;


WITH base AS (
  -- 1. Original Base: Assign session and trading date to raw data
  SELECT
    time, asset_id, open, high, low, close, volume,
    custom_session_group(time) AS session_name,
    get_trading_date(time) AS trading_date 
  FROM market_data -- Assumes the source data table
),
perday AS (
  -- 2. Original Per Day: Aggregate daily OHLC and find min/max time stamps
  SELECT
    trading_date, asset_id,
    MIN(time) AS min_ts, MAX(time) AS max_ts,
    MAX(high) AS high_price,
    (array_agg(time ORDER BY high DESC, time ASC))[1] AS high_ts,
    MIN(low)  AS low_price,
    (array_agg(time ORDER BY low ASC, time ASC))[1] AS low_ts,
    SUM(volume) AS volume_sum,
    COUNT(*) AS bars
  FROM base
  GROUP BY trading_date, asset_id
),
ohlc_session AS (
  -- 3. Get Open/Close Prices and High/Low Session Names
  SELECT 
    p.trading_date, p.asset_id, p.high_ts, p.low_ts,
    (SELECT b.open FROM base b WHERE b.trading_date = p.trading_date AND b.asset_id = p.asset_id AND b.time = p.min_ts) AS open,
    (SELECT b.close FROM base b WHERE b.trading_date = p.trading_date AND b.asset_id = p.asset_id AND b.time = p.max_ts) AS close,
    (SELECT b.session_name FROM base b WHERE b.time = p.high_ts AND b.trading_date = p.trading_date AND b.asset_id = p.asset_id) AS high_session,
    (SELECT b.session_name FROM base b WHERE b.time = p.low_ts AND b.trading_date = p.trading_date AND b.asset_id = p.asset_id) AS low_session
  FROM perday p
),
classified AS (
    -- 4. Get the daily candle classification (Day_Type)
    SELECT
        p.trading_date, p.asset_id,
        ohlc.open, p.high_price AS high, p.low_price AS low, ohlc.close,
        get_market_type(ohlc.open, p.high_price, p.low_price, ohlc.close) AS classification
    FROM perday p
    JOIN ohlc_session ohlc USING (trading_date, asset_id)
),
enriched AS (
    -- 5. Calculate DOW, Lagged day data, and Categorical Targets
    SELECT
        cld.trading_date,
        cld.asset_id,
        TO_CHAR(cld.trading_date, 'Dy') AS DOW,
        cld.open, cld.high, cld.low, cld.close,
        p.volume_sum AS volume,
        p.bars AS bars,
        (cld.classification).session_type AS Day_Type,
        (cld.classification).consolidation_subtype AS Consolidation_Subtype,
        get_close_type(cld.open, cld.high, cld.low, cld.close) AS Day_Close_Type,
        calculate_nyam_bias(cld.trading_date, cld.asset_id) AS NYAM_Bias,
        -- Prior Day Lagged Fields (Correctly calculated using LAG)
        LAG(cld.high, 1, NULL) OVER (PARTITION BY cld.asset_id ORDER BY cld.trading_date) AS Prior_Day_High,
        LAG(cld.low, 1, NULL) OVER (PARTITION BY cld.asset_id ORDER BY cld.trading_date) AS Prior_Day_Low,
        -- High/Low Metadata
        p.high_ts, ohlc.high_session,
        p.low_ts, ohlc.low_session
    FROM classified cld
    JOIN perday p USING (trading_date, asset_id)
    JOIN ohlc_session ohlc USING (trading_date, asset_id)
),
WeeklyAgg AS (
    -- 6. FIX: Calculate the weekly High/Low for EVERY week directly from daily data
    SELECT
        -- Get the start of the week for grouping
        DATE_TRUNC('week', trading_date) AS week_start,
        asset_id,
        MAX(high) AS weekly_high,
        MIN(low) AS weekly_low
    FROM enriched -- Use the daily enriched data
    GROUP BY 1, 2
),
prior_week_dates AS (
    -- 7. Calculate the start date of the week *before* the current trading day
    SELECT
        trading_date,
        asset_id,
        -- Calculate the start of the WEEK BEFORE the CURRENT trading_date
        DATE_TRUNC('week', trading_date) - INTERVAL '7 days' AS prior_week_start
    FROM enriched
)
-- Final Insertion into the daily_views table
INSERT INTO daily_views (
    trading_date, asset_id, DOW, open, high, high_ts, high_session, 
    low, low_ts, low_session, close, volume, bars, 
    Day_Type, Consolidation_Subtype, Day_Close_Type, NYAM_Bias,
    Prior_Day_High, Prior_Day_Low, Prior_Week_High, Prior_Week_Low
)
SELECT
    e.trading_date, e.asset_id, e.DOW, e.open, e.high, e.high_ts, e.high_session, 
    e.low, e.low_ts, e.low_session, e.close, e.volume, e.bars, 
    e.Day_Type, e.Consolidation_Subtype, e.Day_Close_Type, e.NYAM_Bias,
    e.Prior_Day_High, e.Prior_Day_Low,
    waw.weekly_high AS Prior_Week_High,  -- Joins to the High of the week calculated in WeeklyAgg
    waw.weekly_low AS Prior_Week_Low      -- Joins to the Low of the week calculated in WeeklyAgg
FROM enriched e
JOIN prior_week_dates pwd USING (trading_date, asset_id)
LEFT JOIN WeeklyAgg waw  -- Join to our internal weekly aggregation CTE
    ON waw.asset_id = e.asset_id
    -- The join condition is the key: Match the CURRENT row's PRIOR week start date
    AND waw.week_start = pwd.prior_week_start;
