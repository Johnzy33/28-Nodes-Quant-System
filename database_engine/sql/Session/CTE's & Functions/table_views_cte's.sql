----------------------------------
-- Common Table Expressions (CTEs) & Functions for Table Views
----------------------------------  

-- Note: The following CTEs and functions are building blocks used in various table view calculations. They are not standalone queries but are intended to be integrated into larger SQL statements for specific analyses.  
-------------------------------
-- CTE: Session Context
-------------------------------
-- This CTE builds the session context for assets on a given trading date.
TRUNCATE asset_session_context; -- Use generalized table name
WITH sequenced_sessions AS (
    -- 1. Combine session details with a sequence number for each asset
    SELECT
        trading_date,
        asset_id,
        session_name,
        start_ts,
        session_type AS bias_3_state,
        CASE 
            WHEN session_type IN ('Bullish', 'Bearish') THEN session_type
            WHEN session_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other' 
        END AS bias_7_state,
        -- Generate a unique sequence number for sessions within each asset
        ROW_NUMBER() OVER (PARTITION BY asset_id ORDER BY start_ts) AS session_num
    FROM asset_session_views
),
context_pairs AS (
    -- 2. Use LAG window function to find PS1 and PS2 efficiently
    SELECT
        -- CS (Current Session) Details
        cs.trading_date,
        cs.asset_id,
        cs.session_name AS cs_name,
        cs.bias_3_state AS cs_bias_3_state,
        cs.bias_7_state AS cs_bias_7_state,
        -- PS1 (Preceding Session 1) Details
        LAG(cs.session_name, 1) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps1_name,
        LAG(cs.bias_3_state, 1) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps1_bias_3_state,
        LAG(cs.bias_7_state, 1) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps1_bias_7_state,
        -- PS2 (Preceding Session 2) Details
        LAG(cs.session_name, 2) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps2_name,
        LAG(cs.bias_3_state, 2) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps2_bias_3_state,
        LAG(cs.bias_7_state, 2) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps2_bias_7_state
        
    FROM sequenced_sessions cs
),
context_keys AS (
    -- 3. Calculate Unique Foreign Keys for all combinations
    SELECT
        *,
        -- PK for the PS1 -> CS pair (Used by PCS, CVI)
        DENSE_RANK() OVER (
            ORDER BY 
                asset_id, ps1_name, cs_name, 
                ps1_bias_3_state, cs_bias_3_state
        ) + nextval('cs_ps1_fk_seq') AS calculated_cs_ps1_fk,
        -- FK for the PS2 -> PS1 pair (Used by TCS)
        CASE WHEN ps2_name IS NOT NULL THEN 
             DENSE_RANK() OVER (
                ORDER BY 
                    asset_id, ps2_name, ps1_name, 
                    ps2_bias_7_state, ps1_bias_7_state
            ) + nextval('ps2_ps1_fk_seq')
        END AS calculated_ps2_ps1_fk,
        -- FK for the PS2 -> PS1 -> CS triple (Used by 2nd Order Metrics)
        CASE WHEN ps2_name IS NOT NULL THEN
            DENSE_RANK() OVER (
                ORDER BY 
                    asset_id, ps2_name, ps1_name, cs_name, 
                    ps2_bias_7_state, ps1_bias_7_state, cs_bias_7_state
            ) + nextval('cs_ps2_fk_seq')
        END AS calculated_cs_ps2_fk
        
    FROM context_pairs
    -- Only include rows where ps1_name is not null
    WHERE ps1_name IS NOT NULL
)
-- 4. Insert into the final context table
INSERT INTO asset_session_context (
    cs_ps1_fk, trading_date, asset_id, cs_name, cs_bias_3_state, 
    cs_bias_7_state, ps1_name, ps1_bias_3_state, ps1_bias_7_state, 
    ps2_name, ps2_bias_3_state, ps2_bias_7_state, ps2_ps1_fk, cs_ps2_fk
)
SELECT
    calculated_cs_ps1_fk, trading_date, asset_id, cs_name, cs_bias_3_state, 
    cs_bias_7_state, ps1_name, ps1_bias_3_state, ps1_bias_7_state, 
    ps2_name, ps2_bias_3_state, ps2_bias_7_state, calculated_ps2_ps1_fk, calculated_cs_ps2_fk
