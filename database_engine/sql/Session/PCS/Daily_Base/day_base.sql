---------------------
-- Base day calculation
-----------------------

DROP TABLE IF EXISTS day_type_base_rates CASCADE;

-- Metric 10: Unconditional Day Type Base Rates (P(Day Type Base Rate))
CREATE TABLE IF NOT EXISTS day_type_base_rates (
    asset_id TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    -- New column to identify the lookback period: '6M', '3Y', or 'ALL'
    lookback_period TEXT NOT NULL, 
    outcome_count BIGINT NOT NULL,
    p_day_type_base DOUBLE PRECISION NOT NULL,  -- The unconditional probability
    -- Primary key must now include the lookback_period
    PRIMARY KEY (asset_id, daily_outcome_7, lookback_period) 
);
TRUNCATE day_type_base_rates;

-- Define the lookback periods and their corresponding date intervals
WITH lookback_intervals AS (
    SELECT '6M' AS lookback_period, CURRENT_DATE - INTERVAL '6 months' AS start_date
    UNION ALL
    SELECT '1Y' AS lookback_period, CURRENT_DATE - INTERVAL '1 years' AS start_date
    UNION ALL
    -- 'ALL' history starts at a very early date
    SELECT 'ALL' AS lookback_period, '1900-01-01'::DATE AS start_date 
),
daily_outcome_7_state AS (
    -- Defines the 7-State Outcome for all relevant days
    SELECT
        trading_date,
        asset_id,
        CASE
            WHEN day_type IN ('Bullish', 'Bearish') THEN day_type
            WHEN day_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other'
        END AS daily_outcome_7
    FROM asset_daily_views
    -- The asset_id filter should remain, but is now general, not hardcoded US2000 in the table name
    WHERE day_type IS NOT NULL AND asset_id = 'assets:US2000' 
),
-- Cartesian product to pair every day with every required lookback period
dated_outcomes AS (
    SELECT
        dos.asset_id,
        dos.daily_outcome_7,
        li.lookback_period,
        dos.trading_date
    FROM daily_outcome_7_state dos
    CROSS JOIN lookback_intervals li
    -- Apply the date filter for the specific lookback period
    WHERE dos.trading_date >= li.start_date
),
total_days_by_lookback AS (
    -- Calculate the denominator (D): Total trading days for each lookback period
    SELECT
        asset_id,
        lookback_period,
        COUNT(DISTINCT trading_date)::NUMERIC AS total_count
    FROM dated_outcomes
    GROUP BY 1, 2
),
base_rate_calculation AS (
    -- Calculate the numerator (N) and final probability (N/D)
    SELECT
        d.asset_id,
        d.daily_outcome_7,
        d.lookback_period,
        COUNT(*)::NUMERIC AS outcome_count,
        -- Join to the total_days_by_lookback for the denominator (D)
        ROUND(
            (COUNT(*)::NUMERIC / NULLIF(td.total_count, 0)) * 100, 
            2
        ) AS p_day_type_base
    FROM dated_outcomes d
    JOIN total_days_by_lookback td
        ON d.asset_id = td.asset_id AND d.lookback_period = td.lookback_period
    GROUP BY 1, 2, 3, td.total_count
)
-- INSERT INTO Base Rate Table
INSERT INTO day_type_base_rates (
    asset_id, daily_outcome_7, lookback_period, outcome_count, p_day_type_base
)
SELECT * FROM base_rate_calculation;

-------------------------------------------
-- 

DROP TABLE IF EXISTS day_type_base_rates CASCADE;

CREATE TABLE IF NOT EXISTS day_type_base_rates (
    asset_id TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    lookback_period TEXT NOT NULL, 
    outcome_count BIGINT NOT NULL,
    p_day_type_base DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (asset_id, daily_outcome_7, lookback_period) 
);

TRUNCATE day_type_base_rates;
-- Define the lookback periods and their corresponding date intervals
WITH lookback_intervals AS (
    SELECT '6M' AS lookback_period, CURRENT_DATE - INTERVAL '6 months' AS start_date
    UNION ALL
    SELECT '1Y' AS lookback_period, CURRENT_DATE - INTERVAL '1 year' AS start_date
    UNION ALL
    SELECT 'ALL' AS lookback_period, '1900-01-01'::DATE AS start_date 
),
daily_outcome_7_state AS (
    -- Defines the 7-State Outcome for all relevant days
    SELECT
        trading_date,
        asset_id,
        CASE
            WHEN day_type IN ('Bullish', 'Bearish') THEN day_type
            WHEN day_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other'
        END AS daily_outcome_7
    FROM asset_daily_views
    WHERE day_type IS NOT NULL 
),
-- Cartesian product to pair every day with every required lookback period
dated_outcomes AS (
    SELECT
        dos.asset_id,
        dos.daily_outcome_7,
        li.lookback_period,
        dos.trading_date
    FROM daily_outcome_7_state dos
    CROSS JOIN lookback_intervals li
    WHERE dos.trading_date >= li.start_date
),
total_days_by_lookback AS (
    -- Calculate the denominator (D): Total trading days for each lookback period
    SELECT
        asset_id,
        lookback_period,
        COUNT(DISTINCT trading_date)::NUMERIC AS total_count
    FROM dated_outcomes
    GROUP BY 1, 2
),
base_rate_calculation AS (
    -- Calculate the numerator (N) and final probability (N/D)
    SELECT
        d.asset_id,
        d.daily_outcome_7,
        d.lookback_period,
        COUNT(*)::NUMERIC AS outcome_count,
        ROUND(
            (COUNT(*)::NUMERIC / NULLIF(td.total_count, 0)) * 100, 
            2
        ) AS p_day_type_base
    FROM dated_outcomes d
    JOIN total_days_by_lookback td
        ON d.asset_id = td.asset_id AND d.lookback_period = td.lookback_period
    GROUP BY 1, 2, 3, td.total_count
)
INSERT INTO day_type_base_rates (
    asset_id, daily_outcome_7, lookback_period, outcome_count, p_day_type_base
)
SELECT * FROM base_rate_calculation;