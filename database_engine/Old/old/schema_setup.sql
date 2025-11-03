--------------------------------------------------
TABLES
-------------------------------------------------

-- -----------------------------------------------------------
-- 1. ASSET TABLE (Reference Table)
-- -----------------------------------------------------------
DROP TABLE IF EXISTS assets CASCADE;
CREATE TABLE IF NOT EXISTS assets (
    id TEXT PRIMARY KEY,    
    symbol TEXT NOT NULL UNIQUE, 
    timezone TEXT NOT NULL, 
    source TEXT NOT NULL,
    UNIQUE(symbol, source),
    name TEXT, 
    asset_class TEXT,
    currency TEXT,
    exchange TEXT, 
    active BOOLEAN DEFAULT TRUE
);

-- -----------------------------------------------------------
-- 2. CORE HYPERTABLE (The Raw Data)
-- -----------------------------------------------------------
DROP TABLE asset_market_data;

TRUNCATE TABLE asset_market_data;
CREATE TABLE IF NOT EXISTS market_data (
    time TIMESTAMPTZ NOT NULL,
    asset_id TEXT NOT NULL REFERENCES assets(id),
    open DOUBLE PRECISION NOT NULL,
    high DOUBLE PRECISION NOT NULL,
    low DOUBLE PRECISION NOT NULL,
    close DOUBLE PRECISION NOT NULL,
    volume DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (time, asset_id)
);
----------------------------------------------------------------
--- 3 SESSION VIEW TABLE
-----------------------------------------------------------
DROP TABLE IF EXISTS us2000_session_views CASCADE;
CREATE TABLE IF NOT EXISTS us2000_session_views (
  trading_date date NOT NULL,
  asset_id text NOT NULL,
  session_name text NOT NULL,
  pattern text NOT NULL,
  start_ts timestamptz NOT NULL,
  end_ts timestamptz NOT NULL,
  open double precision,
  high double precision,
  high_ts timestamptz,
  low double precision,
  low_ts timestamptz,
  close double precision,
  volume double precision,
  bars bigint,
  PRIMARY KEY (trading_date, asset_id, session_name)
);

-- Refresh us2000_session_views from market_data_1hr (run after data load)
TRUNCATE us2000_session_views;

WITH base AS (
  SELECT
    time,
    asset_id,
    open, high,  low,  close, volume,
    custom_session_group(time) AS session_name,
    --PAT fun
    ((time AT TIME ZONE 'America/New_York') + INTERVAL '6 hours')::date AS trading_date
  FROM market_data_1hr
  -- optional: WHERE asset_id = 'assets:US2000'
),
per_session AS (
  SELECT
    trading_date,
    asset_id,
    session_name,
    MIN(time) AS start_ts,
    MAX(time) AS end_ts,
    MAX(high) AS high_price,
    (array_agg(time ORDER BY high DESC, time ASC))[1]   AS high_ts,
    MIN(low)  AS low_price,
    (array_agg(time ORDER BY low ASC, time ASC))[1]     AS low_ts,
    SUM(volume) AS volume_sum,
    COUNT(*) AS bars
  FROM base
  GROUP BY trading_date, asset_id, session_name
),
opens AS (
  SELECT b.trading_date, b.asset_id, b.session_name, b.open AS open_price
  FROM base b
  JOIN per_session p
    ON p.trading_date = b.trading_date
   AND p.asset_id = b.asset_id
   AND p.session_name = b.session_name
   AND p.start_ts = b.time
),
closes AS (
  SELECT b.trading_date, b.asset_id, b.session_name, b.close AS close_price
  FROM base b
  JOIN per_session p
    ON p.trading_date = b.trading_date
   AND p.asset_id = b.asset_id
   AND p.session_name = b.session_name
   AND p.end_ts = b.time
)
INSERT INTO us2000_session_views (trading_date, asset_id, session_name, start_ts, end_ts, open, high, high_ts, low, low_ts, close, volume, bars, pattern)
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
  get_candle_pattern(o.open_price, p.high_price, p.low_price, c.close_price) AS pattern
FROM per_session p
LEFT JOIN opens  o USING (trading_date, asset_id, session_name)
LEFT JOIN closes c USING (trading_date, asset_id, session_name);

-- -----------------------------------------------------------
-- 4. DAILY VIEW (updated: include high_ts, low_ts and session that produced them)
DROP TABLE IF EXISTS us2000_daily_views CASCADE;

