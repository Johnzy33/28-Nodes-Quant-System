-- -----------------------------------------------------------
-- 3. SESSION VIEW TABLE (Schema Updated for 2-Column Classification)
-- -----------------------------------------------------------
DROP TABLE IF EXISTS us2000_session_views CASCADE;
CREATE TABLE IF NOT EXISTS us2000_session_views (
  trading_date date NOT NULL,
  asset_id text NOT NULL,
  session_name text NOT NULL,
  session_type text NOT NULL,              -- Primary classification
  consolidation_subtype text,             -- Secondary classification
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

-- -----------------------------------------------------------
-- 4. DAILY VIEW TABLE (Schema Updated for 2-Column Classification)
-- -----------------------------------------------------------
DROP TABLE IF EXISTS us2000_daily_views CASCADE;
CREATE TABLE IF NOT EXISTS us2000_daily_views (
  trading_date date NOT NULL,
  asset_id text NOT NULL,
  daily_type text,                        -- Primary classification (formerly 'pattern')
  consolidation_subtype text,             -- Secondary classification
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

-- -----------------------------------------------------------
-- 5. WEEKLY VIEW TABLE (Schema Updated for 2-Column Classification)
-- -----------------------------------------------------------
DROP TABLE IF EXISTS us2000_weekly_views CASCADE;
CREATE TABLE IF NOT EXISTS us2000_weekly_views (
  week_start date NOT NULL,
  asset_id text NOT NULL,
  start_trading_date date NOT NULL,
  end_trading_date date NOT NULL,
  weekly_type text,                       -- Primary classification (formerly 'weekly_pattern')
  consolidation_subtype text,             -- Secondary classification
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

-- -----------------------------------------------------------
-- 6. MONTHLY VIEW TABLE (Schema Updated for 2-Column Classification)
-- -----------------------------------------------------------
DROP TABLE IF EXISTS us2000_monthly_views CASCADE;
CREATE TABLE IF NOT EXISTS us2000_monthly_views (
  month_start date NOT NULL,
  asset_id text NOT NULL,
  start_trading_date date NOT NULL,
  end_trading_date date NOT NULL,
  monthly_type text,                      -- Primary classification (formerly 'monthly_pattern')
  consolidation_subtype text,             -- Secondary classification
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

-- -----------------------------------------------------------
-- 7. TAKEN EVENT TABLE
-- -----------------------------------------------------------
-- Table to store all sequential level breaks (The "Event Log")
DROP TABLE IF EXISTS us2000_takedown_events CASCADE;
CREATE TABLE IF NOT EXISTS us2000_takedown_events (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,  
    -- WHO DID IT: The session that exceeded/undercut the level
    breaker_session TEXT NOT NULL,  
    -- WHOSE LEVEL WAS BROKEN: The session that defined the level
    prior_session TEXT NOT NULL,  
    -- WHAT LEVEL: High or Low
    prior_level TEXT NOT NULL,  
    -- Price information for auditing/verification
    prior_level_price DOUBLE PRECISION,
    breaker_price DOUBLE PRECISION,  
    -- Structural significance flag
    cumulative_level BOOLEAN DEFAULT FALSE,   
    PRIMARY KEY (trading_date, asset_id, breaker_session, prior_session, prior_level)
);

DROP TABLE IF EXISTS us2000_unified_bias_source CASCADE;
-- Metric X: Unified Bias Source Table
CREATE TABLE IF NOT EXISTS us2000_unified_bias_source (
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    session_name TEXT NOT NULL,
    start_ts TIMESTAMP WITH TIME ZONE NOT NULL,
    end_ts TIMESTAMP WITH TIME ZONE NOT NULL,
    bias_7_state TEXT NOT NULL,
    PRIMARY KEY (asset_id, session_name, start_ts)
);
TRUNCATE us2000_unified_bias_source;

-- CTE to calculate the 7-State bias for every session
WITH session_bias_calculation AS (
    SELECT
        asset_id,
        trading_date,
        session_name,
        start_ts,
        end_ts,
        -- Defines the 7-State bias for the session
        CASE
            WHEN session_type IN ('Bullish', 'Bearish') THEN session_type
            WHEN session_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other'
        END AS bias_7_state
    FROM us2000_session_views
    WHERE asset_id = 'assets:US2000'
)
-- INSERT INTO Unified Bias Source Table
INSERT INTO us2000_unified_bias_source (
    asset_id, trading_date, session_name, start_ts, end_ts, bias_7_state
)
SELECT
    asset_id,
    trading_date,
    session_name,
    start_ts,
    end_ts,
    bias_7_state
FROM session_bias_calculation
ORDER BY start_ts;