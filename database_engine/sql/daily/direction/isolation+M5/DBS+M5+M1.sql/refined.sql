

TRUNCATE asset_daily_composite_score;

WITH Weekly_Ranges AS (
    -- 1. Calculate the Prior Week High (PWH), Low (PWL), and Type (PWT)
    SELECT
        week_start, asset_id,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWH,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWL,
        -- 🔥 NEW: Fetch the Weekly_Type of the prior week (PWT)
        LAG(Weekly_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS Prior_Weekly_Type 
    FROM asset_weekly_views
),
Daily_Context_With_Prior AS (
    -- 2. Calculate the Prior Day context for joining D.M5 (Fixes the window function error)
    SELECT
        trading_date, asset_id, "high", "low", day_type,
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(dow, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || dow AS DOW_Transition
    FROM asset_daily_views
),
Daily_Context_With_DBS_Inputs AS (
    -- 3. Join Daily Context, Weekly Ranges, and D.M5 Risk Metrics
    SELECT
        D.trading_date, D.asset_id, D."high", D."low", 
        D.Prior_Day_Type, D.DOW_Transition,
        W.PWH, W.PWL,
        W.Prior_Weekly_Type, -- ✅ Prior Week Type added here
        COALESCE(CM.D_M5_Bullish_Reversal_Risk, 0.0) AS D_M5_Bullish_Reversal_Risk,
        COALESCE(CM.D_M5_Bearish_Reversal_Risk, 0.0) AS D_M5_Bearish_Reversal_Risk,
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
    WHERE W.PWH IS NOT NULL AND D.Prior_Day_Type IS NOT NULL AND W.Prior_Weekly_Type IS NOT NULL -- ✅ PWT filter added
),
Conditional_DBS_Counts AS (
    -- 4. Calculate conditional counts for DBS scores (PWH/PWL)
    SELECT
        trading_date, asset_id, Prior_Day_Type, DOW_Transition, Prior_Weekly_Type, -- ✅ PWT added
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk,
        -- Total Days for Aggregation (The divisor for probability)
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
    -- 5. Calculates and aliases the three DBS scores cleanly
    SELECT DISTINCT
        trading_date, asset_id, Prior_Weekly_Type,
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk,
        (PWH_Break_ST_Count::NUMERIC / NULLIF(Total_ST_Days, 0.0)) - (PWL_Break_ST_Count::NUMERIC / NULLIF(Total_ST_Days, 0.0)) AS DBS_ST_Score,
        (PWH_Break_LT_Count::NUMERIC / NULLIF(Total_LT_Days, 0.0)) - (PWL_Break_LT_Count::NUMERIC / NULLIF(Total_LT_Days, 0.0)) AS DBS_LT_Score,
        (PWH_Break_YTD_Count::NUMERIC / NULLIF(Total_YTD_Days, 0.0)) - (PWL_Break_YTD_Count::NUMERIC / NULLIF(Total_YTD_Days, 0.0)) AS DBS_YTD_Score
    FROM Conditional_DBS_Counts
    WHERE Prior_Day_Type IS NOT NULL
),
DCS_Filter_Factors AS (
    -- 🔥 NEW CTE: Calculates the two filter factors based on DBS_ST_Score 🔥
    SELECT
        trading_date, asset_id,
        DBS_ST_Score, DBS_LT_Score, DBS_YTD_Score,
        D_M5_Bullish_Reversal_Risk AS F_Reversal_Log,
        -- 1. D.M5 Reversal Risk Filter (F_Reversal)
        (CASE 
            WHEN DBS_ST_Score >= 0 
            THEN (1.0 - 0.2 * D_M5_Bullish_Reversal_Risk) 
            ELSE (1.0 - 0.2 * D_M5_Bearish_Reversal_Risk) 
        END) AS F_Reversal_Filter,
        -- 2. D.M1 Prior Week Type Confirmation Factor (F_D.M1)
        (CASE
            WHEN Prior_Weekly_Type IN ('Bullish') AND DBS_ST_Score >= 0 THEN 1.05
            WHEN Prior_Weekly_Type IN ('Bullish') AND DBS_ST_Score < 0 THEN 0.95
            WHEN Prior_Weekly_Type IN ('Bearish') AND DBS_ST_Score < 0 THEN 1.05
            WHEN Prior_Weekly_Type IN ('Bearish') AND DBS_ST_Score >= 0 THEN 0.95
            ELSE 1.00 
        END) AS F_DM1_Filter
    FROM DBS_Score_Calculation
),
DCS_Refined_Calculation AS (
    -- 🔥 Final CTE: Uses all materialized column aliases for final multiplication 🔥
    SELECT
        trading_date, asset_id,
        F_Reversal_Log, F_Reversal_Filter, F_DM1_Filter,
        DBS_ST_Score, DBS_LT_Score, DBS_YTD_Score,
        -- Final Refined Score: DBS_Strongest * F_Reversal * F_DM1
        (CASE
            -- Condition: Check if the maximum absolute DBS score is above the 0.25 threshold
            WHEN GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) > 0.25 
            THEN 
                -- Select the strongest DBS score
                (CASE
                    WHEN ABS(DBS_ST_Score) = GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) THEN DBS_ST_Score
                    WHEN ABS(DBS_LT_Score) = GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) THEN DBS_LT_Score
                    ELSE DBS_YTD_Score
                END)
                * F_Reversal_Filter -- D.M5 Filter (Now exists!)
                * F_DM1_Filter      -- D.M1 Filter (Now exists!)
            ELSE 0.0 
        END) AS DCS_Targeted_Score
    FROM DCS_Filter_Factors
)
-- 8. Final Insertion (Insertion logic remains the same)
INSERT INTO asset_daily_composite_score (
    trading_date, asset_id, DCS, DCS_Classification, F_Reversal
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
    
    ROUND(d.F_DM1_Filter::NUMERIC, 4) AS F_Reversal 
FROM DCS_Refined_Calculation d
ON CONFLICT (trading_date, asset_id) 
DO UPDATE SET
    DCS = EXCLUDED.DCS, DCS_Classification = EXCLUDED.DCS_Classification, F_Reversal = EXCLUDED.F_Reversal;