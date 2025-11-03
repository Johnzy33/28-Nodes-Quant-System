-- -----------------------------------------------------------
-- 3. SESSION VIEW REFRESH (WITH NEW CLASSIFICATION)
-- -----------------------------------------------------------
TRUNCATE us2000_session_view;

WITH base AS (
  SELECT
    time,
    asset_id,
    open, high,  low,  close, volume,
    custom_session_group(time) AS session_name,
    get_trading_date(time) AS trading_date -- Use the function
  FROM asset_market_data -- Corrected table name
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
  GROUP BY trading_date, asset_id, session_name
),
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
INSERT INTO us2000_session_views (
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


-- -----------------------------------------------------------
-- 4. DAILY VIEW REFRESH (WITH NEW CLASSIFICATION)
-- -----------------------------------------------------------
TRUNCATE us2000_daily_views;

WITH base AS (
  SELECT
    time, asset_id, open, high, low, close, volume,
    custom_session_group(time) AS session_name,
    get_trading_date(time) AS trading_date -- Use the function
  FROM asset_market_data -- Corrected table name
),
perday AS (
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
INSERT INTO us2000_daily_views (
    trading_date, asset_id, open, high, high_ts, high_session, 
    low, low_ts, low_session, close, volume, bars, 
    daily_type, consolidation_subtype
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


-- -----------------------------------------------------------
-- 5. WEEKLY VIEW REFRESH (WITH NEW CLASSIFICATION)
-- -----------------------------------------------------------
TRUNCATE us2000_weekly_views;

WITH base AS (
  SELECT trading_date, asset_id, open, high, low, close
  FROM us2000_daily_views -- Aggregating from the Daily View
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
  FROM asset_daily_views
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
    LEFT JOIN us2000_daily_views s_open ON s_open.trading_date = p.start_trading_date AND s_open.asset_id = p.asset_id
    LEFT JOIN us2000_daily_views s_close ON s_close.trading_date = p.end_trading_date AND s_close.asset_id = p.asset_id
)
INSERT INTO us2000_weekly_views (
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
LEFT JOIN us2000_daily_views s_open ON s_open.trading_date = p.start_trading_date AND s_open.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_close ON s_close.trading_date = p.end_trading_date AND s_close.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_high ON s_high.trading_date = p.high_trading_date AND s_high.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_low ON s_low.trading_date = p.low_trading_date AND s_low.asset_id = p.asset_id
LEFT JOIN classified cld ON cld.week_start = p.week_start AND cld.asset_id = p.asset_id;


-- -----------------------------------------------------------
-- 6. MONTHLY VIEW REFRESH (WITH NEW CLASSIFICATION)
-- -----------------------------------------------------------
TRUNCATE us2000_monthly_views;

WITH base AS (
  SELECT trading_date, asset_id, open, high, low, close
  FROM us2000_daily_views -- Aggregating from the Daily View
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
  FROM us2000_daily_views
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
    LEFT JOIN us2000_daily_views s_open ON s_open.trading_date = p.start_trading_date AND s_open.asset_id = p.asset_id
    LEFT JOIN us2000_daily_views s_close ON s_close.trading_date = p.end_trading_date AND s_close.asset_id = p.asset_id
)
INSERT INTO us2000_monthly_views (
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
LEFT JOIN us2000_daily_views s_open ON s_open.trading_date = p.start_trading_date AND s_open.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_close ON s_close.trading_date = p.end_trading_date AND s_close.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_high ON s_high.trading_date = p.high_trading_date AND s_high.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_low ON s_low.trading_date = p.low_trading_date AND s_low.asset_id = p.asset_id
LEFT JOIN classified cld ON cld.month_start = p.month_start AND cld.asset_id = p.asset_id;
-- -----------------------------------------------------------
-- 7. EVENT TABLE REFRESH
-- -----------------------------------------------------------
TRUNCATE us2000_takedown_events;

-- =================================================================================
-- BASE CTEs (from previous conversation - ensuring they are all here)
-- =================================================================================
WITH daily_comparison AS (
    -- Calculates PrevDay Daily H/L and 3-day Cumulative flags
    SELECT
        trading_date, asset_id,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS prev_day_high,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS prev_day_low,
        LAG(high, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS prev_day_2_high,
        LAG(low, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS prev_day_2_low,
        (LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) > 
         LAG(high, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AND 
         LAG(high, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) IS NOT NULL)
        AS is_cumulative_high,
        (LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) < 
         LAG(low, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AND 
         LAG(low, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) IS NOT NULL)
        AS is_cumulative_low
    FROM us2000_daily_views
),
ordered_sessions AS (
    -- Orders sessions for Intra-Day comparisons
    SELECT
        trading_date, asset_id, session_name, high, low, start_ts,
        ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS session_rank
    FROM us2000_session_views
),
sequential_comparison AS (
    -- Finds immediate prior session's H/L for Intra-Day
    SELECT
        current.trading_date, current.asset_id, current.session_name AS breaker_session,
        current.high AS breaker_high, current.low AS breaker_low,
        LAG(current.session_name, 1) OVER w AS prior_session,
        LAG(current.high, 1) OVER w AS prior_high,
        LAG(current.low, 1) OVER w AS prior_low
    FROM ordered_sessions current
    WINDOW w AS (PARTITION BY current.trading_date, current.asset_id ORDER BY current.session_rank)
),
intra_day_events AS (
    -- Final Intra-Day events
    SELECT
        trading_date, asset_id, breaker_session, prior_session,
        'High' AS prior_level, prior_high AS prior_level_price, breaker_high AS breaker_price,
        FALSE AS cumulative_level 
    FROM sequential_comparison WHERE prior_session IS NOT NULL AND breaker_high > prior_high 
    UNION ALL
    SELECT
        trading_date, asset_id, breaker_session, prior_session,
        'Low' AS prior_level, prior_low AS prior_level_price, breaker_low AS breaker_low,
        FALSE AS cumulative_level
    FROM sequential_comparison WHERE prior_session IS NOT NULL AND breaker_low < prior_low
),
prev_day_daily_breakers AS (
    -- PrevDay Daily H/L Takedowns (FIRST BREAKER LOGIC)
    SELECT
        s.trading_date, s.asset_id, s.session_name AS breaker_session, 
        'PrevDay-Daily' AS prior_session,
        CASE WHEN s.high > dc.prev_day_high THEN 'High' ELSE 'Low' END AS prior_level,
        CASE WHEN s.high > dc.prev_day_high THEN dc.prev_day_high ELSE dc.prev_day_low END AS prior_level_price,
        CASE WHEN s.high > dc.prev_day_high THEN s.high ELSE s.low END AS breaker_price,
        CASE WHEN s.high > dc.prev_day_high THEN dc.is_cumulative_high ELSE dc.is_cumulative_low END AS cumulative_level,
        s.start_ts,
        ROW_NUMBER() OVER (
            PARTITION BY s.trading_date, s.asset_id, CASE WHEN s.high > dc.prev_day_high THEN 'High' ELSE 'Low' END 
            ORDER BY s.start_ts ASC
        ) AS rank_num
    FROM us2000_session_views s
    JOIN daily_comparison dc ON dc.trading_date = s.trading_date AND dc.asset_id = s.asset_id
    WHERE (s.high > dc.prev_day_high AND dc.prev_day_high IS NOT NULL) OR 
          (s.low < dc.prev_day_low AND dc.prev_day_low IS NOT NULL)
),
-- =================================================================================
-- NEW CTEs: PREVDAY UNTAKEN SESSION LEVELS
-- =================================================================================
untaken_levels_yesterday AS (
    -- Check if a session's H/L survived the remainder of its own trading day
    SELECT
        current.trading_date, current.asset_id, current.session_name, current.high AS session_high, current.low AS session_low,
        MAX(current.high) OVER (PARTITION BY current.trading_date, current.asset_id ORDER BY current.start_ts ROWS BETWEEN 1 FOLLOWING AND UNBOUNDED FOLLOWING) AS subsequent_day_high,
        MIN(current.low) OVER (PARTITION BY current.trading_date, current.asset_id ORDER BY current.start_ts ROWS BETWEEN 1 FOLLOWING AND UNBOUNDED FOLLOWING) AS subsequent_day_low
    FROM us2000_session_views current
),
prior_day_survivors AS (
    -- Final list of levels that survived until the end of the prior day
    SELECT
        trading_date AS prior_trading_date, asset_id, session_name AS prior_session_name,
        CASE WHEN subsequent_day_high IS NULL OR session_high > subsequent_day_high THEN session_high ELSE NULL END AS untaken_high_price,
        CASE WHEN subsequent_day_low IS NULL OR session_low < subsequent_day_low THEN session_low ELSE NULL END AS untaken_low_price
    FROM untaken_levels_yesterday
    WHERE (session_high > subsequent_day_high OR subsequent_day_high IS NULL) 
       OR (session_low < subsequent_day_low OR subsequent_day_low IS NULL)
),
current_day_takedown AS (
    -- Join current day sessions with the prior day's surviving levels
    SELECT
        s.trading_date AS current_trading_date, s.asset_id, s.session_name AS breaker_session, s.start_ts,
        pds.prior_session_name, pds.untaken_high_price, pds.untaken_low_price,
        s.high AS breaker_high, s.low AS breaker_low
    FROM us2000_session_views s
    JOIN prior_day_survivors pds ON 
        pds.asset_id = s.asset_id AND
        pds.prior_trading_date = (s.trading_date - INTERVAL '1 day') -- ***DATE MATH ASSUMPTION***
    WHERE (s.high > pds.untaken_high_price AND pds.untaken_high_price IS NOT NULL)
       OR (s.low < pds.untaken_low_price AND pds.untaken_low_price IS NOT NULL)
),
prev_day_untaken_events AS (
    -- Rank the current day's takedowns to log only the FIRST one
    SELECT
        current_trading_date AS trading_date, asset_id, breaker_session, prior_session_name AS prior_session,
        CASE WHEN breaker_high > untaken_high_price THEN 'Untaken-High' ELSE 'Untaken-Low' END AS prior_level,
        CASE WHEN breaker_high > untaken_high_price THEN untaken_high_price ELSE untaken_low_price END AS prior_level_price,
        CASE WHEN breaker_high > untaken_high_price THEN breaker_high ELSE breaker_low END AS breaker_price,
        FALSE AS cumulative_level,
        ROW_NUMBER() OVER (
            PARTITION BY current_trading_date, asset_id, prior_session_name, 
            CASE WHEN breaker_high > untaken_high_price THEN 'High' ELSE 'Low' END 
            ORDER BY start_ts ASC
        ) AS rank_num
    FROM current_day_takedown
)
-- =================================================================================
-- FINAL INSERT STATEMENT: UNION ALL 3 types of events
-- =================================================================================
INSERT INTO us2000_takedown_events (
    trading_date, asset_id, breaker_session, prior_session, prior_level, 
    prior_level_price, breaker_price, cumulative_level
)
-- 1. Intra-Day Events
SELECT 
    trading_date, asset_id, breaker_session, prior_session, prior_level, 
    prior_level_price, breaker_price, cumulative_level
FROM intra_day_events
UNION ALL
-- 2. PrevDay Daily H/L Events (First Breaker Only)
SELECT 
    trading_date, asset_id, breaker_session, prior_session, prior_level, 
    prior_level_price, breaker_price, cumulative_level
FROM prev_day_daily_breakers
WHERE rank_num = 1
UNION ALL
-- 3. PrevDay UNTAKEN SESSION H/L Events (First Breaker Only)
SELECT 
    trading_date, asset_id, breaker_session, prior_session, prior_level, 
    prior_level_price, breaker_price, cumulative_level
FROM prev_day_untaken_events
WHERE rank_num = 1;
COMMIT;