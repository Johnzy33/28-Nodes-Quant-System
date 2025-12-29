----------------------
-- Session View, Main table
----------------------------

DROP TABLE IF EXISTS session_views CASCADE;
CREATE TABLE IF NOT EXISTS session_views (
  trading_date date NOT NULL,
  asset_id text NOT NULL,
  session_name text NOT NULL,
  session_type text NOT NULL,              -- Primary classification
  --consolidation_subtype text,             -- Secondary classification
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

-- Ensure the target table is a Hypertable (recommended for continuous aggregation)
SELECT create_hypertable('session_views', 'trading_date', if_not_exists => TRUE);

-- Execute the stored procedure to run the final calculation and INSERT data
CALL refresh_session_views();
-- Create the Stored Procedure
CREATE OR REPLACE PROCEDURE refresh_session_views()
LANGUAGE sql
AS $$

TRUNCATE session_views;
WITH classified AS (
    -- Read from Layer 1 MV and apply the classification function
    SELECT
        sbm.trading_date,
        sbm.asset_id,
        sbm.session_name,
        get_market_type(sbm.open, sbm.high, sbm.low, sbm.close) AS classification,
        sbm.start_ts, sbm.end_ts, sbm.open, sbm.high, sbm.high_ts, 
        sbm.low, sbm.low_ts, sbm.close, sbm.volume, sbm.bars
    FROM session_base sbm
)
INSERT INTO session_views (
    trading_date, asset_id, session_name, session_type, consolidation_subtype,
    start_ts, end_ts, open, high, high_ts, low, low_ts, close, volume, bars
)
SELECT
    cld.trading_date,
    cld.asset_id,
    cld.session_name,
    (cld.classification).session_type,
    (cld.classification).consolidation_subtype,
    cld.start_ts, cld.end_ts, cld.open, cld.high, cld.high_ts, 
    cld.low, cld.low_ts, cld.close, cld.volume, cld.bars
FROM classified cld
-- Handle updates for existing sessions and insert for new ones
ON CONFLICT (trading_date, asset_id, session_name) DO UPDATE SET
    start_ts = EXCLUDED.start_ts,
    end_ts = EXCLUDED.end_ts,
    open = EXCLUDED.open,
    high = EXCLUDED.high,
    low = EXCLUDED.low,
    close = EXCLUDED.close,
    volume = EXCLUDED.volume,
    bars = EXCLUDED.bars,
    session_type = EXCLUDED.session_type,
    consolidation_subtype = EXCLUDED.consolidation_subtype,
    high_ts = EXCLUDED.high_ts,
    low_ts = EXCLUDED.low_ts;
$$;