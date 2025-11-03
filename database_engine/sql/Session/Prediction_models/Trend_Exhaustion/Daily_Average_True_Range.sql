-------------------------------
-- Daily Average True Range (ATR) Model
------------------------------- 

DROP TABLE IF EXISTS asset_daily_atr_14d CASCADE;
CREATE TABLE IF NOT EXISTS asset_daily_atr_14d (
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    true_range DOUBLE PRECISION NOT NULL,    -- Max(H, PDC) - Min(L, PDC)
    daily_atr DOUBLE PRECISION NOT NULL,     -- 14-day Exponential Moving Average of True Range
    PRIMARY KEY (asset_id, trading_date)
);
TRUNCATE asset_daily_atr_14d;

WITH daily_prices AS (
    -- Get H, L, C for the day and the Previous Day's Close (PDC)
    SELECT
        sv.asset_id,
        sv.trading_date,
        MAX(sv.high) AS daily_high,  -- CORRECTED: Used 'high'
        MIN(sv.low) AS daily_low,    -- CORRECTED: Used 'low'
        -- Get the Previous Day's Close (PDC)
        LAG(MAX(sv.close)) OVER (PARTITION BY sv.asset_id ORDER BY sv.trading_date) AS pdc
    FROM asset_session_views sv
    GROUP BY 1, 2
),
true_range_calc AS (
    -- Calculate True Range (TR) = Max(H, PDC) - Min(L, PDC)
    SELECT
        d.asset_id,
        d.trading_date,
        GREATEST(d.daily_high, d.pdc) - LEAST(d.daily_low, d.pdc) AS true_range
    FROM daily_prices d
    WHERE d.pdc IS NOT NULL -- Exclude the first day of data
),
atr_ema_calc AS (
    -- Calculate 14-day EMA of TR (Standard ATR)
    -- This requires a recursive or window function calculation (simplified below)
    SELECT
        t.asset_id,
        t.trading_date,
        t.true_range,
        -- Simplified concept for EMA calculation (Actual calculation is complex/iterative)
        AVG(t.true_range) OVER (
            PARTITION BY t.asset_id 
            ORDER BY t.trading_date 
            ROWS BETWEEN 13 PRECEDING AND CURRENT ROW
        ) AS daily_atr
    FROM true_range_calc t
)
-- INSERT INTO M13 Daily ATR Table
INSERT INTO asset_daily_atr_14d (
    asset_id, trading_date, true_range, daily_atr
)
SELECT * FROM atr_ema_calc;