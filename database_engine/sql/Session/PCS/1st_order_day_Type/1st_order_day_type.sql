------------------------
-- Ist order day type
-----------------------

DROP TABLE IF EXISTS day_type_1st_order CASCADE;

-- Metric 5-A: Conditional Probability (P(Success | Pattern, Tier))
CREATE TABLE IF NOT EXISTS day_type_1st_order (
    asset_id TEXT NOT NULL,
    ps_name TEXT NOT NULL,
    ps_bias_3 TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias_3 TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    -- NEW: The three-tiered day type classification
    day_type_tier TEXT NOT NULL,
    -- NEW: Column to identify the lookback period: '6M', '1Y', or 'ALL'
    lookback_period TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_day_type DOUBLE PRECISION NOT NULL,  -- The conditional probability (success rate)
    -- PRIMARY KEY must now include day_type_tier and lookback_period
    PRIMARY KEY (asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, day_type_tier, lookback_period) 
);

TRUNCATE TABLE day_type_1st_order;

-- 1. Define the lookback periods and their corresponding date intervals (UPDATED: 3Y -> 1Y)
WITH lookback_intervals AS (
    SELECT '6M' AS lookback_period, CURRENT_DATE - INTERVAL '6 months' AS start_date
    UNION ALL
    SELECT '1Y' AS lookback_period, CURRENT_DATE - INTERVAL '1 year' AS start_date 
    UNION ALL
    SELECT 'ALL' AS lookback_period, '1900-01-01'::DATE AS start_date 
),
-- 2. Source CTEs (Your Original Logic - Unchanged)
unified_bias_source AS (
    SELECT trading_date, asset_id, session_name, start_ts, session_type AS bias_3_state,
        CASE WHEN session_type IN ('Bullish', 'Bearish') THEN session_type
             WHEN session_type = 'Consolidation' THEN consolidation_subtype ELSE 'Other' END AS bias_7_state
    FROM asset_session_views
),
daily_outcome_7_state AS (
    SELECT trading_date, asset_id,
        CASE WHEN day_type IN ('Bullish', 'Bearish') THEN day_type
             WHEN day_type = 'Consolidation' THEN consolidation_subtype ELSE 'Other' END AS daily_outcome_7
    FROM asset_daily_views
    WHERE day_type IS NOT NULL 
),
sequential_pairs_3b AS (
    SELECT ps.trading_date, ps.asset_id, ps.session_name AS ps_name, ps.bias_3_state AS ps_bias_3,
           cs.session_name AS cs_name, cs.bias_3_state AS cs_bias_3
    FROM unified_bias_source ps
    JOIN unified_bias_source cs
        ON ps.asset_id = cs.asset_id AND ps.start_ts < cs.start_ts
        AND ( (ps.session_name = 'AS' AND cs.session_name = 'LN' AND ps.trading_date = cs.trading_date) OR
              (ps.session_name = 'LN' AND cs.session_name = 'NYAM' AND ps.trading_date = cs.trading_date) OR
              (ps.session_name = 'NYAM' AND cs.session_name = 'NYL' AND ps.trading_date = cs.trading_date) OR
              (ps.session_name = 'NYL' AND cs.session_name = 'NYPM' AND ps.trading_date = cs.trading_date) OR
              (ps.session_name = 'NYPM' AND cs.session_name = 'AS' AND cs.trading_date = ps.trading_date + INTERVAL '1 day') )
),
-- 3. Data Combined and Tiered (Central CTE for both N and D)
TieredDatedPatterns AS (
    SELECT
        p.asset_id, p.trading_date, p.ps_name, p.ps_bias_3, p.cs_name, p.cs_bias_3,
        d.daily_outcome_7, li.lookback_period,
        BR.p_day_type_base, 
        -- NEW TIER CLASSIFICATION: based on the base rate from Metric 10
        CASE
            WHEN BR.p_day_type_base >= 70.00 THEN 'Tier 1'  
            WHEN BR.p_day_type_base >= 50.00 THEN 'Tier 2'  
            ELSE 'Tier 3'                                  
        END AS day_type_tier -- NEW TIER COLUMN
    FROM sequential_pairs_3b p
    JOIN daily_outcome_7_state d
        ON p.trading_date = d.trading_date AND p.asset_id = d.asset_id
    CROSS JOIN lookback_intervals li
    -- Apply the date filter for the specific lookback period
    LEFT JOIN day_type_base_rates BR -- Join to Base Rate table for the tiering metric
        ON p.asset_id = BR.asset_id 
        AND d.daily_outcome_7 = BR.daily_outcome_7 
        AND li.lookback_period = BR.lookback_period
    WHERE p.trading_date >= li.start_date
),
-- 4. Numerator (N): Success Count for each new Tier/Lookback combination
day_type_1st_order_successes AS (
    SELECT
        asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, day_type_tier, lookback_period,
        COUNT(*) AS success_count -- Numerator N
    FROM TieredDatedPatterns
    GROUP BY 1, 2, 3, 4, 5, 6, 7, 8
),
-- 5. Denominator (D): Total Attempts for each new Tier/Lookback combination
day_type_1st_order_D AS (
    SELECT
        asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, day_type_tier, lookback_period,
        COUNT(*) AS total_attempts -- Denominator D
    FROM TieredDatedPatterns
    GROUP BY 1, 2, 3, 4, 5, 6, 7
)
-- 6. Final INSERT (Joining N and D)
INSERT INTO day_type_1st_order (
    asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, day_type_tier, lookback_period, success_count, total_attempts, p_day_type
)
SELECT
    s.asset_id, s.ps_name, s.ps_bias_3, s.cs_name, s.cs_bias_3, s.daily_outcome_7, 
    s.day_type_tier, s.lookback_period, 
    s.success_count,
    d.total_attempts,
    ROUND((s.success_count::NUMERIC / NULLIF(d.total_attempts, 0)) * 100, 2) AS p_day_type