FROM context_keys;



----------------------------------
-- CTE: Session Views
----------------------------------
-- This CTE extracts session-level data for assets on a given trading date.

TRUNCATE asset_session_views; -- Use generalized table name

WITH base AS (
  SELECT
    time,
    asset_id,
    open, high,  low,  close, volume,
    custom_session_group(time) AS session_name,
    get_trading_date(time) AS trading_date
  FROM asset_market_data -- <<< GENERALIZED TABLE NAME
),
per_session AS (
  SELECT
    trading_date,
    asset_id,
    session_name,
    MIN(time) AS start_ts,
    MAX(time) AS end_ts,
    MAX(high) AS high_price,
    (array_agg(time ORDER BY high DESC, time ASC))[1] AS high_ts,
    MIN(low)  AS low_price,
    (array_agg(time ORDER BY low ASC, time ASC))[1] AS low_ts,
    SUM(volume) AS volume_sum,
    COUNT(*) AS bars
  FROM base
  GROUP BY trading_date, asset_id, session_name -- Already includes asset_id
),
-- ... (opens, closes, classified CTEs remain the same) ...
opens AS (
  SELECT b.trading_date, b.asset_id, b.session_name, b.open AS open_price
  FROM base b
  JOIN per_session p USING (trading_date, asset_id, session_name)
  WHERE p.start_ts = b.time
),
closes AS (
  SELECT b.trading_date, b.asset_id, b.session_name, b.close AS close_price
  FROM base b
  JOIN per_session p USING (trading_date, asset_id, session_name)
  WHERE p.end_ts = b.time
),
classified AS (
    SELECT
        p.trading_date,
        p.asset_id,
        p.session_name,
        get_market_type(o.open_price, p.high_price, p.low_price, c.close_price) AS classification
    FROM per_session p
    LEFT JOIN opens o USING (trading_date, asset_id, session_name)
    LEFT JOIN closes c USING (trading_date, asset_id, session_name)
)
INSERT INTO asset_session_views ( -- <<< GENERALIZED TABLE NAME
    trading_date, asset_id, session_name, start_ts, end_ts, 
    open, high, high_ts, low, low_ts, close, volume, bars, 
    session_type, consolidation_subtype
)
SELECT
  p.trading_date,
  p.asset_id,
  p.session_name,
  p.start_ts,
  p.end_ts,
  o.open_price,
  p.high_price,
  p.high_ts,
  p.low_price,
  p.low_ts,
  c.close_price,
  p.volume_sum,
  p.bars,
  (cld.classification).session_type,
  (cld.classification).consolidation_subtype
FROM per_session p
LEFT JOIN opens o USING (trading_date, asset_id, session_name)
LEFT JOIN closes c USING (trading_date, asset_id, session_name)
LEFT JOIN classified cld USING (trading_date, asset_id, session_name);

----------------------------------
-- CTE: Daily Views
----------------------------------
-- This CTE aggregates session-level data to daily-level for assets.    

TRUNCATE asset_daily_views; -- Use generalized table name

