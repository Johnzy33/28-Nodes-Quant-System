-------------------
--- snaphot
------------------

-- Target table for the Heavy Lifting Pipeline output
DROP TABLE asset_daily_metrics_snapshot;

CREATE TABLE IF NOT EXISTS asset_daily_metrics_snapshot (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    DOW_Transition TEXT,
    Prior_Weekly_Type TEXT,
    -- Final Calculated DBS Scores (D.M2 component)
    DBS_ST_Score NUMERIC(10, 4),
    DBS_LT_Score NUMERIC(10, 4),
    DBS_YTD_Score NUMERIC(10, 4),
    -- Raw M_n Probabilities (from asset_daily_conditional_metrics lookup)
    D_M5_Bullish_Reversal_Risk NUMERIC(10, 4),
    D_M5_Bearish_Reversal_Risk NUMERIC(10, 4),
    D_M3_Continuation_Prob NUMERIC(10, 4),
    D_M4_Bullish_FT_Prob NUMERIC(10, 4),
    D_M4_Bearish_FT_Prob NUMERIC(10, 4),
    Calculation_Date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    PRIMARY KEY(trading_date, asset_id)
);

-- TRUNCATE asset_daily_metrics_snapshot; -- Optional: Clear the table before a full recalculation

WITH Weekly_Ranges AS (
    -- 1. Calculate the Prior Week High (PWH), Low (PWL), and Prior Week Type (PWT)
    SELECT
        week_start, asset_id,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWH,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWL,
        LAG(Weekly_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS Prior_Weekly_Type 
    FROM asset_weekly_views
),
Daily_Context_With_Prior AS (
    -- 2. Calculate the Prior Day context for joining metrics
    SELECT
        trading_date, asset_id, "high", "low",
        -- Note: Prior_Day_Type is assumed to be calculated and stored in asset_daily_views or derived here
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS PDH,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS PDL,
        LAG(dow, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || dow AS DOW_Transition
    FROM asset_daily_views
),
Daily_Context_With_DBS_Inputs AS (
    -- 3. Join Daily Context, Weekly Ranges, and all M_n Risk Metrics
    SELECT
        D.trading_date, D.asset_id, D."high", D."low", D.Prior_Day_Type, D.DOW_Transition,
        W.Prior_Weekly_Type, D.PDH, D.PDL, W.PWH, W.PWL, 
        -- Fetch ALL base M_n probabilities needed for current calculation AND logging
        COALESCE(CM.D_M5_Bullish_Reversal_Risk, 0.0) AS D_M5_Bullish_Reversal_Risk,
        COALESCE(CM.D_M5_Bearish_Reversal_Risk, 0.0) AS D_M5_Bearish_Reversal_Risk,
        COALESCE(CM.D_M3_Continuation_Prob, 0.0) AS D_M3_Continuation_Prob,
        COALESCE(CM.D_M4_Bullish_FT_Prob, 0.0) AS D_M4_Bullish_FT_Prob,
        COALESCE(CM.D_M4_Bearish_FT_Prob, 0.0) AS D_M4_Bearish_FT_Prob,
        -- Break Flags (Used for calculating the DBS scores)
        CASE WHEN D."high" > D.PDH THEN 1 ELSE 0 END AS PDH_Break,
        CASE WHEN D."low" < D.PDL THEN 1 ELSE 0 END AS PDL_Break,
        CASE WHEN D."high" > W.PWH THEN 1 ELSE 0 END AS PWH_Break,
        CASE WHEN D."low" < W.PWL THEN 1 ELSE 0 END AS PWL_Break,
        -- Time Horizon Markers (Used for conditional counting)
        CASE WHEN D.trading_date >= (SELECT MAX(trading_date) FROM asset_daily_views) - INTERVAL '6 months' THEN TRUE ELSE FALSE END AS Is_ST,
        CASE WHEN D.trading_date >= (SELECT MAX(trading_date) FROM asset_daily_views) - INTERVAL '3 years' THEN TRUE ELSE FALSE END AS Is_LT,
        CASE WHEN EXTRACT(YEAR FROM D.trading_date) = EXTRACT(YEAR FROM (SELECT MAX(trading_date) FROM asset_daily_views)) THEN TRUE ELSE FALSE END AS Is_YTD
    FROM Daily_Context_With_Prior D
    INNER JOIN Weekly_Ranges W ON W.asset_id = D.asset_id AND DATE_TRUNC('week', D.trading_date) = W.week_start
    LEFT JOIN asset_daily_conditional_metrics CM 
        ON CM.asset_id = D.asset_id 
        AND CM.PD_Type = D.Prior_Day_Type 
        AND CM.DOW_Transition = D.DOW_Transition
    WHERE W.PWH IS NOT NULL AND D.Prior_Day_Type IS NOT NULL
),
Conditional_DBS_Aggregates AS (
    -- 4. Calculate conditional counts and total days for all timeframes
    SELECT
        trading_date, asset_id, Prior_Weekly_Type,
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk, 
        D_M3_Continuation_Prob, D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob,
        -- Short-Term (ST) Aggregates
        SUM(CASE WHEN Is_ST THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_ST_Days,
        SUM(CASE WHEN Is_ST AND PWH_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_ST_Count,
        SUM(CASE WHEN Is_ST AND PWL_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_ST_Count,
        -- Long-Term (LT) Aggregates
        SUM(CASE WHEN Is_LT THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_LT_Days,
        SUM(CASE WHEN Is_LT AND PWH_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_LT_Count,
        SUM(CASE WHEN Is_LT AND PWL_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_LT_Count,
        -- Year-to-Date (YTD) Aggregates
        SUM(CASE WHEN Is_YTD THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_YTD_Days,
        SUM(CASE WHEN Is_YTD AND PWH_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_YTD_Count,
        SUM(CASE WHEN Is_YTD AND PWL_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_YTD_Count
    FROM Daily_Context_With_DBS_Inputs
),
DBS_Score_Calculation AS (
    -- 5. Calculate and materialize the three final DBS scores (D.M2)
    SELECT DISTINCT
        trading_date, asset_id, Prior_Weekly_Type,
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk,
        D_M3_Continuation_Prob, D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob,
        -- DBS_ST_Score (Short-Term)
        ROUND(((PWH_Break_ST_Count::NUMERIC / NULLIF(Total_ST_Days, 0.0)) - (PWL_Break_ST_Count::NUMERIC / NULLIF(Total_ST_Days, 0.0)))::NUMERIC, 4) AS DBS_ST_Score,
        -- DBS_LT_Score (Long-Term)
        ROUND(((PWH_Break_LT_Count::NUMERIC / NULLIF(Total_LT_Days, 0.0)) - (PWL_Break_LT_Count::NUMERIC / NULLIF(Total_LT_Days, 0.0)))::NUMERIC, 4) AS DBS_LT_Score,
        -- DBS_YTD_Score (YTD)
        ROUND(((PWH_Break_YTD_Count::NUMERIC / NULLIF(Total_YTD_Days, 0.0)) - (PWL_Break_YTD_Count::NUMERIC / NULLIF(Total_YTD_Days, 0.0)))::NUMERIC, 4) AS DBS_YTD_Score
    FROM Conditional_DBS_Aggregates
    WHERE Prior_Weekly_Type IS NOT NULL
)
-- 6. Final Insertion into the Metrics Snapshot Table
INSERT INTO asset_daily_metrics_snapshot (
    trading_date, asset_id, Prior_Weekly_Type, DBS_ST_Score, DBS_LT_Score, DBS_YTD_Score, 
    D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk, D_M3_Continuation_Prob, 
    D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob
)
SELECT
    trading_date, asset_id, Prior_Weekly_Type, DBS_ST_Score, DBS_LT_Score, DBS_YTD_Score, 
    D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk, D_M3_Continuation_Prob, 
    D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob
FROM DBS_Score_Calculation
ON CONFLICT (trading_date, asset_id) 
DO UPDATE SET
    Prior_Weekly_Type = EXCLUDED.Prior_Weekly_Type, 
    DBS_ST_Score = EXCLUDED.DBS_ST_Score, 
    DBS_LT_Score = EXCLUDED.DBS_LT_Score, 
    DBS_YTD_Score = EXCLUDED.DBS_YTD_Score,
    D_M5_Bullish_Reversal_Risk = EXCLUDED.D_M5_Bullish_Reversal_Risk,
    D_M5_Bearish_Reversal_Risk = EXCLUDED.D_M5_Bearish_Reversal_Risk,
    D_M3_Continuation_Prob = EXCLUDED.D_M3_Continuation_Prob,
    D_M4_Bullish_FT_Prob = EXCLUDED.D_M4_Bullish_FT_Prob,
    D_M4_Bearish_FT_Prob = EXCLUDED.D_M4_Bearish_FT_Prob,
    Calculation_Date = NOW();


----------------------------------------

DROP TABLE asset_daily_metrics_snapshot;
-- Target table: Stores the output of ALL metric calculations (DBS Scores and M_n Factors)
CREATE TABLE IF NOT EXISTS asset_daily_metrics_snapshot (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    Prior_Weekly_Type TEXT,
    Prior_Day_Type TEXT,
    DOW_Transition TEXT,
    -- Final Calculated DBS Scores (D.M2 component)
    DBS_ST_Score NUMERIC(10, 4),
    DBS_LT_Score NUMERIC(10, 4),
    DBS_YTD_Score NUMERIC(10, 4),
    -- ALL Raw M_n Probabilities (used for factor calculation and logging)
    D_M5_Bullish_Reversal_Risk NUMERIC(10, 4),
    D_M5_Bearish_Reversal_Risk NUMERIC(10, 4),
    D_M3_Continuation_Prob NUMERIC(10, 4),
    D_M4_Bullish_FT_Prob NUMERIC(10, 4),
    D_M4_Bearish_FT_Prob NUMERIC(10, 4),
    Calculation_Date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    PRIMARY KEY(trading_date, asset_id)
);

TRUNCATE TABLE asset_daily_metrics_snapshot;

WITH Weekly_Ranges AS (
    -- 1. Get Prior Week High/Low/Type for joining
    SELECT
        week_start, asset_id,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWH,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWL,
        LAG(Weekly_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS Prior_Weekly_Type 
    FROM weekly_views
),
Daily_Context_Base AS (
    -- 2. Establish daily context: PD_Type, PDH, PDL, DOW_Transition
    SELECT
        trading_date, asset_id, "high", "low", "open", "close", dow, day_type,
        -- T-1 Context
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS PDH,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS PDL,
        LAG(dow, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || dow AS DOW_Transition
    FROM daily_views
),
D_M3_M4_Context AS (
    -- CTEs needed for D.M3/D.M4 Aggregation (using provided logic)
    SELECT 
        trading_date, asset_id, Prior_Day_Type, DOW_Transition,
        -- D.M3: Continuation Flag
        CASE WHEN LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) = 'Bullish' AND day_type = 'Bullish' THEN 1
             WHEN LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) = 'Bearish' AND day_type = 'Bearish' THEN 1
             ELSE 0 END AS Continuation,
        -- D.M4: Prior Break Type
        CASE WHEN LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) > LAG(PDH, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) THEN 'Bullish_Break'
             WHEN LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) < LAG(PDL, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) THEN 'Bearish_Break'
             ELSE 'No_Break' END AS Prior_Break_Type,
        CASE WHEN "close" > "open" THEN 'Bullish_Close' ELSE 'Bearish_Close' END AS Day_Close_Type
    FROM Daily_Context_Base
),
D_M5_Probabilities AS (
    -- D.M5: Daily DOW Reversal Risk (Aggregated only by DOW, NOT PD_Type)
    SELECT
        asset_id,
        dow AS CD_DOW,
        SUM(CASE WHEN high > PDH AND ("high" - "close") / ("high" - "low") >= 0.70 THEN 1 ELSE 0 END)::NUMERIC /
            NULLIF(SUM(CASE WHEN high > PDH THEN 1 ELSE 0 END), 0) AS D_M5_Bullish_Reversal_Risk,
        SUM(CASE WHEN low < PDL AND ("close" - "low") / ("high" - "low") <= 0.30 THEN 1 ELSE 0 END)::NUMERIC /
            NULLIF(SUM(CASE WHEN low < PDL THEN 1 ELSE 0 END), 0) AS D_M5_Bearish_Reversal_Risk
    FROM Daily_Context_Base
    WHERE PDH IS NOT NULL -- Ensure we have a prior day to compare
    GROUP BY 1, 2
),
M3_M4_Aggregated AS (
    -- 3. Calculate D.M3 and D.M4 Probabilities based on conditional keys
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
Daily_Context_With_DBS_Inputs AS (
    -- 4. Join all context and set DBS break flags and time markers
    SELECT
        D.trading_date, D.asset_id, W.Prior_Weekly_Type, D.Prior_Day_Type, D.DOW_Transition,
        -- Break Flags (Used for calculating the DBS scores)
        CASE WHEN D."high" > W.PWH THEN 1 ELSE 0 END AS PWH_Break,
        CASE WHEN D."low" < W.PWL THEN 1 ELSE 0 END AS PWL_Break,
        -- Time Horizon Markers (Used for conditional counting)
        CASE WHEN D.trading_date >= (SELECT MAX(trading_date) FROM asset_daily_views) - INTERVAL '6 months' THEN TRUE ELSE FALSE END AS Is_ST,
        CASE WHEN D.trading_date >= (SELECT MAX(trading_date) FROM asset_daily_views) - INTERVAL '3 years' THEN TRUE ELSE FALSE END AS Is_LT,
        CASE WHEN EXTRACT(YEAR FROM D.trading_date) = EXTRACT(YEAR FROM (SELECT MAX(trading_date) FROM asset_daily_views)) THEN TRUE ELSE FALSE END AS Is_YTD
    FROM Daily_Context_Base D
    INNER JOIN Weekly_Ranges W ON W.asset_id = D.asset_id AND DATE_TRUNC('week', D.trading_date) = W.week_start
    WHERE D.Prior_Day_Type IS NOT NULL AND W.PWH IS NOT NULL
),
Conditional_DBS_Aggregates AS (
    -- 5. Calculate conditional counts for all timeframes (Full DBS logic)
    SELECT
        trading_date, asset_id, Prior_Weekly_Type, Prior_Day_Type, DOW_Transition,
        -- Short-Term (ST) Aggregates
        SUM(CASE WHEN Is_ST THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_ST_Days,
        SUM(CASE WHEN Is_ST AND PWH_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_ST_Count,
        SUM(CASE WHEN Is_ST AND PWL_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_ST_Count,
        -- Long-Term (LT) Aggregates
        SUM(CASE WHEN Is_LT THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_LT_Days,
        SUM(CASE WHEN Is_LT AND PWH_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_LT_Count,
        SUM(CASE WHEN Is_LT AND PWL_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_LT_Count,
        -- Year-to-Date (YTD) Aggregates
        SUM(CASE WHEN Is_YTD THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_YTD_Days,
        SUM(CASE WHEN Is_YTD AND PWH_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_YTD_Count,
        SUM(CASE WHEN Is_YTD AND PWL_Break = 1 THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_YTD_Count
        
    FROM Daily_Context_With_DBS_Inputs
),
DBS_Score_Calculation AS (
    -- 6. Calculate the three final DBS scores (D.M2) and merge all M_n results
    SELECT DISTINCT ON (D.asset_id, D.trading_date)
        D.trading_date, D.asset_id, D.Prior_Weekly_Type, D.Prior_Day_Type, D.DOW_Transition,
        -- Final DBS Scores (D.M2)
        ROUND(((C.PWH_Break_ST_Count::NUMERIC / NULLIF(C.Total_ST_Days, 0.0)) - (C.PWL_Break_ST_Count::NUMERIC / NULLIF(C.Total_ST_Days, 0.0)))::NUMERIC, 4) AS DBS_ST_Score,
        ROUND(((C.PWH_Break_LT_Count::NUMERIC / NULLIF(C.Total_LT_Days, 0.0)) - (C.PWL_Break_LT_Count::NUMERIC / NULLIF(C.Total_LT_Days, 0.0)))::NUMERIC, 4) AS DBS_LT_Score,
        ROUND(((C.PWH_Break_YTD_Count::NUMERIC / NULLIF(C.Total_YTD_Days, 0.0)) - (C.PWL_Break_YTD_Count::NUMERIC / NULLIF(C.Total_YTD_Days, 0.0)))::NUMERIC, 4) AS DBS_YTD_Score,
        -- Merge M3, M4, M5 probabilities
        COALESCE(M34.D_M3_Continuation_Prob, 0.5) AS D_M3_Continuation_Prob,
        COALESCE(M34.D_M4_Bullish_FT_Prob, 0.5) AS D_M4_Bullish_FT_Prob,
        COALESCE(M34.D_M4_Bearish_FT_Prob, 0.5) AS D_M4_Bearish_FT_Prob,
        COALESCE(M5.D_M5_Bullish_Reversal_Risk, 0.5) AS D_M5_Bullish_Reversal_Risk, 
        COALESCE(M5.D_M5_Bearish_Reversal_Risk, 0.5) AS D_M5_Bearish_Reversal_Risk
    FROM Daily_Context_With_DBS_Inputs D
    INNER JOIN Conditional_DBS_Aggregates C 
        ON D.trading_date = C.trading_date AND D.asset_id = C.asset_id
    LEFT JOIN M3_M4_Aggregated M34 
        ON D.asset_id = M34.asset_id AND D.Prior_Day_Type = M34.Prior_Day_Type AND D.DOW_Transition = M34.DOW_Transition
    LEFT JOIN D_M5_Probabilities M5 
        ON D.asset_id = M5.asset_id AND TRIM(D.DOW_Transition) LIKE '%-> ' || M5.CD_DOW
)
-- 7. Final Insertion into the Metrics Snapshot Table (The destination)
INSERT INTO asset_daily_metrics_snapshot (
    trading_date, asset_id, Prior_Weekly_Type, Prior_Day_Type, DOW_Transition,
    DBS_ST_Score, DBS_LT_Score, DBS_YTD_Score, 
    D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk, 
    D_M3_Continuation_Prob, D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob
)
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