------------------
-- Structual Battle Index (SBI)
------------------

DROP TABLE IF EXISTS asset_structural_battle_index CASCADE;
CREATE TABLE IF NOT EXISTS asset_structural_battle_index (
    asset_id TEXT NOT NULL,
    -- Context of the session immediately preceding the structural break
    ps1_name TEXT NOT NULL,
    ps1_bias_7 TEXT NOT NULL,
    -- Context of the session that achieves the structural break
    cs_name TEXT NOT NULL, 
    prior_level TEXT NOT NULL,                     -- 'High' (PDH) or 'Low' (PDL)
    
    total_breaks BIGINT NOT NULL,                  -- Denominator: Total breaks for this PS->CS pattern
    
    avg_battle_duration_s DOUBLE PRECISION NOT NULL, -- Average time (in seconds) spent battling near the level
    avg_battle_range_pct DOUBLE PRECISION NOT NULL,  -- Average price fluctuation range (as a % of Daily ATR) near the level
    
    battle_label TEXT NOT NULL,                    -- 'Decisive' (low S.B.I.) or 'Indecisive' (high S.B.I.)
    
    PRIMARY KEY (asset_id, ps1_name, ps1_bias_7, cs_name, prior_level)
);
TRUNCATE asset_structural_battle_index;

-- =================================================================================
-- M12: Structural Battle Index (SBI)
-- Measures the average duration and range of price action before a structural break.
-- DEPENDENCIES: asset_session_views, asset_structural_takedowns (must contain break_ts), M13 (asset_daily_atr_14d)
-- =================================================================================

-- 0. DEFINE CONSTANTS
WITH constants AS (
    SELECT 900.0 AS neutral_sbi_duration_s, -- 15 minutes
           10.0 AS neutral_sbi_range_pct    -- 10% of Daily ATR
),
-- 1. CALCULATE DAILY ATR (M13 Dependency)
daily_prices AS (
    -- Get H, L, C for the day and the Previous Day's Close (PDC)
    SELECT
        sv.asset_id,
        sv.trading_date,
        MAX(sv.high) AS daily_high,
        MIN(sv.low) AS daily_low,
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
    WHERE d.pdc IS NOT NULL
),
asset_daily_atr_14d AS (
    -- Calculate 14-day SMA of TR (Approximation for ATR EMA)
    SELECT
        t.asset_id,
        t.trading_date,
        AVG(t.true_range) OVER (
            PARTITION BY t.asset_id 
            ORDER BY t.trading_date 
            ROWS BETWEEN 13 PRECEDING AND CURRENT ROW
        ) AS daily_atr
    FROM true_range_calc t
),
-- 2. IDENTIFY ALL STRUCTURAL TAKEDOWNS AND GATHER DATA
required_data AS (
    SELECT
        t.asset_id,
        t.trading_date,
        uc.ps1_name,
        uc.ps1_bias_7_state AS ps1_bias_7,
        uc.cs_name,
        t.prior_level,
        -- ASSUMED: The explicit time the level was broken (T_end)
        t.break_ts, 
        sv.start_ts AS cs_start_ts, -- T_start (Approximation: CS session start time)
        sv.high AS cs_session_high,
        sv.low AS cs_session_low,
        atr.daily_atr -- M13 result for normalization
    FROM asset_structural_takedowns t
    JOIN asset_session_context uc 
        ON t.asset_id = uc.asset_id AND t.trading_date = uc.trading_date AND t.session_name = uc.cs_name
    JOIN asset_session_views sv 
        ON uc.asset_id = sv.asset_id AND uc.trading_date = sv.trading_date AND uc.cs_name = sv.session_name
    JOIN asset_daily_atr_14d atr 
        ON uc.asset_id = atr.asset_id AND uc.trading_date = atr.trading_date
    WHERE t.break_ts IS NOT NULL AND atr.daily_atr > 0.0 -- Ensure valid data
),
-- 3. CALCULATE BATTLE METRICS PER EVENT
battle_calculation AS (
    SELECT
        rd.asset_id,
        rd.ps1_name,
        rd.ps1_bias_7,
        rd.cs_name,
        rd.prior_level,
        -- Duration (seconds): T_end - T_start
        EXTRACT(EPOCH FROM (rd.break_ts - rd.cs_start_ts)) AS battle_duration_s,
        -- Price Range: Max High - Min Low of the CS session up to the break
        (rd.cs_session_high - rd.cs_session_low) AS battle_price_range,
        rd.daily_atr
    FROM required_data rd
    WHERE rd.break_ts > rd.cs_start_ts -- Exclude invalid durations
),
-- 4. AGGREGATE THE SBI BY PATTERN
aggregated_sbi AS (
    SELECT
        c.asset_id,
        c.ps1_name,
        c.ps1_bias_7,
        c.cs_name,
        c.prior_level,
        COUNT(*) AS total_breaks,
        AVG(c.battle_duration_s) AS avg_battle_duration_s,
        -- Normalize the price range by dividing by the Daily ATR
        AVG(c.battle_price_range / c.daily_atr) * 100 AS avg_battle_range_pct
        
    FROM battle_calculation c
    GROUP BY 1, 2, 3, 4, 5
)
-- 5. FINAL INSERTION INTO M12 TABLE (asset_structural_battle_index)
INSERT INTO asset_structural_battle_index (
    asset_id, ps1_name, ps1_bias_7, cs_name, prior_level, total_breaks, 
    avg_battle_duration_s, avg_battle_range_pct, battle_label
)
SELECT
    a.asset_id,
    a.ps1_name,
    a.ps1_bias_7,
    a.cs_name,
    a.prior_level,
    a.total_breaks,
    a.avg_battle_duration_s,
    a.avg_battle_range_pct,
    -- Assign label based on constants
    CASE
        WHEN a.avg_battle_duration_s < c.neutral_sbi_duration_s 
             AND a.avg_battle_range_pct < c.neutral_sbi_range_pct 
            THEN 'Decisive'
        ELSE 'Indecisive'
    END AS battle_label
FROM aggregated_sbi a
CROSS JOIN constants c
WHERE a.total_breaks > 0;