WITH base AS (
  SELECT
    time, asset_id, open, high, low, close, volume,
    custom_session_group(time) AS session_name,
    get_trading_date(time) AS trading_date
  FROM asset_market_data -- <<< GENERALIZED TABLE NAME
),
perday AS (
  -- ... (perday CTE logic remains the same, groups by trading_date, asset_id) ...
  SELECT
    trading_date,
    asset_id,
    MIN(time) AS min_ts,
    MAX(time) AS max_ts,
    MAX(high) AS high_price,
    (array_agg(time ORDER BY high DESC, time ASC))[1] AS high_ts,
    MIN(low)  AS low_price,
    (array_agg(time ORDER BY low ASC, time ASC))[1] AS low_ts,
    SUM(volume) AS volume_sum,
    COUNT(*) AS bars
  FROM base
  GROUP BY trading_date, asset_id
),
-- ... (opens, closes, high_row, low_row, classified CTEs remain the same) ...
opens AS (
  SELECT b.trading_date, b.asset_id, b.open AS open_price
  FROM base b
  JOIN perday p ON p.trading_date = b.trading_date AND p.asset_id = b.asset_id AND p.min_ts = b.time
),
closes AS (
  SELECT b.trading_date, b.asset_id, b.close AS close_price
  FROM base b
  JOIN perday p ON p.trading_date = b.trading_date AND p.asset_id = b.asset_id AND p.max_ts = b.time
),
high_row AS (
  SELECT b.trading_date, b.asset_id, b.time AS high_ts, b.session_name AS high_session
  FROM base b
  JOIN perday p ON p.trading_date = b.trading_date AND p.asset_id = b.asset_id AND p.high_ts = b.time
),
low_row AS (
  SELECT b.trading_date, b.asset_id, b.time AS low_ts, b.session_name AS low_session
  FROM base b
  JOIN perday p ON p.trading_date = b.trading_date AND p.asset_id = b.asset_id AND p.low_ts = b.time
),
classified AS (
    SELECT
        p.trading_date,
        p.asset_id,
        get_market_type(o.open_price, p.high_price, p.low_price, c.close_price) AS classification
    FROM perday p
    LEFT JOIN opens o USING (trading_date, asset_id)
    LEFT JOIN closes c USING (trading_date, asset_id)
)
INSERT INTO asset_daily_views ( -- <<< GENERALIZED TABLE NAME
    trading_date, asset_id, open, high, high_ts, high_session, 
    low, low_ts, low_session, close, volume, bars, 
    day_type, consolidation_subtype
)
SELECT
  p.trading_date,
  p.asset_id,
  o.open_price,
  p.high_price,
  hr.high_ts,
  hr.high_session,
  p.low_price,
  lr.low_ts,
  lr.low_session,
  c.close_price,
  p.volume_sum,
  p.bars,
  (cld.classification).session_type,
  (cld.classification).consolidation_subtype
FROM perday p
LEFT JOIN opens o USING (trading_date, asset_id)
LEFT JOIN closes c USING (trading_date, asset_id)
LEFT JOIN high_row hr USING (trading_date, asset_id)
LEFT JOIN low_row lr USING (trading_date, asset_id)
LEFT JOIN classified cld USING (trading_date, asset_id);

----------------------------------
-- CTE: Weekly Views
----------------------------------
-- This CTE aggregates daily-level data to weekly-level for assets.

TRUNCATE asset_weekly_views; -- Use generalized table name

WITH base AS (
  SELECT trading_date, asset_id, open, high, low, close, volume, bars
  FROM asset_daily_views -- <<< GENERALIZED TABLE NAME
),
perweek AS (
  SELECT
    date_trunc('week', trading_date::timestamp)::date AS week_start,
    asset_id,
    MIN(trading_date) AS start_trading_date,
    MAX(trading_date) AS end_trading_date,
    MAX(high) AS high_price,
    MIN(low) AS low_price,
    SUM(volume) AS volume_sum,
    SUM(bars) AS bars,
    (array_agg(trading_date ORDER BY high DESC, trading_date ASC))[1] AS high_trading_date,
    (array_agg(trading_date ORDER BY low  ASC, trading_date ASC))[1] AS low_trading_date
  FROM asset_daily_views -- <<< GENERALIZED TABLE NAME
  GROUP BY 1, 2
),
classified AS (
    SELECT
        p.week_start,
        p.asset_id,
        s_open.open AS open_price,
        p.high_price,
        p.low_price,
        s_close.close AS close_price,
        get_market_type(s_open.open, p.high_price, p.low_price, s_close.close) AS classification
    FROM perweek p
    LEFT JOIN asset_daily_views s_open ON s_open.trading_date = p.start_trading_date AND s_open.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
    LEFT JOIN asset_daily_views s_close ON s_close.trading_date = p.end_trading_date AND s_close.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
)
INSERT INTO asset_weekly_views ( -- <<< GENERALIZED TABLE NAME
  week_start, asset_id, start_trading_date, end_trading_date,
  open, high, low, close, volume, bars, weekly_type,
  consolidation_subtype, high_trading_date, high_ts, high_session, 
  low_trading_date, low_ts, low_session
)
SELECT
  p.week_start,
  p.asset_id,
  p.start_trading_date,
  p.end_trading_date,
  s_open.open AS open,
  p.high_price,
  p.low_price,
  s_close.close AS close,
  p.volume_sum,
  p.bars,
  (cld.classification).session_type,
  (cld.classification).consolidation_subtype,
  p.high_trading_date,
  s_high.high_ts,
  s_high.high_session,
  p.low_trading_date,
  s_low.low_ts,
  s_low.low_session
