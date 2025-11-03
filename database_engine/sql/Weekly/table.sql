-------------
-- Table: Weekly
-------------
DROP TABLE IF EXISTS asset_weekly_views CASCADE;

CREATE TABLE asset_weekly_views (
    week_start DATE NOT NULL,
    asset_id TEXT NOT NULL,
    -- Temporal/Categorical W.Mn Inputs
    Month_of_Year INTEGER NOT NULL,
    Weekly_Type TEXT,
    Consolidation_Subtype TEXT,
    -- Core OHLC Data
    open DOUBLE PRECISION,
    high DOUBLE PRECISION,
    low DOUBLE PRECISION,
    close DOUBLE PRECISION,
    volume BIGINT,
    bars BIGINT,
    -- W.M5 Prerequisites (Time/Range to break PRIOR week structure)
    Time_To_PWH_Break INTEGER,
    Time_To_PWL_Break INTEGER,
    Range_Used_At_PWH_Break DOUBLE PRECISION,
    Range_Used_At_PWL_Break DOUBLE PRECISION,
    -- W.M6 & W.M5 Prerequisites (Adding Weekly ATR for normalization)
    Weekly_Range DOUBLE PRECISION,
    Weekly_Pullback_Ratio DOUBLE PRECISION,
    Weekly_ATR DOUBLE PRECISION, -- NEW: Added for W.M5 normalization
    -- High/Low Metadata
    high_trading_date DATE,
    high_ts TIMESTAMP WITH TIME ZONE,
    high_session TEXT,
    low_trading_date DATE,
    low_ts TIMESTAMP WITH TIME ZONE,
    low_session TEXT,
    
    PRIMARY KEY(week_start, asset_id)
);


TRUNCATE TABLE asset_weekly_views