CREATE TABLE IF NOT EXISTS us2000_daily_views (
  trading_date date NOT NULL,
  asset_id text NOT NULL,
  pattern text,
  open double precision,
  high double precision,
  high_ts timestamptz,
  high_session text,
  low double precision,
  low_ts timestamptz,
  low_session text,
  close double precision,
  volume bigint,
  bars bigint,
  PRIMARY KEY (trading_date, asset_id)
);

TRUNCATE us2000_daily_views;

WITH base AS (
  SELECT
    time, asset_id, open, high, low, close, volume,
    custom_session_group(time) AS session_name,
    ((time AT TIME ZONE 'America/New_York') + INTERVAL '6 hours')::date AS trading_date
  FROM market_data_1hr
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
)
INSERT INTO us2000_daily_views (trading_date, asset_id, pattern, open, high, high_ts, high_session, low, low_ts, low_session, close, volume, bars)
SELECT
  p.trading_date,
  p.asset_id,
  get_candle_pattern(o.open_price, p.high_price, p.low_price, c.close_price) AS pattern,
  o.open_price,
  p.high_price,
  hr.high_ts,
  hr.high_session,
  p.low_price,
  lr.low_ts,
  lr.low_session,
  c.close_price,
  p.volume_sum,
  p.bars
FROM perday p
LEFT JOIN opens o ON p.trading_date = o.trading_date AND p.asset_id = o.asset_id
LEFT JOIN closes c ON p.trading_date = c.trading_date AND p.asset_id = c.asset_id
LEFT JOIN high_row hr ON p.trading_date = hr.trading_date AND p.asset_id = hr.asset_id
LEFT JOIN low_row lr  ON p.trading_date = lr.trading_date AND p.asset_id = lr.asset_id;
COMMIT;
-----------------------------------------------------------
-- 5 WEEKLY VIEW TABLE
-- -----------------------------------------------------------
DROP TABLE IF EXISTS us2000_weekly_views CASCADE;
CREATE TABLE IF NOT EXISTS us2000_weekly_views (
  week_start date NOT NULL,
  asset_id text NOT NULL,
  start_trading_date date NOT NULL,
  end_trading_date date NOT NULL,
  weekly_pattern text,
  open double precision,
  high double precision,
  high_trading_date date,
  high_ts timestamptz,
  high_session text,
  low double precision,
  low_trading_date date,
  low_ts timestamptz,
  low_session text,
  close double precision,
  volume bigint,
  bars bigint,
  PRIMARY KEY (week_start, asset_id)
);

TRUNCATE us2000_weekly_views;

