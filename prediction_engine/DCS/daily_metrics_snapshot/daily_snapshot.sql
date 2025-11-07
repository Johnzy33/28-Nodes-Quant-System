-------------------------------------------
-- Sanpshot file for the DCS scoring 
-----------------------------------------

DROP TABLE daily_metrics_snapshot;

-- 3. Daily Metrics Snapshot (Inputs for DCS - Hypertable)
DROP TABLE IF EXISTS daily_metrics_snapshot CASCADE;
CREATE TABLE IF NOT EXISTS daily_metrics_snapshot (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    Prior_Weekly_Type TEXT,
    Prior_Day_Type TEXT,
    DOW_Transition TEXT,
    DBS_ST_Score NUMERIC(10, 4),
    DBS_LT_Score NUMERIC(10, 4),
    DBS_YTD_Score NUMERIC(10, 4),
    D_M5_Bullish_Reversal_Risk NUMERIC(10, 4),
    D_M5_Bearish_Reversal_Risk NUMERIC(10, 4),
    D_M3_Continuation_Prob NUMERIC(10, 4),
    D_M4_Bullish_FT_Prob NUMERIC(10, 4),
    D_M4_Bearish_FT_Prob NUMERIC(10, 4),
    Calculation_Date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY(trading_date, asset_id)
);
SELECT create_hypertable('daily_metrics_snapshot', 'trading_date', if_not_exists => TRUE);
CALL refresh_daily_metrics_snapshot();