FROM day_type_1st_order_successes s
JOIN day_type_1st_order_D d
    ON s.asset_id = d.asset_id 
    AND s.ps_name = d.ps_name
    AND s.ps_bias_3 = d.ps_bias_3 
    AND s.cs_name = d.cs_name
    AND s.cs_bias_3 = d.cs_bias_3
    AND s.day_type_tier = d.day_type_tier -- NEW JOIN KEY
    AND s.lookback_period = d.lookback_period -- NEW JOIN KEY
WHERE d.total_attempts > 0
ON CONFLICT (asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, day_type_tier, lookback_period) 
DO UPDATE SET
    success_count = EXCLUDED.success_count,
    total_attempts = EXCLUDED.total_attempts,
    p_day_type = EXCLUDED.p_day_type;
    
----------------------------------

DROP TABLE IF EXISTS day_type_1st_order CASCADE;

DROP TABLE IF EXISTS day_type_1st_order CASCADE;

CREATE TABLE IF NOT EXISTS day_type_1st_order (
    ps_cs_pattern_fk BIGINT NOT NULL,       -- Pattern Type ID
    asset_id TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    day_type_tier TEXT NOT NULL,            -- Tier based on Base Rate
    lookback_period TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_day_type DOUBLE PRECISION NOT NULL,
    
    PRIMARY KEY (ps_cs_pattern_fk, daily_outcome_7, day_type_tier, lookback_period) 
);

TRUNCATE TABLE day_type_1st_order;