WITH base AS (
  SELECT trading_date, asset_id, open, high, low, close, volume, bars
  FROM us2000_daily_views
),
perweek AS (
  SELECT
    date_trunc('week', trading_date::timestamp)::date          AS week_start,
    asset_id,
    MIN(trading_date)                                        AS start_trading_date,
    MAX(trading_date)                                        AS end_trading_date,
    MAX(high)                                                AS high_price,
    MIN(low)                                                 AS low_price,
    SUM(volume)                                              AS volume_sum,
    SUM(bars)                                                AS bars,
    (array_agg(trading_date ORDER BY high DESC, trading_date ASC))[1] AS high_trading_date,
    (array_agg(trading_date ORDER BY low  ASC, trading_date ASC))[1] AS low_trading_date
  FROM base
  GROUP BY 1,2
)
INSERT INTO us2000_weekly_views (
  week_start, asset_id, start_trading_date, end_trading_date,
  open, high, low, close, volume, bars, weekly_pattern,
  high_trading_date, high_ts, high_session, low_trading_date, low_ts, low_session
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
  get_candle_pattern(s_open.open, p.high_price, p.low_price, s_close.close) AS weekly_pattern,
  p.high_trading_date,
  s_high.high_ts,
  s_high.high_session,
  p.low_trading_date,
  s_low.low_ts,
  s_low.low_session
FROM perweek p
LEFT JOIN us2000_daily_views s_open
  ON s_open.trading_date = p.start_trading_date AND s_open.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_close
  ON s_close.trading_date = p.end_trading_date   AND s_close.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_high
  ON s_high.trading_date = p.high_trading_date   AND s_high.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_low
  ON s_low.trading_date  = p.low_trading_date    AND s_low.asset_id = p.asset_id
ORDER BY p.week_start;
-----------------------------------------------------------
-- 6 MONTHLY VIEW TABLE
-- -----------------------------------------------------------
DROP TABLE IF EXISTS us2000_monthly_views CASCADE;
CREATE TABLE IF NOT EXISTS us2000_monthly_views (
  month_start date NOT NULL,
  asset_id text NOT NULL,
  start_trading_date date NOT NULL,
  end_trading_date date NOT NULL,
  monthly_pattern text,
  open double precision,
  high double precision,
  high_trading_date date,
  high_ts timestamptz,
  high_session text,
  low double precision,
  low_trading_date date,
  low_ts timestamptz,
  low_session text,
  close double precision,
  volume bigint,
  bars bigint,
  PRIMARY KEY (month_start, asset_id)
);

TRUNCATE us2000_monthly_views;

WITH base AS (
  SELECT trading_date, asset_id, open, high, low, close, volume, bars
  FROM us2000_daily_views
),
permonth AS (
  SELECT
    date_trunc('month', trading_date::timestamp)::date       AS month_start,
    asset_id,
    MIN(trading_date)                                        AS start_trading_date,
    MAX(trading_date)                                        AS end_trading_date,
    MAX(high)                                                AS high_price,
    MIN(low)                                                 AS low_price,
    SUM(volume)                                              AS volume_sum,
    SUM(bars)                                                AS bars,
    (array_agg(trading_date ORDER BY high DESC, trading_date ASC))[1] AS high_trading_date,
    (array_agg(trading_date ORDER BY low  ASC, trading_date ASC))[1] AS low_trading_date
  FROM base
  GROUP BY 1,2
)
INSERT INTO us2000_monthly_views (
  month_start, asset_id, start_trading_date, end_trading_date,
  open, high, low, close, volume, bars, monthly_pattern,
  high_trading_date, high_ts, high_session, low_trading_date, low_ts, low_session
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
  get_candle_pattern(s_open.open, p.high_price, p.low_price, s_close.close) AS monthly_pattern,
  p.high_trading_date,
  s_high.high_ts,
  s_high.high_session,
  p.low_trading_date,
  s_low.low_ts,
  s_low.low_session
FROM permonth p
LEFT JOIN us2000_daily_views s_open
  ON s_open.trading_date = p.start_trading_date AND s_open.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_close
  ON s_close.trading_date = p.end_trading_date   AND s_close.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_high
  ON s_high.trading_date = p.high_trading_date   AND s_high.asset_id = p.asset_id
LEFT JOIN us2000_daily_views s_low
  ON s_low.trading_date  = p.low_trading_date    AND s_low.asset_id = p.asset_id
ORDER BY p.month_start;

---------------event triggers-----------------------------
-- Note this line------
--------------------------------

TRUNCATE us2000_takedown_events;

-- =================================================================================
-- PART 0: DAILY COMPARISON (Used for PrevDay Levels and Cumulative Check)
-- =================================================================================
WITH daily_comparison AS (
    -- Get PrevDay Daily High/Low and check 3-day survival chain for cumulative check
    SELECT
        trading_date, asset_id,
        -- PrevDay Daily Levels
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS prev_day_high,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS prev_day_low,
        -- 2nd PrevDay Daily Levels for Cumulative Check
        LAG(high, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS prev_day_2_high,
        LAG(low, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS prev_day_2_low,
        -- CUMULATIVE HIGH CHECK (PrevDay High > 2nd PrevDay High)
        (LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) > 
         LAG(high, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AND 
         LAG(high, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) IS NOT NULL)
        AS is_cumulative_high,
        -- CUMULATIVE LOW CHECK (PrevDay Low < 2nd PrevDay Low)
        (LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) < 
         LAG(low, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AND 
         LAG(low, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) IS NOT NULL)
        AS is_cumulative_low
    FROM asset_daily_views
),
-- =================================================================================
-- PART 1: INTRA-DAY SEQUENTIAL TAKEDOWNS (AS -> LN, LN -> NYAM, etc.)
-- Logs breaks where Current Session takes Immediate Prior Session's H/L
-- =================================================================================
ordered_sessions AS (
    -- 1. Order Sessions Chronologically for LAG function
    SELECT
        trading_date, asset_id, session_name, high, low, start_ts,
        ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS session_rank
    FROM asset_session_views
),
sequential_comparison AS (
    -- 2. Use LAG to pull the immediately preceding session's data
    SELECT
        current.trading_date,
        current.asset_id,
        current.session_name AS breaker_session,
        current.high AS breaker_high,
        current.low AS breaker_low,
        LAG(current.session_name, 1) OVER w AS prior_session,
        LAG(current.high, 1) OVER w AS prior_high,
        LAG(current.low, 1) OVER w AS prior_low
    FROM ordered_sessions current
    WINDOW w AS (PARTITION BY current.trading_date, current.asset_id ORDER BY current.session_rank)
),
intra_day_events AS (
    -- 3. UNPIVOT High and Low Takedown events for intra-day breaks
    -- High Takedown event
    SELECT
        trading_date, asset_id, breaker_session, prior_session,
        'High' AS prior_level, prior_high AS prior_level_price, breaker_high AS breaker_price,
        FALSE AS cumulative_level 
    FROM sequential_comparison
    WHERE prior_session IS NOT NULL AND breaker_high > prior_high 
    
    UNION ALL 
    -- Low Takedown event
    SELECT
        trading_date, asset_id, breaker_session, prior_session,
        'Low' AS prior_level, prior_low AS prior_level_price, breaker_low AS breaker_price,
        FALSE AS cumulative_level
    FROM sequential_comparison
    WHERE prior_session IS NOT NULL AND breaker_low < prior_low
),
-- =================================================================================
-- PART 2: PREVIOUS DAY TAKEDOWNS (Daily Levels) - LOGIC CORRECTED FOR FIRST BREAKER
-- =================================================================================
prev_day_breakers AS (
    -- Identify ALL sessions on the current day that broke either the PrevDay High or Low
    SELECT
        s.trading_date,
        s.asset_id,
        s.session_name AS breaker_session, 
        'PrevDay-Daily' AS prior_session,
        CASE 
            WHEN s.high > dc.prev_day_high THEN 'High'
            WHEN s.low < dc.prev_day_low THEN 'Low'
            ELSE NULL
        END AS prior_level,
        CASE 
            WHEN s.high > dc.prev_day_high THEN dc.prev_day_high
            WHEN s.low < dc.prev_day_low THEN dc.prev_day_low
            ELSE NULL
        END AS prior_level_price,
        CASE 
            WHEN s.high > dc.prev_day_high THEN s.high
            WHEN s.low < dc.prev_day_low THEN s.low
            ELSE NULL
        END AS breaker_price,
        -- RE-INTEGRATED CUMULATIVE CHECK:
        CASE
            WHEN s.high > dc.prev_day_high THEN dc.is_cumulative_high
            WHEN s.low < dc.prev_day_low THEN dc.is_cumulative_low
            ELSE FALSE
        END AS cumulative_level,
        s.start_ts,
        -- Rank 1 = First session to break that specific level (High or Low) on the current day
        ROW_NUMBER() OVER (
            PARTITION BY s.trading_date, s.asset_id, 
            CASE 
                WHEN s.high > dc.prev_day_high THEN 'High' 
                WHEN s.low < dc.prev_day_low THEN 'Low' 
                ELSE NULL 
            END 
            ORDER BY s.start_ts ASC
        ) AS rank_num
    FROM asset_session_views s
    JOIN daily_comparison dc 
        ON dc.trading_date = s.trading_date 
        AND dc.asset_id = s.asset_id
    -- Filter out rows that broke neither level
    WHERE (s.high > dc.prev_day_high AND dc.prev_day_high IS NOT NULL)
       OR (s.low < dc.prev_day_low AND dc.prev_day_low IS NOT NULL)
)
-- =================================================================================
-- PART 3: FINAL INSERT (Combine All Events)
-- =================================================================================
INSERT INTO us2000_takedown_events (
    trading_date, asset_id, breaker_session, prior_session, prior_level, 
    prior_level_price, breaker_price, cumulative_level
)
-- 1. Insert Intra-Day Events
SELECT 
    trading_date, 
    asset_id, -- No prefix added, using simple ID
    breaker_session, prior_session, prior_level, 
    prior_level_price, breaker_price, cumulative_level
FROM intra_day_events
UNION ALL
-- 2. Insert PrevDay Events (First Breaker Only)
SELECT 
    trading_date, 
    asset_id, -- No prefix added, using simple ID
    breaker_session, prior_session, prior_level, 
    prior_level_price, breaker_price, cumulative_level
FROM prev_day_breakers
WHERE rank_num = 1; -- Crucially filters to only the first session to break the level

------------------------
-- follow through
----------------
-- Create the target table if it doesn't exist (assuming asset_id is the simple ID, e.g., 'US2000')
CREATE TABLE IF NOT EXISTS us2000_probability_matrix (
    breaker_session TEXT NOT NULL,
    prior_session TEXT NOT NULL,
    prior_level TEXT NOT NULL,
    total_takedowns BIGINT NOT NULL,
    ft_count BIGINT NOT NULL,
    fr_count BIGINT NOT NULL,
    n_count BIGINT NOT NULL,
    p_follow_through DOUBLE PRECISION NOT NULL,
    p_reversal DOUBLE PRECISION NOT NULL,
    p_neutral DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (breaker_session, prior_session, prior_level)
);

-- Clear the matrix before recalculating
TRUNCATE us2000_probability_matrix;

-- =================================================================================
-- SUB-TRACK 2: PROBABILITY MATRIX CALCULATION
-- =================================================================================
WITH numbered_sessions AS (
    -- 1. Order Sessions to easily find the Breaker Session's rank and subsequent sessions
    SELECT
        trading_date, asset_id, session_name, high, low, start_ts,
        ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS session_rank
    FROM us2000_session_views
),
event_details AS (
    -- 2. Combine Takedown Events (TDE) with Breaker Session (BS) data
    SELECT
        tde.trading_date,
        tde.asset_id,
        tde.breaker_session,
        tde.prior_session,
        tde.prior_level,
        tde.breaker_price,
        ns.session_rank AS bs_rank,
        ns.high AS bs_high,
        ns.low AS bs_low
    FROM us2000_takedown_events tde
    JOIN numbered_sessions ns
        ON tde.trading_date = ns.trading_date
        AND tde.asset_id = ns.asset_id
        AND tde.breaker_session = ns.session_name
),
subsequent_range AS (
    -- 3. Find the maximum High and minimum Low for all sessions AFTER the Breaker Session (BS)
    SELECT
        ed.trading_date,
        ed.asset_id,
        ed.breaker_session,
        ed.prior_session,
        ed.prior_level,
        ed.breaker_price,
        ed.bs_high,
        ed.bs_low,        
        -- Subsequent High: Max high of all sessions with a higher rank (i.e., later in the day)
        MAX(ns.high) OVER (
            PARTITION BY ns.trading_date, ns.asset_id
            ORDER BY ns.session_rank
            ROWS BETWEEN 1 FOLLOWING AND UNBOUNDED FOLLOWING
        ) AS subsequent_high,        
        -- Subsequent Low: Min low of all sessions with a higher rank (i.e., later in the day)
        MIN(ns.low) OVER (
            PARTITION BY ns.trading_date, ns.asset_id
            ORDER BY ns.session_rank
            ROWS BETWEEN 1 FOLLOWING AND UNBOUNDED FOLLOWING
        ) AS subsequent_low
        
    FROM event_details ed
    JOIN numbered_sessions ns
        ON ed.trading_date = ns.trading_date
        AND ed.asset_id = ns.asset_id
        AND ed.bs_rank = ns.session_rank -- Join back to BS row to use the window function properly
),
classified_events AS (
    -- 4. Classify each event outcome (FT, FR, or N)
    SELECT
        trading_date,
        asset_id,
        breaker_session,
        prior_session,
        prior_level,        
        -- Determine the outcome based on the logic design
        CASE
            -- Case 1: Takedown was UP (High Level Broken)
            WHEN prior_level IN ('High', 'Untaken-High', 'Daily-High') THEN
                CASE
                    -- FT: Subsequent price went HIGHER than the Breaker Price
                    WHEN subsequent_high > breaker_price THEN 'FT'
                    -- FR: Subsequent price took out the LOW of the Breaker Session
                    WHEN subsequent_low < bs_low THEN 'FR'
                    -- N: Neither FT nor FR occurred
                    ELSE 'N'
                END
                        -- Case 2: Takedown was DOWN (Low Level Broken)
            WHEN prior_level IN ('Low', 'Untaken-Low', 'Daily-Low') THEN
                CASE
                    -- FT: Subsequent price went LOWER than the Breaker Price
                    WHEN subsequent_low < breaker_price THEN 'FT'
                    -- FR: Subsequent price took out the HIGH of the Breaker Session
                    WHEN subsequent_high > bs_high THEN 'FR'
                    -- N: Neither FT nor FR occurred
                    ELSE 'N'
                END
            
            ELSE 'N' -- Default safety
        END AS outcome
        
    FROM subsequent_range
),
aggregated_counts AS (
    -- 5. Aggregate the counts by pattern (Breaker Session, Prior Session, Prior Level)
    SELECT
        breaker_session,
        prior_session,
        prior_level,
        COUNT(*) AS total_takedowns,
        SUM(CASE WHEN outcome = 'FT' THEN 1 ELSE 0 END) AS ft_count,
        SUM(CASE WHEN outcome = 'FR' THEN 1 ELSE 0 END) AS fr_count,
        SUM(CASE WHEN outcome = 'N' THEN 1 ELSE 0 END) AS n_count
    FROM classified_events
    GROUP BY 1, 2, 3
)
-- 6. FINAL INSERT: Calculate Probabilities and store the matrix
INSERT INTO us2000_probability_matrix (
    breaker_session, prior_session, prior_level, 
    total_takedowns, ft_count, fr_count, n_count, 
    p_follow_through, p_reversal, p_neutral
)
SELECT
    breaker_session,
    prior_session,
    prior_level,
    total_takedowns,
    ft_count,
    fr_count,
    n_count,
    -- P(Follow-Through)
    ROUND((ft_count::NUMERIC / total_takedowns) * 100, 2) AS p_follow_through,
    -- P(Reversal)
    ROUND((fr_count::NUMERIC / total_takedowns) * 100, 2) AS p_reversal,
    -- P(Neutral)
    ROUND((n_count::NUMERIC / total_takedowns) * 100, 2) AS p_neutral
FROM aggregated_counts
WHERE total_takedowns > 0;

-----------------------------------------------------
-- Taken logic (pattern is defined)
----------------------------------------------------
DROP TABLE IF EXISTS us2000_conditional_matrix CASCADE;
-- Create the target table for the conditional matrix
CREATE TABLE IF NOT EXISTS us2000_conditional_matrix (
    prior_session TEXT NOT NULL,
    ps_bias TEXT NOT NULL,
    breaker_session TEXT NOT NULL,
    bs_bias TEXT NOT NULL,
    prior_level TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    break_count BIGINT NOT NULL,
    p_break DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (prior_session, ps_bias, breaker_session, bs_bias, prior_level)
);

-- Clear the matrix before recalculating
TRUNCATE us2000_conditional_matrix;

-- =================================================================================
-- STEP 1: SESSION BIAS TAGGING (S1)
-- =================================================================================
WITH session_bias_tagged AS (
    -- 1. Get current and preceding session data (PS) for all sessions
    SELECT
        current.trading_date,
        current.asset_id,
        current.session_name,
        current.start_ts,       
        LAG(current.high, 1) OVER w AS prev_high,
        LAG(current.low, 1) OVER w AS prev_low,        
        -- Apply the three-way bias tagging logic (Bullish, Bearish, Consolidation)
        CASE
            -- Bullish: Took out Prev High, did NOT take out Prev Low
            WHEN current.high > LAG(current.high, 1) OVER w 
             AND current.low >= LAG(current.low, 1) OVER w
            THEN 'Bullish'            
            -- Bearish: Took out Prev Low, did NOT take out Prev High
            WHEN current.low < LAG(current.low, 1) OVER w 
             AND current.high <= LAG(current.high, 1) OVER w
            THEN 'Bearish'            
            -- Consolidation: Everything else (inside day, or two-sided expansion/whipsaw)
            ELSE 'Consolidation'
        END AS session_bias        
    FROM us2000_session_views current
    -- Define window partition for ordering sessions sequentially across days
    WINDOW w AS (PARTITION BY current.asset_id ORDER BY current.start_ts)
),
-- =================================================================================
-- STEP 2: EVENT DATA ENRICHMENT (N - NUMERATOR)
-- =================================================================================
event_enriched AS (
    -- 2. Join Takedown Events (TDE) with tagged sessions to get the joint bias (PS_Bias and BS_Bias)
    SELECT
        tde.trading_date,
        tde.asset_id,
        tde.prior_session AS ps_name,
        tde.breaker_session AS bs_name,
        tde.prior_level,
        
        ps.session_bias AS ps_bias, -- Bias of the Prior Session
        bs.session_bias AS bs_bias  -- Bias of the Breaker Session
        
    FROM us2000_takedown_events tde
    -- Join 1: Get the BIAS of the PRIOR SESSION (PS)
    JOIN session_bias_tagged ps 
        ON tde.trading_date = ps.trading_date 
        AND tde.asset_id = ps.asset_id 
        AND tde.prior_session = ps.session_name
    -- Join 2: Get the BIAS of the BREAKER SESSION (BS)
    JOIN session_bias_tagged bs 
        ON tde.trading_date = bs.trading_date 
        AND tde.asset_id = bs.asset_id 
        AND tde.breaker_session = bs.session_name    
    -- Exclude any rows where either PS or BS could not be tagged (e.g., first ever session)
    WHERE ps.session_bias IS NOT NULL AND bs.session_bias IS NOT NULL
),
break_counts AS (
    -- 3. Calculate the NUMERATOR (N): Count of specific breaks for each joint pattern
    SELECT
        ps_name,
        ps_bias,
        bs_name,
        bs_bias,
        prior_level,
        COUNT(*) AS break_count
    FROM event_enriched
    GROUP BY 1, 2, 3, 4, 5
),
-- =================================================================================
-- STEP 3: DENOMINATOR AND FINAL CALCULATION (D - DENOMINATOR)
-- =================================================================================
pattern_attempts AS (
    -- 4. Calculate the DENOMINATOR (D): Total times the joint pattern was available
    -- This requires counting the *sequence* of (PS, PS_Bias) followed by (BS, BS_Bias)
    SELECT
        ps.session_name AS ps_name,
        ps.session_bias AS ps_bias,
        cs.session_name AS bs_name,
        cs.session_bias AS bs_bias,
        COUNT(*) AS total_attempts
        
    FROM session_bias_tagged ps -- Prior Session (PS)
    JOIN session_bias_tagged cs -- Current/Breaker Session (BS)
        ON ps.asset_id = cs.asset_id
        -- The PS must immediately precede the CS/BS
        -- We must ensure cs.start_ts is the *next* sequential session after ps.start_ts
        -- This is achieved implicitly by relying on session_name sequencing or explicitly checking start_ts sequence
        -- For simplicity and relying on session naming conventions (AS->LN, LN->NYAM), we filter sessions logically:
        AND (
            (ps.session_name = 'AS' AND cs.session_name = 'LN') OR
            (ps.session_name = 'LN' AND cs.session_name = 'NYAM') OR
            (ps.session_name = 'NYAM' AND cs.session_name = 'NYPL') OR
            (ps.session_name = 'NYL' AND cs.session_name = 'NYPM') OR
            (ps.session_name = 'NYPM' AND cs.session_name = 'AS' AND cs.trading_date = ps.trading_date + INTERVAL '1 day')
        )
        AND (ps.trading_date = cs.trading_date OR ps.session_name = 'NYPM') 
    WHERE ps.session_bias IS NOT NULL AND cs.session_bias IS NOT NULL
    GROUP BY 1, 2, 3, 4
)
-- 5. FINAL INSERT: Join N and D and compute the conditional probability P=N/D
INSERT INTO us2000_conditional_matrix (
    prior_session, ps_bias, breaker_session, bs_bias, prior_level, 
    total_attempts, break_count, p_break
)
SELECT
    n.ps_name,
    n.ps_bias,
    n.bs_name,
    n.bs_bias,
    n.prior_level,
    d.total_attempts,
    n.break_count,
    -- P(Break) = N / D
    ROUND((n.break_count::NUMERIC / d.total_attempts) * 100, 2) AS p_break
FROM break_counts n
JOIN pattern_attempts d
    ON n.ps_name = d.ps_name
    AND n.ps_bias = d.ps_bias
    AND n.bs_name = d.bs_name
    AND n.bs_bias = d.bs_bias
-- Exclude patterns with zero attempts (denominator) to prevent division by zero, although COUNT(*) should prevent this.
WHERE d.total_attempts > 0;

-- --------------
-- Taken, with session candle 
----------------------------------

-- Assuming the session view has columns: trading_date, asset_id, session_name, market_type (or similar)
-- We will assume the fourth column is the session bias, and name it 'market_type_bias'.

DROP TABLE IF EXISTS us2000_conditional_matrix_1 CASCADE;

-- Create the target table for the conditional matrix
CREATE TABLE IF NOT EXISTS us2000_conditional_matrix (
    prior_session TEXT NOT NULL,
    ps_bias TEXT NOT NULL,
    breaker_session TEXT NOT NULL,
    bs_bias TEXT NOT NULL,
    prior_level TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    break_count BIGINT NOT NULL,
    p_break DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (prior_session, ps_bias, breaker_session, bs_bias, prior_level)
);

-- Clear the matrix before recalculating
TRUNCATE us2000_conditional_matrix_1;

-- =================================================================================
-- STEP 1 (SIMPLIFIED): SESSION BIAS SELECTION
-- =================================================================================
WITH session_bias_source AS (
    -- Select the pre-calculated market bias directly from the session views
    SELECT
        trading_date,
        asset_id,
        session_name,
        -- Assuming the bias is in the column after session_name (e.g., column 4 in your sample)
        -- We alias it as 'session_bias'
        session_type AS session_bias,
        start_ts
    FROM us2000_session_views t -- Alias us2000_session_views as 't' for brevity
    -- Filter out any sessions without a defined bias if necessary, though NULL checks in joins should suffice
),
-- =================================================================================
-- STEP 2: EVENT DATA ENRICHMENT (N - NUMERATOR)
-- =================================================================================
event_enriched AS (
    -- 1. Join Takedown Events (TDE) with tagged sessions to get the joint bias (PS_Bias and BS_Bias)
    SELECT
        tde.trading_date,
        tde.asset_id,
        tde.prior_session AS ps_name,
        tde.breaker_session AS bs_name,
        tde.prior_level,
        
        ps.session_bias AS ps_bias, -- Bias of the Prior Session
        bs.session_bias AS bs_bias  -- Bias of the Breaker Session
        
    FROM us2000_takedown_events tde
    -- Join 1: Get the BIAS of the PRIOR SESSION (PS)
    JOIN session_bias_source ps 
        ON tde.trading_date = ps.trading_date 
        AND tde.asset_id = ps.asset_id 
        AND tde.prior_session = ps.session_name
    -- Join 2: Get the BIAS of the BREAKER SESSION (BS)
    JOIN session_bias_source bs 
        ON tde.trading_date = bs.trading_date 
        AND tde.asset_id = bs.asset_id 
        AND tde.breaker_session = bs.session_name    
    -- Filter out any row where a bias tag was missing (though unlikely with this design)
    WHERE ps.session_bias IS NOT NULL AND bs.session_bias IS NOT NULL
),
break_counts AS (
    -- 2. Calculate the NUMERATOR (N): Count of specific breaks for each joint pattern
    SELECT
        ps_name,
        ps_bias,
        bs_name,
        bs_bias,
        prior_level,
        COUNT(*) AS break_count
    FROM event_enriched
    GROUP BY 1, 2, 3, 4, 5
),
-- =================================================================================
-- STEP 3: DENOMINATOR AND FINAL CALCULATION (D - DENOMINATOR)
-- =================================================================================
pattern_attempts AS (
    -- 3. Calculate the DENOMINATOR (D): Total times the joint pattern was available
    -- Total count of times the sequence (PS, PS_Bias) -> (BS, BS_Bias) occurred.
    SELECT
        ps.session_name AS ps_name,
        ps.session_bias AS ps_bias,
        cs.session_name AS bs_name,
        cs.session_bias AS bs_bias,
        COUNT(*) AS total_attempts
        
    FROM session_bias_source ps -- Prior Session (PS)
    JOIN session_bias_source cs -- Current/Breaker Session (BS)
        ON ps.asset_id = cs.asset_id
        -- Match the correct sequential pair (e.g., AS followed by LN)
        AND (
            (ps.session_name = 'AS' AND cs.session_name = 'LN') OR
            (ps.session_name = 'LN' AND cs.session_name = 'NYAM') OR
            (ps.session_name = 'NYAM' AND cs.session_name = 'NYL') OR
            (ps.session_name = 'NYL' AND cs.session_name = 'NYPM') OR
            -- Handle the day transition (NYPM followed by next day AS) - requires matching dates correctly
            (ps.session_name = 'NYPM' AND cs.session_name = 'AS' AND cs.trading_date = ps.trading_date + INTERVAL '1 day')
        )
        -- Ensure we are matching the correct day's sequence (excluding the day transition case above)
        AND (ps.trading_date = cs.trading_date OR ps.session_name = 'NYPM') 
    GROUP BY 1, 2, 3, 4
)
-- 4. FINAL INSERT: Join N and D and compute the conditional probability P=N/D
INSERT INTO us2000_conditional_matrix_1 (
    prior_session, ps_bias, breaker_session, bs_bias, prior_level, 
    total_attempts, break_count, p_break
)
SELECT
    n.ps_name,
    n.ps_bias,
    n.bs_name,
    n.bs_bias,
    n.prior_level,
    d.total_attempts,
    n.break_count,
    -- P(Break) = N / D
    ROUND((n.break_count::NUMERIC / d.total_attempts) * 100, 2) AS p_break
FROM break_counts n
JOIN pattern_attempts d
    ON n.ps_name = d.ps_name
    AND n.ps_bias = d.ps_bias
    AND n.bs_name = d.bs_name
    AND n.bs_bias = d.bs_bias
WHERE d.total_attempts > 0;