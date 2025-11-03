


DROP TABLE IF EXISTS asset_daily_directional_comparison;

DROP TABLE IF EXISTS asset_daily_directional_comparison;
CREATE TABLE asset_daily_directional_comparison (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    -- ST Score (6 months)
    DBS_ST NUMERIC(10, 4),
    DBS_ST_Classification TEXT,
    -- LT Score (3 years)
    DBS_LT NUMERIC(10, 4),
    DBS_LT_Classification TEXT,
    -- YTD Score (Current Year)
    DBS_YTD NUMERIC(10, 4),
    DBS_YTD_Classification TEXT,
    
    PRIMARY KEY (trading_date, asset_id)
);

DROP TABLE IF EXISTS asset_daily_directional_comparison;

DROP TABLE IF EXISTS asset_daily_directional_comparison;

TRUNCATE asset_daily_directional_comparison;

WITH Weekly_Ranges AS (
    -- 1. Calculate the Prior Week High (PWH) and Prior Week Low (PWL)
    SELECT
        week_start,
        asset_id,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWH,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWL
    FROM asset_weekly_views
),
Daily_Context AS (
    -- 2. Join Daily Context with the Prior Weekly Range and determine Breaks
    SELECT
        D.trading_date, D.asset_id, D."high", D."low", 
        LAG(D.day_type, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) AS Prior_Day_Type,
        LAG(D.dow, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) || ' -> ' || D.dow AS DOW_Transition,
        -- Join PWH/PWL to the daily row (approximated by week_start)
        W.PWH, W.PWL,
        -- Break Flags
        CASE WHEN D."high" > W.PWH THEN TRUE ELSE FALSE END AS PWH_Break,
        CASE WHEN D."low" < W.PWL THEN TRUE ELSE FALSE END AS PWL_Break,
        -- Time Horizon Markers
        CASE WHEN D.trading_date >= (SELECT MAX(trading_date) FROM asset_daily_views) - INTERVAL '3 months' THEN TRUE ELSE FALSE END AS Is_ST,
        CASE WHEN D.trading_date >= (SELECT MAX(trading_date) FROM asset_daily_views) - INTERVAL '6 months' THEN TRUE ELSE FALSE END AS Is_LT,
        CASE WHEN EXTRACT(YEAR FROM D.trading_date) = EXTRACT(YEAR FROM (SELECT MAX(trading_date) FROM asset_daily_views)) THEN TRUE ELSE FALSE END AS Is_YTD
    FROM asset_daily_views D
    INNER JOIN Weekly_Ranges W ON W.asset_id = D.asset_id AND DATE_TRUNC('week', D.trading_date) = W.week_start
    WHERE W.PWH IS NOT NULL -- Exclude the first week where PWH/PWL is NULL
),
Conditional_PWH_PWL_Counts AS (
    -- 3. Calculate conditional counts and total occurrences for each context
    SELECT
        trading_date, asset_id, Prior_Day_Type, DOW_Transition,
        -- Total Days for Aggregation (The divisor for probability)
        SUM(CASE WHEN Is_ST THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_ST_Days,
        SUM(CASE WHEN Is_LT THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_LT_Days,
        SUM(CASE WHEN Is_YTD THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS Total_YTD_Days,
        -- ST (6 Months) Break Counts
        SUM(CASE WHEN Is_ST AND PWH_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_ST_Count,
        SUM(CASE WHEN Is_ST AND PWL_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_ST_Count,
        -- LT (3 Years) Break Counts
        SUM(CASE WHEN Is_LT AND PWH_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_LT_Count,
        SUM(CASE WHEN Is_LT AND PWL_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_LT_Count,
        -- YTD (Current Year) Break Counts
        SUM(CASE WHEN Is_YTD AND PWH_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWH_Break_YTD_Count,
        SUM(CASE WHEN Is_YTD AND PWL_Break THEN 1 ELSE 0 END) OVER (PARTITION BY asset_id, Prior_Day_Type, DOW_Transition) AS PWL_Break_YTD_Count
    FROM Daily_Context
),
DBS_Calculation AS (
    -- 4. Calculate final DBS scores (Probability Difference)
    SELECT DISTINCT
        trading_date, asset_id,
        -- ST Score (6 Months)
        (PWH_Break_ST_Count::NUMERIC / NULLIF(Total_ST_Days, 0.0)) - (PWL_Break_ST_Count::NUMERIC / NULLIF(Total_ST_Days, 0.0)) AS DBS_ST_Score,
        -- LT Score (3 Years)
        (PWH_Break_LT_Count::NUMERIC / NULLIF(Total_LT_Days, 0.0)) - (PWL_Break_LT_Count::NUMERIC / NULLIF(Total_LT_Days, 0.0)) AS DBS_LT_Score,
        -- YTD Score (Current Year)
        (PWH_Break_YTD_Count::NUMERIC / NULLIF(Total_YTD_Days, 0.0)) - (PWL_Break_YTD_Count::NUMERIC / NULLIF(Total_YTD_Days, 0.0)) AS DBS_YTD_Score
    FROM Conditional_PWH_PWL_Counts
    WHERE Prior_Day_Type IS NOT NULL 
)
-- 5. Final Insertion (Unchanged)
INSERT INTO asset_daily_directional_comparison (
    trading_date, asset_id, 
    DBS_ST, DBS_ST_Classification, 
    DBS_LT, DBS_LT_Classification, 
    DBS_YTD, DBS_YTD_Classification 
)
SELECT
    d.trading_date, d.asset_id,
    
    ROUND(d.DBS_ST_Score::NUMERIC, 4) AS DBS_ST,
    CASE
        WHEN d.DBS_ST_Score >= 0.50 THEN 'High Conviction Long'
        WHEN d.DBS_ST_Score <= -0.50 THEN 'High Conviction Short'
        WHEN ABS(d.DBS_ST_Score) > 0.25 THEN 'Medium Conviction'
        ELSE 'Neutral/Low Conviction'
    END AS DBS_ST_Classification,
    
    ROUND(d.DBS_LT_Score::NUMERIC, 4) AS DBS_LT,
    CASE
        WHEN d.DBS_LT_Score >= 0.50 THEN 'High Conviction Long'
        WHEN d.DBS_LT_Score <= -0.50 THEN 'High Conviction Short'
        WHEN ABS(d.DBS_LT_Score) > 0.25 THEN 'Medium Conviction'
        ELSE 'Neutral/Low Conviction'
    END AS DBS_LT_Classification,
    
    ROUND(d.DBS_YTD_Score::NUMERIC, 4) AS DBS_YTD,
    CASE
        WHEN d.DBS_YTD_Score >= 0.50 THEN 'High Conviction Long'
        WHEN d.DBS_YTD_Score <= -0.50 THEN 'High Conviction Short'
        WHEN ABS(d.DBS_YTD_Score) > 0.25 THEN 'Medium Conviction'
        ELSE 'Neutral/Low Conviction'
    END AS DBS_YTD_Classification
FROM DBS_Calculation d
ON CONFLICT (trading_date, asset_id) 
DO UPDATE SET
    DBS_ST = EXCLUDED.DBS_ST, DBS_ST_Classification = EXCLUDED.DBS_ST_Classification,
    DBS_LT = EXCLUDED.DBS_LT, DBS_LT_Classification = EXCLUDED.DBS_LT_Classification,
    DBS_YTD = EXCLUDED.DBS_YTD, DBS_YTD_Classification = EXCLUDED.DBS_YTD_Classification;