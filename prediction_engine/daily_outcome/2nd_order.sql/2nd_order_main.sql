-- -----------------------------------------------------------
-- 1. Create the day_type_2nd_order_full Table
-- This table matches the full production pattern key.
-- -----------------------------------------------------------
DROP TABLE day_type_2nd_order;

CREATE TABLE day_type_2nd_order (
    asset_id text NOT NULL,
    -- 2nd Order Pattern Key (3 Sessions, 7-Bias States)
    ps2_name text NOT NULL,
    ps2_bias_7 text NOT NULL,
    ps1_name text NOT NULL,
    ps1_bias_7 text NOT NULL,
    cs_name text NOT NULL,
    cs_bias_7 text NOT NULL,
    -- Outcome
    daily_outcome_7 text NOT NULL, -- The 7-state outcome (e.g., Consolidation-L, Bullish)
    -- Metrics (p_day_type is the probability of the outcome given the pattern)
    total_attempts bigint NOT NULL,
    success_count bigint NOT NULL,
    p_day_type numeric(5, 4) NOT NULL,
    -- The PRIMARY KEY uses all 8 pattern components
    PRIMARY KEY(asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, daily_outcome_7)
);

-- -----------------------------------------------------------
-- 2. SQL to populate/recalculate the day_type_2nd_order_full table
-- Aggregation now groups by the full 7-component pattern key.
-- -----------------------------------------------------------

-- CTE 1: Identify the final 7-state Outcome (daily_outcome_7) for each trading_date

TRUNCATE TABLE day_type_2nd_order;

WITH daily_outcomes AS (
    -- Assuming a dedicated column or view exists for the 7-state outcome.
    -- For now, we'll use 'day_type' from daily_views as a proxy, but production
    -- would require a CASE statement to map to the 7-state outcome.
    SELECT
        trading_date,
        asset_id,
        day_type AS daily_outcome_7 -- Placeholder for the true 7-state outcome column
    FROM
        daily_views 
    WHERE day_type IS NOT NULL
),
-- CTE 2: Join the Session Context (Pattern Keys) with the Day Outcome
context_with_outcome AS (
    SELECT
        sc.asset_id,
        sc.ps2_name,
        sc.ps2_bias_7,
        sc.ps1_name,
        sc.ps1_bias_7,
        sc.cs_name,
        sc.cs_bias_7,
        outcomes.daily_outcome_7
    FROM
        session_context sc
    JOIN
        daily_outcomes outcomes
        -- Join the context (ending session pattern) to the outcome (Day Type)
        ON sc.trading_date = outcomes.trading_date AND sc.asset_id = outcomes.asset_id
    WHERE
        sc.ps2_name IS NOT NULL -- Must have a 2nd preceding session for a 2nd order pattern
),
-- CTE 3: Calculate Counts and Total Denominators
pattern_counts AS (
    SELECT
        asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, daily_outcome_7,
        COUNT(*)::numeric AS success_count, -- Successes = count of this specific pattern+outcome combo
        -- Total attempts is partitioned by the pattern key (excluding the outcome)
        SUM(COUNT(*)) OVER (
            PARTITION BY asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7
        ) AS total_attempts_for_pattern
    FROM
        context_with_outcome
    GROUP BY
        1, 2, 3, 4, 5, 6, 7, 8 -- Group by the full 8-component PK
)
-- Step C: Insert the results into the day_type_2nd_order_full table
INSERT INTO day_type_2nd_order (
    asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, daily_outcome_7, 
    total_attempts, success_count, p_day_type
)
SELECT
    asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, daily_outcome_7,
    total_attempts_for_pattern::bigint,
    success_count::bigint,
    ROUND(success_count / NULLIF(total_attempts_for_pattern, 0), 4) AS p_day_type
FROM
    pattern_counts
WHERE
    total_attempts_for_pattern > 10
ON CONFLICT (asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, daily_outcome_7) DO UPDATE
SET
    total_attempts = EXCLUDED.total_attempts,
    success_count = EXCLUDED.success_count,
    p_day_type = EXCLUDED.p_day_type;
