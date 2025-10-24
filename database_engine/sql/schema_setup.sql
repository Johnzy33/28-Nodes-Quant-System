-- schema_setup.sql

-- Set Timezone to UTC for session
SET timezone = 'UTC';


-- -----------------------------------------------------------
-- 1. ASSET TABLE (Reference Table)
-- -----------------------------------------------------------
-- Stores metadata about each tradable asset.
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
-- 2. CORE HYPERTABLE (The raw 1-hour data)
-- -----------------------------------------------------------
-- This table stores all normalized 1-hour candle data.

CREATE TABLE IF NOT EXISTS market_data_1hr (
    time TIMESTAMPTZ NOT NULL,
    asset_id TEXT NOT NULL REFERENCES assets(id),
    open DOUBLE PRECISION NOT NULL,
    high DOUBLE PRECISION NOT NULL,
    low DOUBLE PRECISION NOT NULL,
    close DOUBLE PRECISION NOT NULL,
    volume DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (time, asset_id)
);

SELECT create_hypertable('market_data_1hr', 'time', if_not_exists => TRUE);
-- Index on asset_id for faster queries filtering by asset
CREATE INDEX IF NOT EXISTS idx_asset_id ON market_data_1hr (asset_id);


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