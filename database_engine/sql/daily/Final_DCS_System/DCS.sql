
--------------------
-- The final System 
-------------------
DROP TABLE asset_daily_composite_score;
-- D.C.S Target Table Definition (Ensures all logging columns are present)
CREATE TABLE IF NOT EXISTS asset_daily_composite_score (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,                   -- Final Daily Composite Score (Calculated with D.M2, D.M5, D.M1)
    DCS_Classification TEXT,                -- Final categorical output
    DCS NUMERIC(10, 4),
    -- LOGGING COLUMNS (Historical M_n factors for review)
    F_Reversal NUMERIC(10, 4),              -- Active D.M5 Factor (1.0 - 0.2 * Risk)
    F_DM1_Factor NUMERIC(10, 4),            -- Active D.M1 Factor (1.05 / 0.95)
    F_Commitment NUMERIC(10, 4),            -- D.M3 Factor (0.8 + 0.4 * Prob)
    F_Sustainability NUMERIC(10, 4),        -- D.M4 Factor (Follow-Through Ratio)
    
    PRIMARY KEY (trading_date, asset_id)
);

TRUNCATE asset_daily_composite_score;

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
        trading_date, asset_id, "high", "low", day_type,
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(dow, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || dow AS DOW_Transition
    FROM asset_daily_views
),
Daily_Context_With_DBS_Inputs AS (
    -- 3. Join Daily Context, Weekly Ranges, and all M_n Risk Metrics
    SELECT
        D.trading_date, D.asset_id, D."high", D."low", 
        D.Prior_Day_Type, D.DOW_Transition,
        W.PWH, W.PWL, W.Prior_Weekly_Type, 
        -- Fetch ALL base M_n probabilities needed for current calculation AND logging
        COALESCE(CM.D_M5_Bullish_Reversal_Risk, 0.0) AS D_M5_Bullish_Reversal_Risk,
        COALESCE(CM.D_M5_Bearish_Reversal_Risk, 0.0) AS D_M5_Bearish_Reversal_Risk,
        COALESCE(CM.D_M3_Continuation_Prob, 0.0) AS D_M3_Continuation_Prob, -- 🔥 LOGGING
        COALESCE(CM.D_M4_Bullish_FT_Prob, 0.0) AS D_M4_Bullish_FT_Prob,     -- 🔥 LOGGING
        COALESCE(CM.D_M4_Bearish_FT_Prob, 0.0) AS D_M4_Bearish_FT_Prob,     -- 🔥 LOGGING
        -- Break Flags and Time Horizon Markers
        CASE WHEN D."high" > W.PWH THEN TRUE ELSE FALSE END AS PWH_Break,
        CASE WHEN D."low" < W.PWL THEN TRUE ELSE FALSE END AS PWL_Break,
        CASE WHEN D.trading_date >= (SELECT MAX(trading_date) FROM asset_daily_views) - INTERVAL '6 months' THEN TRUE ELSE FALSE END AS Is_ST,
        CASE WHEN D.trading_date >= (SELECT MAX(trading_date) FROM asset_daily_views) - INTERVAL '3 years' THEN TRUE ELSE FALSE END AS Is_LT,
        CASE WHEN EXTRACT(YEAR FROM D.trading_date) = EXTRACT(YEAR FROM (SELECT MAX(trading_date) FROM asset_daily_views)) THEN TRUE ELSE FALSE END AS Is_YTD
    FROM Daily_Context_With_Prior D
    INNER JOIN Weekly_Ranges W ON W.asset_id = D.asset_id AND DATE_TRUNC('week', D.trading_date) = W.week_start
    LEFT JOIN asset_daily_conditional_metrics CM 
        ON CM.asset_id = D.asset_id 
        AND CM.PD_Type = D.Prior_Day_Type 
        AND CM.DOW_Transition = D.DOW_Transition
    WHERE W.PWH IS NOT NULL AND D.Prior_Day_Type IS NOT NULL AND W.Prior_Weekly_Type IS NOT NULL
),
Conditional_DBS_Counts AS (
    -- 4. Calculate conditional counts for DBS scores (PWH/PWL) and pass through M_n inputs
    SELECT
        trading_date, asset_id, Prior_Day_Type, DOW_Transition, Prior_Weekly_Type,
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk, 
        D_M3_Continuation_Prob, D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob, -- 🔥 LOGGING INPUTS
        -- Total Days and Break Counts for all timeframes...
        SUM(CASE WHEN Is_ST THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_ST_Days,
        SUM(CASE WHEN Is_LT THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_LT_Days,
        SUM(CASE WHEN Is_YTD THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_YTD_Days,
        -- ST/LT/YTD Break Counts (Used for calculating DBS scores)

        SUM(CASE WHEN Is_ST AND PWH_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_ST_Count,
        SUM(CASE WHEN Is_ST AND PWL_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_ST_Count,
        SUM(CASE WHEN Is_LT AND PWH_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_LT_Count,
        SUM(CASE WHEN Is_LT AND PWL_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_LT_Count,
        SUM(CASE WHEN Is_YTD AND PWH_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_YTD_Count,
        SUM(CASE WHEN Is_YTD AND PWL_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_YTD_Count
    FROM Daily_Context_With_DBS_Inputs
),
DBS_Score_Calculation AS (
    -- 5. Calculate and materialize the three DBS scores
    SELECT DISTINCT
        trading_date, asset_id, Prior_Weekly_Type,
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk,
        D_M3_Continuation_Prob, D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob, -- 🔥 LOGGING INPUTS
        
        (PWH_Break_ST_Count::NUMERIC / NULLIF(Total_ST_Days, 0.0)) - (PWL_Break_ST_Count::NUMERIC / NULLIF(Total_ST_Days, 0.0)) AS DBS_ST_Score,
        (PWH_Break_LT_Count::NUMERIC / NULLIF(Total_LT_Days, 0.0)) - (PWL_Break_LT_Count::NUMERIC / NULLIF(Total_LT_Days, 0.0)) AS DBS_LT_Score,
        (PWH_Break_YTD_Count::NUMERIC / NULLIF(Total_YTD_Days, 0.0)) - (PWL_Break_YTD_Count::NUMERIC / NULLIF(Total_YTD_Days, 0.0)) AS DBS_YTD_Score
    FROM Conditional_DBS_Counts
    WHERE Prior_Day_Type IS NOT NULL
),
DCS_Filter_Factors AS (
    -- 6. Calculate all four factors (F_Reversal and F_DM1 are used for the final score)
    SELECT
        trading_date, asset_id,
        DBS_ST_Score, DBS_LT_Score, DBS_YTD_Score,
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk,
        -- 🔥 F_Commitment (D.M3) - LOGGING ONLY 🔥
        (0.8 + 0.4 * D_M3_Continuation_Prob) AS F_Commitment, 
        -- 🔥 F_Sustainability (D.M4) - LOGGING ONLY 🔥
        CASE 
            WHEN DBS_ST_Score >= 0 THEN 
                (D_M4_Bullish_FT_Prob + 0.1) / (1.0 - D_M4_Bullish_FT_Prob + 0.1) 
            ELSE 
                (D_M4_Bearish_FT_Prob + 0.1) / (1.0 - D_M4_Bearish_FT_Prob + 0.1) 
        END AS F_Sustainability,
        -- F_Reversal (D.M5) - ACTIVE FILTER
        (CASE 
            WHEN DBS_ST_Score >= 0 THEN (1.0 - 0.2 * D_M5_Bullish_Reversal_Risk) 
            ELSE (1.0 - 0.2 * D_M5_Bearish_Reversal_Risk) 
        END) AS F_Reversal_Factor, 
        -- F_DM1_Factor (D.M1) - ACTIVE FILTER
        (CASE
            WHEN Prior_Weekly_Type IN ('Bullish') AND DBS_ST_Score >= 0 THEN 1.05
            WHEN Prior_Weekly_Type IN ('Bullish') AND DBS_ST_Score < 0 THEN 0.95
            WHEN Prior_Weekly_Type IN ('Bearish') AND DBS_ST_Score < 0 THEN 1.05
            WHEN Prior_Weekly_Type IN ('Bearish') AND DBS_ST_Score >= 0 THEN 0.95
            ELSE 1.00 
        END) AS F_DM1_Factor
    FROM DBS_Score_Calculation
),
DCS_Refined_Calculation AS (
    -- 7. Final Calculation of DCS_Refined (using only F_Reversal_Factor and F_DM1_Factor)
    SELECT
        trading_date, asset_id,
        F_Reversal_Factor, F_DM1_Factor, F_Commitment, F_Sustainability,
        DBS_ST_Score, DBS_LT_Score, DBS_YTD_Score,
        
        (CASE
            WHEN GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) > 0.25 
            THEN 
                (CASE
                    WHEN ABS(DBS_ST_Score) = GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) THEN DBS_ST_Score
                    WHEN ABS(DBS_LT_Score) = GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) THEN DBS_LT_Score
                    ELSE DBS_YTD_Score
                END)
                * F_Reversal_Factor 
                * F_DM1_Factor      
            ELSE 0.0 
        END) AS DCS_Targeted_Score
    FROM DCS_Filter_Factors
)
-- 8. Final Insertion with Comprehensive Logging
INSERT INTO asset_daily_composite_score (
    trading_date, asset_id, DCS, DCS_Classification, 
    F_Reversal, F_DM1_Factor, F_Commitment, F_Sustainability
)
SELECT
    d.trading_date, d.asset_id,
    ROUND(d.DCS_Targeted_Score::NUMERIC, 4) AS DCS,
    CASE
        WHEN d.DCS_Targeted_Score >= 0.50 THEN 'High Conviction Long'
        WHEN d.DCS_Targeted_Score > 0.25 THEN 'Medium Conviction Long'
        WHEN d.DCS_Targeted_Score < -0.50 THEN 'High Conviction Short'
        WHEN d.DCS_Targeted_Score < -0.25 THEN 'Medium Conviction Short'
        ELSE 'Neutral/Low Conviction'
    END AS DCS_Classification,
    
    ROUND(d.F_Reversal_Factor::NUMERIC, 4) AS F_Reversal, 
    ROUND(d.F_DM1_Factor::NUMERIC, 4) AS F_DM1_Factor,   
    ROUND(d.F_Commitment::NUMERIC, 4) AS F_Commitment,   
    ROUND(d.F_Sustainability::NUMERIC, 4) AS F_Sustainability
FROM DCS_Refined_Calculation d
ON CONFLICT (trading_date, asset_id) 
DO UPDATE SET
    DCS = EXCLUDED.DCS, DCS_Classification = EXCLUDED.DCS_Classification, 
    F_Reversal = EXCLUDED.F_Reversal, F_DM1_Factor = EXCLUDED.F_DM1_Factor,
    F_Commitment = EXCLUDED.F_Commitment, F_Sustainability = EXCLUDED.F_Sustainability;