--------------------------------------
-- This run the full aggregatione onces and updates it increamentally
-----------------------------------------------------------

-- New Table to store historical DBS calculation inputs for fast lookup
CREATE TABLE IF NOT EXISTS dbs_history_agg (
    asset_id TEXT NOT NULL,
    prior_day_type TEXT NOT NULL,
    dow_transition TEXT NOT NULL,
    -- Short-Term (6 Months)
    total_st_days INTEGER DEFAULT 0,
    pwh_break_st_count INTEGER DEFAULT 0,
    pwl_break_st_count INTEGER DEFAULT 0,
    -- Long-Term (3 Years)
    total_lt_days INTEGER DEFAULT 0,
    pwh_break_lt_count INTEGER DEFAULT 0,
    pwl_break_lt_count INTEGER DEFAULT 0,
    -- Year-to-Date
    total_ytd_days INTEGER DEFAULT 0,
    pwh_break_ytd_count INTEGER DEFAULT 0,
    pwl_break_ytd_count INTEGER DEFAULT 0,
    
    PRIMARY KEY(asset_id, prior_day_type, dow_transition)
);

CALL update_dbs_history();

DELETE FROM dbs_history_agg;

CREATE OR REPLACE PROCEDURE update_dbs_history()
LANGUAGE sql
AS $$
WITH Daily_Derived_Context AS (
    -- 1. Derive DOW and Prior Day Context for ALL days
    SELECT
        trading_date, asset_id, day_type, "high", "low",
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        TRIM(TRAILING ' ' FROM TO_CHAR(trading_date, 'Day')) AS Current_DOW
    FROM daily_views
),
Daily_Context_With_DOW_Transition AS (
    -- Calculate the DOW Transition
    SELECT
        D.*,
        LAG(D.Current_DOW, 1) OVER (PARTITION BY D.asset_id ORDER BY D.trading_date) || ' -> ' || D.Current_DOW AS DOW_Transition
    FROM Daily_Derived_Context D
),
Weekly_Lag_Data AS (
    -- 2. Get the Lagged Weekly High/Low (PWH/PWL)
    SELECT 
        asset_id, week_start,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWH,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWL
    FROM weekly_views
),
latest_day_context AS (
    -- 3. Get the latest day's data and necessary context (MAX date only)
    SELECT
        D.trading_date, D.asset_id, D.Prior_Day_Type, D.DOW_Transition,
        W.PWH, W.PWL, -- Prior Week High/Low from the *previous* week's data
        D."high", D."low"
    FROM Daily_Context_With_DOW_Transition D
    INNER JOIN weekly_views W_Curr 
        ON W_Curr.asset_id = D.asset_id AND DATE_TRUNC('week', D.trading_date) = W_Curr.week_start
    INNER JOIN Weekly_Lag_Data W 
        ON W.asset_id = W_Curr.asset_id AND W.week_start = DATE_TRUNC('week', D.trading_date) -- Join PWH/PWL for the current week's calculation
          WHERE D.trading_date = (SELECT MAX(trading_date) FROM daily_views) -- Max trading date (2025-11-04)
),
latest_break_data AS (
    -- 4. Determine the break flags and ensure context exists
    SELECT
        asset_id, Prior_Day_Type, DOW_Transition,
        CASE WHEN "high" > PWH THEN 1 ELSE 0 END AS PWH_Break,
        CASE WHEN "low" < PWL THEN 1 ELSE 0 END AS PWL_Break,
        TRUE AS Is_ST, TRUE AS Is_LT, 
        (EXTRACT(YEAR FROM trading_date) = EXTRACT(YEAR FROM (SELECT MAX(trading_date) FROM daily_views))) AS Is_YTD
    FROM latest_day_context
    WHERE Prior_Day_Type IS NOT NULL AND PWH IS NOT NULL -- The critical filter
),
incremental_update AS (
    -- 5. Calculate the incremental change (always 1 for total days or break count)
    SELECT
        asset_id, Prior_Day_Type, DOW_Transition,
        SUM(Is_ST::INT) AS Total_ST_Days, SUM(PWH_Break) AS PWH_Break_ST_Count, SUM(PWL_Break) AS PWL_Break_ST_Count,
        SUM(Is_LT::INT) AS Total_LT_Days, SUM(PWH_Break) AS PWH_Break_LT_Count, SUM(PWL_Break) AS PWL_Break_LT_Count,
        SUM(Is_YTD::INT) AS Total_YTD_Days, SUM(PWH_Break) AS PWH_Break_YTD_Count, SUM(PWL_Break) AS PWL_Break_YTD_Count
    FROM latest_break_data
    GROUP BY 1, 2, 3
)
INSERT INTO dbs_history_agg (
    asset_id, prior_day_type, dow_transition,
    total_st_days, pwh_break_st_count, pwl_break_st_count,
    total_lt_days, pwh_break_lt_count, pwl_break_lt_count,
    total_ytd_days, pwh_break_ytd_count, pwl_break_ytd_count
)
SELECT * FROM incremental_update
ON CONFLICT (asset_id, prior_day_type, dow_transition) DO UPDATE SET
    total_st_days = dbs_history_agg.total_st_days + EXCLUDED.total_st_days,
    pwh_break_st_count = dbs_history_agg.pwh_break_st_count + EXCLUDED.pwh_break_st_count,
    pwl_break_st_count = dbs_history_agg.pwl_break_lt_count + EXCLUDED.pwl_break_st_count,
    total_lt_days = dbs_history_agg.total_lt_days + EXCLUDED.total_lt_days,
    pwh_break_lt_count = dbs_history_agg.pwh_break_lt_count + EXCLUDED.pwh_break_lt_count,
    pwl_break_lt_count = dbs_history_agg.pwl_break_lt_count + EXCLUDED.pwl_break_lt_count,
    total_ytd_days = dbs_history_agg.total_ytd_days + EXCLUDED.total_ytd_days,
    pwh_break_ytd_count = dbs_history_agg.pwh_break_ytd_count + EXCLUDED.pwh_break_ytd_count,
    pwl_break_ytd_count = dbs_history_agg.pwl_break_ytd_count + EXCLUDED.pwl_break_ytd_count;