FROM perweek p
LEFT JOIN asset_daily_views s_open ON s_open.trading_date = p.start_trading_date AND s_open.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
LEFT JOIN asset_daily_views s_close ON s_close.trading_date = p.end_trading_date AND s_close.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
LEFT JOIN asset_daily_views s_high ON s_high.trading_date = p.high_trading_date AND s_high.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
LEFT JOIN asset_daily_views s_low ON s_low.trading_date = p.low_trading_date AND s_low.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
LEFT JOIN classified cld ON cld.week_start = p.week_start AND cld.asset_id = p.asset_id;

----------------------------------
-- CTE: Monthly Views
----------------------------------
-- This CTE aggregates daily-level data to monthly-level for assets.
TRUNCATE asset_monthly_views; -- Use generalized table name

WITH base AS (
  SELECT trading_date, asset_id, open, high, low, close, volume, bars
  FROM asset_daily_views -- <<< GENERALIZED TABLE NAME
),
permonth AS (
  SELECT
    date_trunc('month', trading_date::timestamp)::date AS month_start,
    asset_id,
    MIN(trading_date) AS start_trading_date,
    MAX(trading_date) AS end_trading_date,
    MAX(high) AS high_price,
    MIN(low) AS low_price,
    SUM(volume) AS volume_sum,
    SUM(bars) AS bars,
    (array_agg(trading_date ORDER BY high DESC, trading_date ASC))[1] AS high_trading_date,
    (array_agg(trading_date ORDER BY low  ASC, trading_date ASC))[1] AS low_trading_date
  FROM asset_daily_views -- <<< GENERALIZED TABLE NAME
  GROUP BY 1, 2
),
classified AS (
    SELECT
        p.month_start,
        p.asset_id,
        s_open.open AS open_price,
        p.high_price,
        p.low_price,
        s_close.close AS close_price,
        get_market_type(s_open.open, p.high_price, p.low_price, s_close.close) AS classification
    FROM permonth p
    LEFT JOIN asset_daily_views s_open ON s_open.trading_date = p.start_trading_date AND s_open.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
    LEFT JOIN asset_daily_views s_close ON s_close.trading_date = p.end_trading_date AND s_close.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
)
INSERT INTO asset_monthly_views ( -- <<< GENERALIZED TABLE NAME
  month_start, asset_id, start_trading_date, end_trading_date,
  open, high, low, close, volume, bars, monthly_type,
  consolidation_subtype, high_trading_date, high_ts, high_session, 
  low_trading_date, low_ts, low_session
)
SELECT
  p.month_start,
  p.asset_id,
  p.start_trading_date,
  p.end_trading_date,
  s_open.open AS open,
  p.high_price,
  p.low_price,
  s_close.close AS close,
  p.volume_sum,
  p.bars,
  (cld.classification).session_type,
  (cld.classification).consolidation_subtype,
  p.high_trading_date,
  s_high.high_ts,
  s_high.high_session,
  p.low_trading_date,
  s_low.low_ts,
  s_low.low_session
FROM permonth p
LEFT JOIN asset_daily_views s_open ON s_open.trading_date = p.start_trading_date AND s_open.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
LEFT JOIN asset_daily_views s_close ON s_close.trading_date = p.end_trading_date AND s_close.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
LEFT JOIN asset_daily_views s_high ON s_high.trading_date = p.high_trading_date AND s_high.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
LEFT JOIN asset_daily_views s_low ON s_low.trading_date = p.low_trading_date AND s_low.asset_id = p.asset_id -- <<< GENERALIZED TABLE NAME
LEFT JOIN classified cld ON cld.month_start = p.month_start AND cld.asset_id = p.asset_id;

----------------------------------
-- End of CTEs & Functions for Table Views
----------------------------------