INSERT INTO asset_weekly_views (
    week_start, asset_id, Month_of_Year, open, high, low, close, volume, bars, 
    Weekly_Type, Consolidation_Subtype, Weekly_Range, Weekly_Pullback_Ratio, Weekly_ATR,
    Time_To_PWH_Break, Time_To_PWL_Break, Range_Used_At_PWH_Break, Range_Used_At_PWL_Break,
    high_trading_date, high_ts, high_session, low_trading_date, low_ts, low_session
)
WITH weekly_aggregate AS (
    -- 1. Aggregate Daily Views data and calculate Weekly ATR/Range/OHLC
    SELECT
        DATE_TRUNC('week', d.trading_date) AS week_start,
        d.asset_id,
        (ARRAY_AGG(d.open ORDER BY d.trading_date ASC))[1] AS o_price,
        MAX(d.high) AS h_price,
        MIN(d.low) AS l_price,
        (ARRAY_AGG(d.close ORDER BY d.trading_date DESC))[1] AS c_price,
        SUM(d.volume) AS volume_sum,
        SUM(d.bars) AS bars_sum,
        MAX(d.high) - MIN(d.low) AS Current_Weekly_Range,
        AVG(a.daily_atr) AS Weekly_ATR 
    FROM asset_daily_views d
    JOIN asset_daily_atr_14d a ON a.asset_id = d.asset_id AND a.trading_date = d.trading_date
    GROUP BY 1, 2
),
prior_week_data AS (
    -- 2. Pre-calculate Prior Week data
    SELECT wa.week_start, wa.asset_id,
        LAG(wa.h_price, 1) OVER (PARTITION BY wa.asset_id ORDER BY wa.week_start) AS Prior_Week_High,
        LAG(wa.l_price, 1) OVER (PARTITION BY wa.asset_id ORDER BY wa.week_start) AS Prior_Week_Low,
        LAG(wa.Weekly_ATR, 1) OVER (PARTITION BY wa.asset_id ORDER BY wa.week_start) AS Prior_Week_ATR
    FROM weekly_aggregate wa
),
dow_to_index AS (
    -- 3. Helper CTE to map the text DOW to a numeric index
    SELECT trading_date, asset_id, "open", high, low,
        CASE d.dow WHEN 'Mon' THEN 1 WHEN 'Tue' THEN 2 WHEN 'Wed' THEN 3 WHEN 'Thu' THEN 4 WHEN 'Fri' THEN 5 ELSE 99 END AS dow_index 
    FROM asset_daily_views d
),
high_low_metadata AS (
    -- 4. Find the exact high/low timestamps and sessions
    SELECT
        w.week_start, w.asset_id,
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
break_metrics AS (
    -- 5. W.M5 Prerequisites: Calculate Time to Break
    SELECT
        pwd.week_start, pwd.asset_id,
        MIN(CASE WHEN d.high > pwd.Prior_Week_High THEN d.dow_index ELSE NULL END) AS Time_To_PWH_Break,
        MIN(CASE WHEN d.low < pwd.Prior_Week_Low THEN d.dow_index ELSE NULL END) AS Time_To_PWL_Break,
        (ARRAY_AGG(d.open ORDER BY d.dow_index ASC))[1] AS Weekly_Open 
    FROM prior_week_data pwd
    JOIN dow_to_index d ON d.asset_id = pwd.asset_id AND DATE_TRUNC('week', d.trading_date) = pwd.week_start
    WHERE pwd.Prior_Week_High IS NOT NULL
    GROUP BY 1, 2
),
range_metrics AS (
    -- 6. W.M6 & W.M5 Final Range/Ratio Calculation
    SELECT
        b.week_start, b.asset_id, wa.Current_Weekly_Range, wa.Weekly_ATR, b.Time_To_PWH_Break, b.Time_To_PWL_Break, pwd.Prior_Week_ATR,
        CASE WHEN wa.c_price >= wa.o_price THEN (wa.h_price - wa.c_price) / NULLIF(wa.Current_Weekly_Range, 0)
            ELSE (wa.c_price - wa.l_price) / NULLIF(wa.Current_Weekly_Range, 0) END AS Weekly_Pullback_Ratio,
        CASE WHEN b.Time_To_PWH_Break IS NOT NULL THEN (wa.h_price - b.Weekly_Open) / NULLIF(pwd.Prior_Week_ATR, 0)
            WHEN b.Time_To_PWL_Break IS NOT NULL THEN (b.Weekly_Open - wa.l_price) / NULLIF(pwd.Prior_Week_ATR, 0) ELSE NULL END AS Range_Used
    FROM break_metrics b
    JOIN weekly_aggregate wa USING (week_start, asset_id)
    JOIN prior_week_data pwd USING (week_start, asset_id)
),
classification_result AS (
    -- 7. Isolate the Classification result and the OHLC data
    SELECT
        wa.week_start, wa.asset_id, wa.volume_sum, wa.bars_sum, wa.Current_Weekly_Range, wa.Weekly_ATR,
        -- Force casting the OHLC data before the function call
        get_market_type(wa.o_price::DOUBLE PRECISION, wa.h_price::DOUBLE PRECISION, wa.l_price::DOUBLE PRECISION, wa.c_price::DOUBLE PRECISION) AS classification,
        -- RETAIN THE RAW OHLC VALUES (Now isolated from the classification call)
        wa.o_price, wa.h_price, wa.l_price, wa.c_price
    FROM weekly_aggregate wa
)
-- 8. FINAL SELECT: Safely assign all types and names
SELECT
    c.week_start, c.asset_id,
    DATE_PART('month', c.week_start)::INTEGER AS Month_of_Year,
    (c.classification).session_type AS Weekly_Type, 
    (c.classification).consolidation_subtype AS Consolidation_Subtype,
    -- FINAL TYPE CONVERSION: Cast the prefixed columns to the final target column names
    c.o_price::DOUBLE PRECISION AS open, 
    c.h_price::DOUBLE PRECISION AS high, 
    c.l_price::DOUBLE PRECISION AS low, 
    c.c_price::DOUBLE PRECISION AS close, 
    c.volume_sum, 
    c.bars_sum,
    r.Time_To_PWH_Break, r.Time_To_PWL_Break,
    CASE WHEN r.Time_To_PWH_Break IS NOT NULL THEN r.Range_Used ELSE NULL END AS Range_Used_At_PWH_Break,
    CASE WHEN r.Time_To_PWL_Break IS NOT NULL THEN r.Range_Used ELSE NULL END AS Range_Used_At_PWL_Break,
    r.Current_Weekly_Range AS Weekly_Range, 
    r.Weekly_Pullback_Ratio, 
    r.Weekly_ATR,
    m.high_trading_date, m.high_ts, m.high_session, m.low_trading_date, m.low_ts, m.low_session
FROM classification_result c
JOIN high_low_metadata m USING (week_start, asset_id)
JOIN range_metrics r USING (week_start, asset_id)
ON CONFLICT (week_start, asset_id) DO UPDATE SET
    Month_of_Year = EXCLUDED.Month_of_Year, 
    -- EXCLUDED references the final SELECT aliases
    open = EXCLUDED.open, high = EXCLUDED.high, low = EXCLUDED.low,
    close = EXCLUDED.close, volume = EXCLUDED.volume, bars = EXCLUDED.bars, 
    Weekly_Type = EXCLUDED.Weekly_Type,
    Consolidation_Subtype = EXCLUDED.Consolidation_Subtype, 
    Time_To_PWH_Break = EXCLUDED.Time_To_PWH_Break,
    Time_To_PWL_Break = EXCLUDED.Time_To_PWL_Break, 
    Range_Used_At_PWH_Break = EXCLUDED.Range_Used_At_PWH_Break,
    Range_Used_At_PWL_Break = EXCLUDED.Range_Used_At_PWL_Break, 
    Weekly_Range = EXCLUDED.Weekly_Range,
    Weekly_Pullback_Ratio = EXCLUDED.Weekly_Pullback_Ratio, 
    Weekly_ATR = EXCLUDED.Weekly_ATR;