WITH lookback_intervals AS (
    SELECT '6M' AS lookback_period, CURRENT_DATE - INTERVAL '6 months' AS start_date
    UNION ALL
    SELECT '1Y' AS lookback_period, CURRENT_DATE - INTERVAL '1 year' AS start_date 
    UNION ALL
    SELECT 'ALL' AS lookback_period, '1900-01-01'::DATE AS start_date 
),
-- 1. Get the Pattern Keys (ps_cs_pattern_fk) and unique session records
session_context_data AS (
    SELECT 
        session_record_pk, 
        ps_cs_pattern_fk, 
        trading_date, 
        asset_id
    FROM asset_session_context
    WHERE ps_cs_pattern_fk IS NOT NULL 
),
-- 2. Get the Daily Outcome (daily_outcome_7)
daily_outcome_7_state AS (
    SELECT trading_date, asset_id,
        CASE 
            WHEN day_type IN ('Bullish', 'Bearish') THEN day_type
            WHEN day_type = 'Consolidation' THEN consolidation_subtype 
            ELSE 'Other' 
        END AS daily_outcome_7
    FROM asset_daily_views
    WHERE day_type IS NOT NULL 
),
-- 3. Combine Pattern Key, Outcome, and Lookback Filters (The Historical Data Set)
DatedPatterns AS (
    SELECT
        sc.session_record_pk, -- The unique PK is preserved here
        sc.ps_cs_pattern_fk, sc.asset_id, sc.trading_date, dos.daily_outcome_7, li.lookback_period
    FROM session_context_data sc
    JOIN daily_outcome_7_state dos
        ON sc.trading_date = dos.trading_date AND sc.asset_id = dos.asset_id
    CROSS JOIN lookback_intervals li
    WHERE sc.trading_date >= li.start_date
),
-- 4. Tiered Classification
TieredDatedPatterns AS (
    SELECT
        DP.*, BR.p_day_type_base, 
        CASE
            WHEN BR.p_day_type_base >= 70.00 THEN 'Tier 1'  
            WHEN BR.p_day_type_base >= 50.00 THEN 'Tier 2'  
            ELSE 'Tier 3'                                  
        END AS day_type_tier
    FROM DatedPatterns DP
    JOIN day_type_base_rates BR 
        ON DP.asset_id = BR.asset_id 
        AND DP.daily_outcome_7 = BR.daily_outcome_7 
        AND DP.lookback_period = BR.lookback_period
),
-- 5. Numerator (N): Success Count 
day_type_1st_order_successes AS (
    SELECT
        ps_cs_pattern_fk, asset_id, daily_outcome_7, day_type_tier, lookback_period,
        -- COUNTING THE UNIQUE PRIMARY KEY OF THE SESSION RECORD
        COUNT(DISTINCT session_record_pk) AS success_count 
    FROM TieredDatedPatterns
    GROUP BY 1, 2, 3, 4, 5 
),
-- 6. Denominator (D): Total Attempts 
day_type_1st_order_D AS (
    SELECT
        ps_cs_pattern_fk, asset_id, day_type_tier, lookback_period,
        -- COUNTING THE UNIQUE PRIMARY KEY OF THE SESSION RECORD
        COUNT(DISTINCT session_record_pk) AS total_attempts 
    FROM TieredDatedPatterns
    GROUP BY 1, 2, 3, 4
)
-- 7. Final INSERT (The remainder of the logic is correct)
INSERT INTO day_type_1st_order (
    ps_cs_pattern_fk, asset_id, daily_outcome_7, day_type_tier, lookback_period, success_count, total_attempts, p_day_type
)
SELECT
    s.ps_cs_pattern_fk, s.asset_id, s.daily_outcome_7, s.day_type_tier, s.lookback_period, 
    s.success_count, d.total_attempts,
    ROUND((s.success_count::NUMERIC / NULLIF(d.total_attempts, 0)) * 100, 2) AS p_day_type
FROM day_type_1st_order_successes s
JOIN day_type_1st_order_D d
    ON s.ps_cs_pattern_fk = d.ps_cs_pattern_fk
    AND s.asset_id = d.asset_id
    AND s.day_type_tier = d.day_type_tier
    AND s.lookback_period = d.lookback_period
WHERE d.total_attempts > 0
ON CONFLICT (ps_cs_pattern_fk, daily_outcome_7, day_type_tier, lookback_period) 
DO UPDATE SET
    success_count = EXCLUDED.success_count, total_attempts = EXCLUDED.total_attempts, p_day_type = EXCLUDED.p_day_type;