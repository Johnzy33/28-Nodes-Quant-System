

WITH Day_DOW_Context AS (
    -- 1. Derive DOW from trading_date and add context
    SELECT
        trading_date, asset_id, day_type, "high", "low",
        TRIM(TRAILING ' ' FROM TO_CHAR(trading_date, 'Day')) AS dow
    FROM daily_views
),
latest_day_context AS (
    -- 2. Get latest day's data and all prior context (PWH/PWL from weekly_views)
    SELECT
        D.trading_date, D.asset_id, D.day_type, D.dow,
        LAG(D.day_type, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) AS Prior_Day_Type,
        LAG(D.dow, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) || ' -> ' || D.dow AS DOW_Transition,
        LAG(W.high, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) AS PWH,
        LAG(W.low, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) AS PWL,
        D."high", D."low"
    FROM Day_DOW_Context D
    INNER JOIN weekly_views W ON W.asset_id = D.asset_id AND DATE_TRUNC('week', D.trading_date) = W.week_start
    WHERE D.trading_date = (SELECT MAX(trading_date) FROM daily_views) -- Max trading date is 2025-11-04
),
latest_break_data AS (
    -- 3. Determine the break flags and time horizons for the latest day (2025-11-04)
    SELECT
        asset_id, Prior_Day_Type, DOW_Transition,
        CASE WHEN high > PWH THEN 1 ELSE 0 END AS PWH_Break,
        CASE WHEN low < PWL THEN 1 ELSE 0 END AS PWL_Break,
        -- All time horizons apply for the latest day update
        TRUE AS Is_ST, -- Assuming ST is the full lookback for history agg table
        TRUE AS Is_LT,
        (EXTRACT(YEAR FROM trading_date) = EXTRACT(YEAR FROM (SELECT MAX(trading_date) FROM daily_views))) AS Is_YTD
    FROM latest_day_context
    WHERE Prior_Day_Type IS NOT NULL AND PWH IS NOT NULL
)
SELECT
    asset_id, Prior_Day_Type, DOW_Transition,
    SUM(CASE WHEN Is_ST THEN 1 ELSE 0 END) AS Total_ST_Days,
    SUM(CASE WHEN Is_ST AND PWH_Break = 1 THEN 1 ELSE 0 END) AS PWH_Break_ST_Count,
    SUM(CASE WHEN Is_ST AND PWL_Break = 1 THEN 1 ELSE 0 END) AS PWL_Break_ST_Count,
    
    SUM(CASE WHEN Is_LT THEN 1 ELSE 0 END) AS Total_LT_Days,
    SUM(CASE WHEN Is_LT AND PWH_Break = 1 THEN 1 ELSE 0 END) AS PWH_Break_LT_Count,
    SUM(CASE WHEN Is_LT AND PWL_Break = 1 THEN 1 ELSE 0 END) AS PWL_Break_LT_Count,
    
    SUM(CASE WHEN Is_YTD THEN 1 ELSE 0 END) AS Total_YTD_Days,
    SUM(CASE WHEN Is_YTD AND PWH_Break = 1 THEN 1 ELSE 0 END) AS PWH_Break_YTD_Count,
    SUM(CASE WHEN Is_YTD AND PWL_Break = 1 THEN 1 ELSE 0 END) AS PWL_Break_YTD_Count
FROM latest_break_data
GROUP BY 1, 2, 3;

WITH Context AS (
    SELECT
        trading_date, asset_id, day_type,
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        DATE_TRUNC('week', trading_date) AS week_start
    FROM daily_views
)
SELECT 
    C.trading_date,
    C.Prior_Day_Type,
    W.Weekly_Type AS Current_Weekly_Type,
    LAG(W.Weekly_Type, 1) OVER (PARTITION BY C.asset_id ORDER BY C.trading_date) AS Prior_Weekly_Type
FROM Context C
LEFT JOIN weekly_views W 
    ON W.asset_id = C.asset_id 
    AND C.week_start = W.week_start
WHERE C.trading_date >= '2025-10-31'
ORDER BY C.trading_date DESC;