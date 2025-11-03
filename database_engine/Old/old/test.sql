SELECT create_hypertable('us2000_market_data', 'time', if_not_exists => TRUE);
-- Index on asset_id for faster queries filtering by asset
CREATE INDEX IF NOT EXISTS idx_asset_id ON us2000_market_data (asset_id);


-- -----------------------------------------------------------
-- 3. CUSTOM FUNCTION: SESSION GROUPING (DST Aware)
-- -----------------------------------------------------------
-- Groups a timestamp into your custom trading session (AS, LN, NYAM, etc.).
-- Uses the 'America/New_York' timezone to automatically handle DST.

CREATE OR REPLACE FUNCTION custom_session_group(ts TIMESTAMPTZ)
RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $$
    -- Convert the TIMESTAMPTZ to the hour it represents in the New York Time Zone.
    SELECT CASE 
        -- LN: London Session (2:00 to 7:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 2 AND 7 THEN 'LN'    
        
        -- NYAM: New York AM (8:00 to 11:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 8 AND 11 THEN 'NYAM' 
        
        -- NYL: New York Lunch (12:00 to 13:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 12 AND 13 THEN 'NYL'  
        
        -- NYPM: New York PM (14:00 to 17:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 14 AND 17 THEN 'NYPM' 
        
        -- AS: Asia Session (Wraps midnight: 6 PM to 1 AM NY Time)
        -- Part 1: End of Day (18:00 to 23:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 18 AND 23 THEN 'AS'     
        
        -- Part 2: Start of Day (00:00 to 01:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 0 AND 1 THEN 'AS'     
        
        ELSE 'Unknown'
    END;
$$;


-- -----------------------------------------------------------
-- 4. CONTINUOUS AGGREGATES (For Time-Series Analysis)
-- -----------------------------------------------------------

-- Helper function to select the time associated with the min/max value
-- Uses array_agg to order by the target column (e.g., high DESC) and takes the first element.
CREATE OR REPLACE FUNCTION time_of_extremum(vals DOUBLE PRECISION[], times TIMESTAMPTZ[])
RETURNS TIMESTAMPTZ
LANGUAGE SQL IMMUTABLE AS $$
    SELECT (array_agg(t ORDER BY v DESC))[1]
    FROM unnest(vals, times) AS t(v, t);
$$;

-- A. Daily Continuous Aggregate
DROP MATERIALIZED VIEW IF EXISTS daily CASCADE;
CREATE MATERIALIZED VIEW daily
WITH (timescaledb.continuous)
AS
    SELECT
        time_bucket('1 day', time) AS time,
        asset_id,
        first(open, time) AS open,
        max(high) AS high,
        (array_agg(time ORDER BY high DESC, time ASC))[1] AS time_of_high,
        min(low) AS low,
        (array_agg(time ORDER BY low ASC, time ASC))[1] AS time_of_low,
        last(close, time) AS close,
        sum(volume) AS volume,
        NULL::TEXT AS daily_pattern
    FROM market_data_1hr
    GROUP BY 1, 2;
SELECT add_continuous_aggregate_policy('daily',
    start_offset => INTERVAL '2 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '10 minutes', if_not_exists => TRUE);


-- B. Weekly Continuous Aggregate
DROP MATERIALIZED VIEW IF EXISTS weekly CASCADE;
CREATE MATERIALIZED VIEW weekly
WITH (timescaledb.continuous)
AS
    SELECT
        time_bucket('1 week', time) AS time,
        asset_id,
        first(open, time) AS open,
        max(high) AS high,
        (array_agg(time ORDER BY high DESC, time ASC))[1] AS time_of_high,
        min(low) AS low,
        (array_agg(time ORDER BY low ASC, time ASC))[1] AS time_of_low,
        last(close, time) AS close,
        sum(volume) AS volume,
        NULL::TEXT AS weekly_pattern
    FROM market_data_1hr
    GROUP BY 1, 2;
SELECT add_continuous_aggregate_policy('weekly',
    start_offset => INTERVAL '2 months',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day', if_not_exists => TRUE);


-- C. Monthly Continuous Aggregate
DROP MATERIALIZED VIEW IF EXISTS monthly CASCADE;
CREATE MATERIALIZED VIEW monthly
WITH (timescaledb.continuous)
AS
    SELECT
        time_bucket('1 month', time) AS time,
        asset_id,
        first(open, time) AS open,
        max(high) AS high,
        (array_agg(time ORDER BY high DESC, time ASC))[1] AS time_of_high,
        min(low) AS low,
        (array_agg(time ORDER BY low ASC, time ASC))[1] AS time_of_low,
        last(close, time) AS close,
        sum(volume) AS volume,
        NULL::TEXT AS monthly_pattern
    FROM market_data_1hr
    GROUP BY 1, 2;
SELECT add_continuous_aggregate_policy('monthly',
    start_offset => INTERVAL '1 year',
    end_offset => INTERVAL '1 month',
    schedule_interval => INTERVAL '1 week', if_not_exists => TRUE);


-- D. Custom Session Continuous Aggregate
DROP MATERIALIZED VIEW IF EXISTS session CASCADE;
CREATE MATERIALIZED VIEW session
WITH (timescaledb.continuous)
AS
    SELECT
        -- Group by the custom session definition
        time_bucket(
            INTERVAL '1 hour', time
        ) AS time_start,
        custom_session_group(time) AS session_name,
        asset_id,
        first(open, time) AS open,
        max(high) AS high,
        (array_agg(time ORDER BY high DESC, time ASC))[1] AS time_of_high,
        min(low) AS low,
        (array_agg(time ORDER BY low ASC, time ASC))[1] AS time_of_low,
        last(close, time) AS close,
        sum(volume) AS volume,
        NULL::TEXT AS session_pattern
    FROM market_data_1hr
    GROUP BY 1, 2, 3;
SELECT add_continuous_aggregate_policy('session',
    start_offset => INTERVAL '2 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '10 minutes', if_not_exists => TRUE);


-- -----------------------------------------------------------
-- 5. WEEKLY FEATURES TABLE (Analytical Destination)
-- -----------------------------------------------------------
-- Stores derived features for backtesting and probability analysis.

CREATE TABLE IF NOT EXISTS weekly_features (
    time TIMESTAMPTZ NOT NULL,
    asset_id TEXT NOT NULL REFERENCES assets(id),

    -- Weekly OHLCV and Pattern (From candles_weekly)
    weekly_pattern TEXT, 
    weekly_open DOUBLE PRECISION,
    weekly_high DOUBLE PRECISION,
    weekly_low DOUBLE PRECISION,
    weekly_close DOUBLE PRECISION,
    weekly_volume DOUBLE PRECISION,

    -- Weekday Patterns (From candles_daily, 0=Sunday, 1=Monday, etc.)
    dow_0_pattern TEXT, -- Sunday
    dow_1_pattern TEXT, -- Monday
    dow_2_pattern TEXT, -- Tuesday
    dow_3_pattern TEXT, -- Wednesday
    dow_4_pattern TEXT, -- Thursday
    dow_5_pattern TEXT, -- Friday
    dow_6_pattern TEXT, -- Saturday
    
    PRIMARY KEY (time, asset_id)
);

SELECT create_hypertable('weekly_features', 'time', if_not_exists => TRUE);

-----------------------------------------------------------
--- Session View---
CREATE TABLE IF NOT EXISTS session_aggregate_view (
            time TIMESTAMPTZ NOT NULL,
            asset_id TEXT NOT NULL,
            session_name TEXT NOT NULL,
            session_pattern TEXT,
            open DOUBLE PRECISION NOT NULL,
            high DOUBLE PRECISION NOT NULL,
            low DOUBLE PRECISION NOT NULL,
            close DOUBLE PRECISION NOT NULL,
            volume DOUBLE PRECISION NOT NULL,
            high_time TIMESTAMPTZ,              -- Time of session high
            low_time TIMESTAMPTZ,               -- Time of session low
            PRIMARY KEY (time, asset_id, session_name),
            CONSTRAINT fk_asset_session
                FOREIGN KEY (asset_id)
                REFERENCES assets (id)
        );

DROP MATERIALIZED VIEW IF EXISTS session_aggregate_view CASCADE;
CREATE MATERIALIZED VIEW session_aggregate_view
    WITH (timescaledb.continuous)
    AS
        SELECT
            time_bucket('1 day', time) AS time,
            asset_id,
            custom_session_group(time) AS session_name,
            NULL::TEXT AS session_pattern,
            first(open, time) AS open,
            max(high) AS high,
            min(low) AS low,
            last(close, time) AS close,
            sum(volume) AS volume,
            (array_agg(time ORDER BY high DESC, time ASC))[1] AS high_time,
            (array_agg(time ORDER BY low ASC, time ASC))[1] AS low_time
        FROM market_data_1hr
        GROUP BY
            time_bucket('1 day', time),
            asset_id,
            custom_session_group(time);

SELECT add_continuous_aggregate_policy('session_aggregate_view',
    start_offset => INTERVAL '1 week',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '15 minutes',
    if_not_exists => TRUE);

CREATE OR REPLACE FUNCTION get_trading_day(ts TIMESTAMPTZ)
RETURNS TIMESTAMPTZ LANGUAGE SQL IMMUTABLE AS $$
    SELECT date_trunc('day', ts + INTERVAL '6 hours') AT TIME ZONE 'America/New_York';
$$; 

DROP MATERIALIZED VIEW IF EXISTS US2000_daily_aggregate CASCADE;
CREATE MATERIALIZED VIEW US2000_daily_aggregate
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', time) AS bucket,
    (date_trunc('day', time + INTERVAL '6 hours') AT TIME ZONE 'America/New_York') AS trading_day,
    asset_id,
    first(open, time) AS open,
    max(high) AS high,
    min(low) AS low,
    last(close, time) AS close,
    sum(volume) AS volume
FROM market_data_1hr
WHERE asset_id = 'assets:US2000'
GROUP BY 1, 2, 3;

DROP MATERIALIZED VIEW IF EXISTS US2000_daily_aggregate CASCADE;
CREATE MATERIALIZED VIEW US2000_daily_aggregate
WITH (timescaledb.continuous) AS
SELECT
  time_bucket('1 day', time - INTERVAL '6 hours') + INTERVAL '6 hours' AS bucket,
  ( (time - INTERVAL '6 hours')::date ) AS trading_date,
  asset_id,
  first(open, time) AS open,
  max(high) AS high,
  min(low)  AS low,
  last(close, time) AS close,
  sum(volume) AS volume
FROM market_data
WHERE asset_id = 'assets:US2000'
GROUP BY 1,2,3;

DROP MATERIALIZED VIEW IF EXISTS US2000_daily_aggregate CASCADE;
CREATE MATERIALIZED VIEW US2000_daily_aggregate
WITH (timescaledb.continuous) AS
SELECT
  time_bucket('1 day', time) AS bucket,
  asset_id,
  first(open, time) AS open,
  max(high) AS high,
  min(low)  AS low,
  last(close, time) AS close,
  sum(volume) AS volume
FROM market_data_1hr
WHERE asset_id = 'assets:US2000'
GROUP BY 1,2;
-- when querying convert bucket -> trading_date:
SELECT
  ( (bucket AT TIME ZONE 'UTC') AT TIME ZONE 'America/New_York' + INTERVAL '6 hours')::date AS trading_date,
  *
FROM US2000_daily_aggregate;
---------------------------Second Start--------------------------

-------------candle pattern function------------
CREATE OR REPLACE FUNCTION get_candle_pattern(o double precision, h double precision, l double precision, c double precision)
RETURNS TEXT
AS $$
DECLARE
    body_size DOUBLE PRECISION;
    total_range DOUBLE PRECISION;
    body_ratio DOUBLE PRECISION;
    shadow_ratio DOUBLE PRECISION;
BEGIN
    body_size := ABS(c - o);
    total_range := h - l;

    -- Avoid division by zero
    IF total_range = 0 OR total_range < 0.0001 THEN
        RETURN 'Flat';
    END IF;

    body_ratio := body_size / total_range;

    -- 1. Doji Check
    IF body_ratio < 0.05 THEN
        RETURN 'Doji';
    END IF;

    -- 2. Marubozu Check 
    IF body_ratio > 0.9 THEN
        RETURN CASE WHEN c > o THEN 'Bullish Marubozu' ELSE 'Bearish Marubozu' END;
    END IF;

    -- 3. Hammer/Hanging Man Check 
    IF body_ratio < 0.3 THEN 
        IF c > o THEN -- Bullish Candle
            -- Hammer: Long lower shadow
            shadow_ratio := (o - l) / body_size;
            IF shadow_ratio >= 2.0 THEN RETURN 'Hammer'; END IF;
        ELSE -- Bearish Candle
            -- Hanging Man: Long upper shadow
            shadow_ratio := (h - o) / body_size;
            IF shadow_ratio >= 2.0 THEN RETURN 'Hanging Man'; END IF;
        END IF;
    END IF;
    
    -- 4. Standard Candles
    RETURN CASE WHEN c > o THEN 'Bullish' ELSE 'Bearish' END;
END
$$ LANGUAGE plpgsql IMMUTABLE STRICT;
------------------- candle function end -----------------

-- Create storage table (one-time)
CREATE TABLE IF NOT EXISTS session_daily_store (
  trading_date date NOT NULL,
  asset_id text NOT NULL,
  open double precision,
  high double precision,
  low double precision,
  close double precision,
  volume bigint,
  bars bigint,
  PRIMARY KEY (trading_date, asset_id)
);

-- Refresh (run after hypertable is updated / after view refresh)
BEGIN;
TRUNCATE session_daily_store;

WITH base AS (
  SELECT
    time, asset_id, open, high, low, close, volume,
    ((time AT TIME ZONE 'America/New_York') + INTERVAL '6 hours')::date AS trading_date
  FROM market_data_1hr
  -- optional filter: WHERE asset_id = 'assets:US2000'
),
perday AS (
  SELECT
    trading_date,
    asset_id,
    MIN(time) AS min_ts,
    MAX(time) AS max_ts,
    MAX(high) AS high_price,
    MIN(low)  AS low_price,
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
)
INSERT INTO session_daily_store (trading_date, asset_id, open, high, low, close, volume, bars)
SELECT p.trading_date, p.asset_id, o.open_price, p.high_price, p.low_price, c.close_price, p.volume_sum, p.bars
FROM perday p
LEFT JOIN opens o ON p.trading_date = o.trading_date AND p.asset_id = o.asset_id
LEFT JOIN closes c ON p.trading_date = c.trading_date AND p.asset_id = c.asset_id;
COMMIT;

----- session 

-- ...existing code...
-- Persistent session-level storage (one-time)
CREATE TABLE IF NOT EXISTS session_store (
  trading_date date NOT NULL,
  asset_id text NOT NULL,
  session_name text NOT NULL,
  start_ts timestamptz NOT NULL,
  end_ts timestamptz NOT NULL,
  open double precision,
  high double precision,
  low double precision,
  close double precision,
  volume double precision,
  bars bigint,
  PRIMARY KEY (trading_date, asset_id, session_name)
);

-- Refresh session_store from market_data_1hr (run after data load)
TRUNCATE session_store;

WITH base AS (
  SELECT
    time,
    asset_id,
    open, high, low, close, volume,
    custom_session_group(time) AS session_name,
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
    MIN(low)  AS low_price,
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
INSERT INTO session_store (trading_date, asset_id, session_name, start_ts, end_ts, open, high, low, close, volume, bars)
SELECT
  p.trading_date,
  p.asset_id,
  p.session_name,
  p.start_ts,
  p.end_ts,
  o.open_price,
  p.high_price,
  p.low_price,
  c.close_price,
  p.volume_sum,
  p.bars
FROM per_session p
LEFT JOIN opens  o USING (trading_date, asset_id, session_name)
LEFT JOIN closes c USING (trading_date, asset_id, session_name);
-- ...existing code...

---- weekly & Monthly
DROP TABLE IF EXISTS weekly_store CASCADE;
-- Create storage tables (one-time)
CREATE TABLE IF NOT EXISTS weekly_store (
  week_start date NOT NULL,
  asset_id text NOT NULL,
  start_ts timestamptz NOT NULL,
  end_ts timestamptz NOT NULL,
  weekly_pattern text,
  open double precision,
  high double precision,
  low double precision,
  close double precision,
  volume bigint,
  bars bigint,
  PRIMARY KEY (week_start, asset_id)
);

CREATE TABLE IF NOT EXISTS monthly_store (
  month_start date NOT NULL,
  asset_id text NOT NULL,
  start_ts timestamptz NOT NULL,
  end_ts timestamptz NOT NULL,
  open double precision,
  high double precision,
  low double precision,
  close double precision,
  volume bigint,
  bars bigint,
  PRIMARY KEY (month_start, asset_id)
);

-- Refresh weekly_store (run as needed)
TRUNCATE weekly_store;

WITH base AS (
  SELECT
    time,
    asset_id,
    open, high, low, close, volume,
    -- trading_date is NY trading day label (18:00 -> next day)
    ((time AT TIME ZONE 'America/New_York') + INTERVAL '6 hours')::date AS trading_date
  FROM market_data_1hr
  -- optional filter: WHERE asset_id = 'assets:US2000'
),
perweek AS (
  SELECT
    date_trunc('week', trading_date::timestamp)::date AS week_start,
    asset_id,
    MIN(time) AS min_ts,
    MAX(time) AS max_ts,
    MAX(high) AS high_price,
    MIN(low)  AS low_price,
    SUM(volume) AS volume_sum,
    COUNT(*) AS bars
  FROM base
  GROUP BY 1, 2
),
opens AS (
  SELECT b.week_start, b.asset_id, b.open AS open_price
  FROM base b
  JOIN perweek p ON p.week_start = date_trunc('week', b.trading_date::timestamp)::date
                 AND p.asset_id = b.asset_id
                 AND p.min_ts = b.time
),
closes AS (
  SELECT b.week_start, b.asset_id, b.close AS close_price
  FROM base b
  JOIN perweek p ON p.week_start = date_trunc('week', b.trading_date::timestamp)::date
                 AND p.asset_id = b.asset_id
                 AND p.max_ts = b.time
)
INSERT INTO weekly_store (week_start, asset_id, start_ts, end_ts, open, high, low, close, volume, bars)
SELECT
  p.week_start, p.asset_id, p.min_ts, p.max_ts,
  o.open_price, p.high_price, p.low_price, c.close_price, p.volume_sum, p.bars
FROM perweek p
LEFT JOIN opens  o USING (week_start, asset_id)
LEFT JOIN closes c USING (week_start, asset_id);


-- Refresh monthly_store (run as needed)
TRUNCATE monthly_store;

WITH base AS (
  SELECT
    time,
    asset_id,
    open, high, low, close, volume,
    ((time AT TIME ZONE 'America/New_York') + INTERVAL '6 hours')::date AS trading_date
  FROM market_data_1hr
),
permonth AS (
  SELECT
    date_trunc('month', trading_date::timestamp)::date AS month_start,
    asset_id,
    MIN(time) AS min_ts,
    MAX(time) AS max_ts,
    MAX(high) AS high_price,
    MIN(low)  AS low_price,
    SUM(volume) AS volume_sum,
    COUNT(*) AS bars
  FROM base
  GROUP BY 1, 2
),
opens_m AS (
  SELECT b.month_start, b.asset_id, b.open AS open_price
  FROM base b
  JOIN permonth p ON p.month_start = date_trunc('month', b.trading_date::timestamp)::date
                  AND p.asset_id = b.asset_id
                  AND p.min_ts = b.time
),
closes_m AS (
  SELECT b.month_start, b.asset_id, b.close AS close_price
  FROM base b
  JOIN permonth p ON p.month_start = date_trunc('month', b.trading_date::timestamp)::date
                  AND p.asset_id = b.asset_id
                  AND p.max_ts = b.time
)
INSERT INTO monthly_store (month_start, asset_id, start_ts, end_ts, open, high, low, close, volume, bars)
SELECT
  p.month_start, p.asset_id, p.min_ts, p.max_ts,
  o.open_price, p.high_price, p.low_price, c.close_price, p.volume_sum, p.bars
FROM permonth p
LEFT JOIN opens_m o USING (month_start, asset_id)
LEFT JOIN closes_m c USING (month_start, asset_id);

---------- weekly wth pattern, using daily_store--------------

-- ...existing code...

-- add pattern columns (one-time)
ALTER TABLE IF EXISTS weekly_store  ADD COLUMN IF NOT EXISTS weekly_pattern TEXT;
ALTER TABLE IF EXISTS monthly_store ADD COLUMN IF NOT EXISTS monthly_pattern TEXT;
ALTER TABLE IF EXISTS session_daily_store ADD COLUMN IF NOT EXISTS daily_pattern TEXT;
ALTER TABLE IF EXISTS session_store      ADD COLUMN IF NOT EXISTS session_pattern TEXT;

-- when inserting into weekly_store, compute pattern:
WITH base AS (
  SELECT trading_date, asset_id, open, high, low, close, volume
  FROM session_daily_store   -- or market_data_1hr if you prefer raw
),
perweek AS (
  SELECT
    date_trunc('week', trading_date::timestamp)::date AS week_start,
    asset_id,
    MIN(trading_date) AS min_td,
    MAX(trading_date) AS max_td,
    MAX(high) AS high_price,
    MIN(low)  AS low_price,
    SUM(volume) AS volume_sum,
    COUNT(*) AS bars
  FROM base
  GROUP BY 1,2
),
opens AS (
  SELECT s.trading_date, s.asset_id, s.open
  FROM session_daily_store s
),
closes AS (
  SELECT s.trading_date, s.asset_id, s.close
  FROM session_daily_store s
)
INSERT INTO weekly_store (week_start, asset_id, start_ts, end_ts, open, high, low, close, volume, bars, weekly_pattern)
SELECT
  p.week_start,
  p.asset_id,
  p.min_td::timestamptz,    -- optional mapping for start_ts/end_ts
  p.max_td::timestamptz,
  so.open,
  p.high_price,
  p.low_price,
  sc.close,
  p.volume_sum,
  p.bars,
  get_candle_pattern(so.open, p.high_price, p.low_price, sc.close) AS weekly_pattern
FROM perweek p
LEFT JOIN session_daily_store so ON so.trading_date = p.min_td AND so.asset_id = p.asset_id
LEFT JOIN session_daily_store sc ON sc.trading_date = p.max_td AND sc.asset_id = p.asset_id;
COMMIT;

------- weekly store to hand correct week selection---------
-- Build weekly_store from session_daily_store (fast)
CREATE TABLE IF NOT EXISTS weekly_store (
  week_start date NOT NULL,
  asset_id text NOT NULL,
  start_trading_date date NOT NULL,
  end_trading_date date NOT NULL,
  open double precision,
  high double precision,
  low double precision,
  close double precision,
  volume bigint,
  bars bigint,
  weekly_pattern text,
  PRIMARY KEY (week_start, asset_id)
);

TRUNCATE weekly_store;

WITH base AS (
  SELECT trading_date, asset_id, open, high, low, close, volume, bars
  FROM session_daily_store
),
perweek AS (
  SELECT
    date_trunc('week', trading_date::timestamp)::date    AS week_start,
    asset_id,
    MIN(trading_date) AS start_trading_date,
    MAX(trading_date) AS end_trading_date,
    MAX(high)        AS high_price,
    MIN(low)         AS low_price,
    SUM(volume)      AS volume_sum,
    SUM(bars)        AS bars
  FROM base
  GROUP BY 1,2
)
INSERT INTO weekly_store (
  week_start, asset_id, start_trading_date, end_trading_date,
  open, high, low, close, volume, bars, weekly_pattern
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
  get_candle_pattern(s_open.open, p.high_price, p.low_price, s_close.close)
FROM perweek p
LEFT JOIN session_daily_store s_open
  ON s_open.trading_date = p.start_trading_date AND s_open.asset_id = p.asset_id
LEFT JOIN session_daily_store s_close
  ON s_close.trading_date = p.end_trading_date   AND s_close.asset_id = p.asset_id
ORDER BY p.week_start;