$$;


WITH Daily_Derived_Context AS (
    -- 1. Derive Context for all days
    SELECT
        trading_date, asset_id, day_type, "open", "high", "low", "close",
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        TRIM(TRAILING ' ' FROM TO_CHAR(trading_date, 'Day')) AS Current_DOW,
        LAG(TRIM(TRAILING ' ' FROM TO_CHAR(trading_date, 'Day')), 1) OVER (PARTITION BY asset_id ORDER BY trading_date) 
            || ' -> ' || TRIM(TRAILING ' ' FROM TO_CHAR(trading_date, 'Day')) AS DOW_Transition
    FROM daily_views
),
Weekly_Lag_Data AS (
    -- 2. Get the Lagged Weekly High/Low (PWH/PWL)
    SELECT 
        asset_id, week_start,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWH,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY week_start) AS PWL
    FROM weekly_views
),
Historical_Break_Flags AS (
    -- 3. Combine context and flag breaks for ALL historical days
    SELECT
        D.trading_date, D.asset_id, D.Prior_Day_Type, D.DOW_Transition,
        CASE WHEN D."high" > W.PWH THEN 1 ELSE 0 END AS PWH_Break,
        CASE WHEN D."low" < W.PWL THEN 1 ELSE 0 END AS PWL_Break,
        -- Time Horizon Markers (Since this is a full load, all markers are TRUE for now)
        TRUE AS Is_ST, TRUE AS Is_LT, 
        (EXTRACT(YEAR FROM D.trading_date) = EXTRACT(YEAR FROM (SELECT MAX(trading_date) FROM daily_views))) AS Is_YTD
    FROM Daily_Derived_Context D
    INNER JOIN weekly_views W_Curr ON W_Curr.asset_id = D.asset_id AND DATE_TRUNC('week', D.trading_date) = W_Curr.week_start
    INNER JOIN Weekly_Lag_Data W ON W.asset_id = W_Curr.asset_id AND W.week_start = DATE_TRUNC('week', D.trading_date)
    WHERE D.Prior_Day_Type IS NOT NULL AND W.PWH IS NOT NULL -- Filter out days without prior context
)
-- 4. Aggregate all history into the final dbs_history_agg table format
INSERT INTO dbs_history_agg (
    asset_id, prior_day_type, dow_transition, total_st_days, pwh_break_st_count, pwl_break_st_count, 
    total_lt_days, pwh_break_lt_count, pwl_break_lt_count, total_ytd_days, pwh_break_ytd_count, pwl_break_ytd_count
)
SELECT
    asset_id, Prior_Day_Type, DOW_Transition,
    SUM(Is_ST::INT) AS Total_ST_Days, SUM(PWH_Break) AS PWH_Break_ST_Count, SUM(PWL_Break) AS PWL_Break_ST_Count,
    SUM(Is_LT::INT) AS Total_LT_Days, SUM(PWH_Break) AS PWH_Break_LT_Count, SUM(PWL_Break) AS PWL_Break_LT_Count,
    SUM(Is_YTD::INT) AS Total_YTD_Days, SUM(PWH_Break) AS PWH_Break_YTD_Count, SUM(PWL_Break) AS PWL_Break_YTD_Count
FROM Historical_Break_Flags
GROUP BY 1, 2, 3
ON CONFLICT DO NOTHING; -- No update needed, this is a full insert