CREATE OR REPLACE PROCEDURE refresh_daily_metrics_snapshot()
LANGUAGE sql
AS $$
INSERT INTO daily_metrics_snapshot (
    trading_date, asset_id, Prior_Weekly_Type, Prior_Day_Type, DOW_Transition,
    DBS_ST_Score, DBS_LT_Score, DBS_YTD_Score, 
    D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk, 
    D_M3_Continuation_Prob, D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob
)
WITH Daily_Derived_Context AS (
    -- 1. DERIVE ALL MISSING CONTEXT FIELDS (DOW, Prior Day Type, DOW Transition)
    SELECT
        trading_date, asset_id, day_type, "open", "high", "low", "close",
        TRIM(TO_CHAR(trading_date, 'Day')) AS dow, 
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(TRIM(TO_CHAR(trading_date, 'Day')), 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || TRIM(TO_CHAR(trading_date, 'Day')) AS DOW_Transition
    FROM daily_views
),
Weekly_Context AS (
    -- 2. Get Prior Week Type
    SELECT
        week_start, asset_id,
        LAG(Weekly_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS Prior_Weekly_Type 
    FROM weekly_views
),
D_M3_M4_Context AS (
    -- 3. Calculate D.M3 and D.M4-related fields using derived context
    SELECT 
        D.trading_date, D.asset_id, D.Prior_Day_Type, D.DOW_Transition,
        D.dow, D."open", D."high", D."low", D."close",
        -- D.M3 Continuation logic
        CASE WHEN D.Prior_Day_Type = 'Bullish' AND D.day_type = 'Bullish' THEN 1
             WHEN D.Prior_Day_Type = 'Bearish' AND D.day_type = 'Bearish' THEN 1
             ELSE 0 END AS Continuation,
        -- D.M4 Prior Break Type: Needs high/low from T-1 and T-2
        CASE WHEN LAG(D.high, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) > LAG(D.high, 2) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) THEN 'Bullish_Break'
             WHEN LAG(D.low, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) < LAG(D.low, 2) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) THEN 'Bearish_Break'
             ELSE 'No_Break' END AS Prior_Break_Type,
        -- Day Close Type
        CASE WHEN D."close" > D."open" THEN 'Bullish_Close' ELSE 'Bearish_Close' END AS Day_Close_Type
    FROM Daily_Derived_Context D
),
D_M5_Base_Flags AS (
    -- 4. NEW CTE: Calculate PDH/PDL and M5 flags PER ROW before aggregation
    SELECT
        DDC.trading_date, DDC.asset_id, DDC.dow AS CD_DOW, DDC."high", DDC."low", DDC."close",
        LAG(DDC.high, 1) OVER (PARTITION BY DDC.asset_id ORDER BY DDC.trading_date) AS PDH,
        LAG(DDC.low, 1) OVER (PARTITION BY DDC.asset_id ORDER BY DDC.trading_date) AS PDL,
        -- M5 Bullish Reversal Flag
        CASE WHEN DDC.high > LAG(DDC.high, 1) OVER (PARTITION BY DDC.asset_id ORDER BY DDC.trading_date) 
                 AND (DDC."high" - DDC."close") / NULLIF((DDC."high" - DDC."low"), 0) >= 0.70 THEN 1 ELSE 0 END AS Bullish_Rev_Flag,
        -- M5 Bearish Reversal Flag
        CASE WHEN DDC.low < LAG(DDC.low, 1) OVER (PARTITION BY DDC.asset_id ORDER BY DDC.trading_date) 
                 AND (DDC."close" - DDC."low") / NULLIF((DDC."high" - DDC."low"), 0) <= 0.30 THEN 1 ELSE 0 END AS Bearish_Rev_Flag,
        -- Total Bullish Break days
        CASE WHEN DDC.high > LAG(DDC.high, 1) OVER (PARTITION BY DDC.asset_id ORDER BY DDC.trading_date) THEN 1 ELSE 0 END AS Total_Bullish_Break,
        -- Total Bearish Break days
        CASE WHEN DDC.low < LAG(DDC.low, 1) OVER (PARTITION BY DDC.asset_id ORDER BY DDC.trading_date) THEN 1 ELSE 0 END AS Total_Bearish_Break
    FROM Daily_Derived_Context DDC
),
M3_M4_Aggregated AS (
    -- 5. Aggregate D.M3 and D.M4 Probabilities
    SELECT
        asset_id, Prior_Day_Type, DOW_Transition,
        SUM(Continuation)::NUMERIC / NULLIF(COUNT(*), 0) AS D_M3_Continuation_Prob,
        SUM(CASE WHEN Prior_Break_Type = 'Bullish_Break' AND Day_Close_Type = 'Bullish_Close' THEN 1 ELSE 0 END)::NUMERIC /
            NULLIF(SUM(CASE WHEN Prior_Break_Type = 'Bullish_Break' THEN 1 ELSE 0 END), 0) AS D_M4_Bullish_FT_Prob,
        SUM(CASE WHEN Prior_Break_Type = 'Bearish_Break' AND Day_Close_Type = 'Bearish_Close' THEN 1 ELSE 0 END)::NUMERIC /
            NULLIF(SUM(CASE WHEN Prior_Break_Type = 'Bearish_Break' THEN 1 ELSE 0 END), 0) AS D_M4_Bearish_FT_Prob
    FROM D_M3_M4_Context
    WHERE Prior_Day_Type IS NOT NULL
    GROUP BY 1, 2, 3
),
D_M5_Probabilities AS (
    -- 6. D.M5: Daily DOW Reversal Risk (Now aggregating the pre-calculated flags)
    SELECT
        asset_id,
        CD_DOW,
        SUM(Bullish_Rev_Flag)::NUMERIC / NULLIF(SUM(Total_Bullish_Break), 0) AS D_M5_Bullish_Reversal_Risk,
        SUM(Bearish_Rev_Flag)::NUMERIC / NULLIF(SUM(Total_Bearish_Break), 0) AS D_M5_Bearish_Reversal_Risk
    FROM D_M5_Base_Flags
    WHERE PDH IS NOT NULL -- Ensure we have a prior day to compare
    GROUP BY 1, 2
),
DBS_Score_Calculation AS (
    -- 7. Calculate the three final DBS scores (D.M2) and merge all M_n results
    SELECT
        DDC.trading_date, DDC.asset_id, W.Prior_Weekly_Type, DDC.Prior_Day_Type, DDC.DOW_Transition,
        -- FAST DBS SCORING: Join to the pre-calculated history table (H)
        ROUND(((H.pwh_break_st_count::NUMERIC / NULLIF(H.total_st_days, 0.0)) - (H.pwl_break_st_count::NUMERIC / NULLIF(H.total_st_days, 0.0)))::NUMERIC, 4) AS DBS_ST_Score,
        ROUND(((H.pwh_break_lt_count::NUMERIC / NULLIF(H.total_lt_days, 0.0)) - (H.pwl_break_lt_count::NUMERIC / NULLIF(H.total_lt_days, 0.0)))::NUMERIC, 4) AS DBS_LT_Score,
        ROUND(((H.pwh_break_ytd_count::NUMERIC / NULLIF(H.total_ytd_days, 0.0)) - (H.pwl_break_ytd_count::NUMERIC / NULLIF(H.total_ytd_days, 0.0)))::NUMERIC, 4) AS DBS_YTD_Score,
        -- Merge M3, M4, M5 probabilities
        COALESCE(M34.D_M3_Continuation_Prob, 0.5) AS D_M3_Continuation_Prob,
        COALESCE(M34.D_M4_Bullish_FT_Prob, 0.5) AS D_M4_Bullish_FT_Prob,
        COALESCE(M34.D_M4_Bearish_FT_Prob, 0.5) AS D_M4_Bearish_FT_Prob,
        COALESCE(M5.D_M5_Bullish_Reversal_Risk, 0.5) AS D_M5_Bullish_Reversal_Risk, 
        COALESCE(M5.D_M5_Bearish_Reversal_Risk, 0.5) AS D_M5_Bearish_Reversal_Risk
    FROM Daily_Derived_Context DDC
    INNER JOIN Weekly_Context W ON W.asset_id = DDC.asset_id AND DATE_TRUNC('week', DDC.trading_date) = W.week_start
    INNER JOIN dbs_history_agg H 
        ON DDC.asset_id = H.asset_id AND DDC.Prior_Day_Type = H.prior_day_type AND DDC.DOW_Transition = H.dow_transition
    LEFT JOIN M3_M4_Aggregated M34 
        ON DDC.asset_id = M34.asset_id AND DDC.Prior_Day_Type = M34.Prior_Day_Type AND DDC.DOW_Transition = M34.DOW_Transition
    LEFT JOIN D_M5_Probabilities M5 
        ON DDC.asset_id = M5.asset_id AND DDC.dow = M5.CD_DOW
    WHERE DDC.Prior_Day_Type IS NOT NULL AND W.Prior_Weekly_Type IS NOT NULL
)
-- 8. Final Insertion into the Metrics Snapshot Table
SELECT
    trading_date, asset_id, Prior_Weekly_Type, Prior_Day_Type, DOW_Transition,
    DBS_ST_Score, DBS_LT_Score, DBS_YTD_Score, 
    D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk, 
    D_M3_Continuation_Prob, D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob
FROM DBS_Score_Calculation
ON CONFLICT (trading_date, asset_id) 
DO UPDATE SET
    Prior_Weekly_Type = EXCLUDED.Prior_Weekly_Type, Prior_Day_Type = EXCLUDED.Prior_Day_Type, DOW_Transition = EXCLUDED.DOW_Transition,
    DBS_ST_Score = EXCLUDED.DBS_ST_Score, DBS_LT_Score = EXCLUDED.DBS_LT_Score, DBS_YTD_Score = EXCLUDED.DBS_YTD_Score,
    D_M5_Bullish_Reversal_Risk = EXCLUDED.D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk = EXCLUDED.D_M5_Bearish_Reversal_Risk,
    D_M3_Continuation_Prob = EXCLUDED.D_M3_Continuation_Prob, D_M4_Bullish_FT_Prob = EXCLUDED.D_M4_Bullish_FT_Prob,
    D_M4_Bearish_FT_Prob = EXCLUDED.D_M4_Bearish_FT_Prob,
    Calculation_Date = NOW();
$$;

-- Replace the latest date with whatever your MAX(trading_date) is.
WITH Daily_Derived_Context AS (
    SELECT
        trading_date, asset_id, day_type, "open", "high", "low", "close",
        TRIM(TRAILING ' ' FROM TO_CHAR(trading_date, 'Day')) AS dow,
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(TRIM(TRAILING ' ' FROM TO_CHAR(trading_date, 'Day')), 1) OVER (PARTITION BY asset_id ORDER BY trading_date) 
            || ' -> ' || TRIM(TRAILING ' ' FROM TO_CHAR(trading_date, 'Day')) AS DOW_Transition
    FROM daily_views
),
Weekly_Context AS (
    SELECT week_start, asset_id,
        LAG(Weekly_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS Prior_Weekly_Type 
    FROM weekly_views
)
SELECT
    DDC.trading_date, DDC.asset_id, W.Prior_Weekly_Type, DDC.Prior_Day_Type, DDC.DOW_Transition,
    H.asset_id AS History_Join_Check
FROM Daily_Derived_Context DDC
INNER JOIN Weekly_Context W 
    ON W.asset_id = DDC.asset_id AND DATE_TRUNC('week', DDC.trading_date) = W.week_start
-- Check if this crucial join succeeds!
INNER JOIN dbs_history_agg H 
    ON DDC.asset_id = H.asset_id 
    AND DDC.Prior_Day_Type = H.prior_day_type 
    AND DDC.DOW_Transition = H.dow_transition
WHERE DDC.trading_date = '2025-11-05' 
    AND DDC.Prior_Day_Type IS NOT NULL 
    AND W.Prior_Weekly_Type IS NOT NULL;


WITH Day_DOW_Context AS (
    SELECT
        trading_date, asset_id, day_type, "high", "low",
        TRIM(TRAILING ' ' FROM TO_CHAR(trading_date, 'Day')) AS dow
    FROM daily_views
),
latest_day_context AS (
    SELECT
        D.trading_date, D.asset_id, D.dow,
        LAG(D.day_type, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) AS Prior_Day_Type,
        LAG(D.dow, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) || ' -> ' || D.dow AS DOW_Transition,
        LAG(W.high, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) AS PWH, -- Prior Week High
        LAG(W.low, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) AS PWL,   -- Prior Week Low
        D."high", D."low"
    FROM Day_DOW_Context D
    INNER JOIN weekly_views W ON W.asset_id = D.asset_id AND DATE_TRUNC('week', D.trading_date) = W.week_start
    WHERE D.trading_date = (SELECT MAX(trading_date) FROM daily_views) -- Max trading date is 2025-11-04
)
SELECT * FROM latest_day_context;

-- Use Day_DOW_Context and latest_day_context from above
-- ... followed by latest_break_data and incremental_update:

WITH

incremental_update AS (
    SELECT
        asset_id, Prior_Day_Type, DOW_Transition,
        SUM(CASE WHEN Is_ST THEN 1 ELSE 0 END) AS Total_ST_Days, -- Should be 1
        SUM(CASE WHEN Is_ST AND PWH_Break = 1 THEN 1 ELSE 0 END) AS PWH_Break_ST_Count, -- Should be 0 or 1
        ... (all other counts)
    FROM latest_break_data
    GROUP BY 1, 2, 3
)
SELECT * FROM incremental_update;