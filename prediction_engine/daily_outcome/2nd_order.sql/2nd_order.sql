-- -----------------------------------------------------------
-- 1. Create the day_type_2nd_order Table 
-- Primary Key includes asset_id for per-asset probabilities.
-- -----------------------------------------------------------

CREATE TABLE day_type_2nd_order (
    -- Primary Key: Asset + 2nd Order Pattern Key + Outcome
    asset_id text NOT NULL,
    ps2_bias_7 text NOT NULL,
    ps1_bias_7 text NOT NULL,
    day_type text NOT NULL, -- The outcome day type (e.g., Bullish Day)
    -- Aggregation Metrics
    count_of_occurrence integer NOT NULL,
    probability numeric(5, 4) NOT NULL,

    PRIMARY KEY(asset_id, ps2_bias_7, ps1_bias_7, day_type)
);


-- -----------------------------------------------------------
-- 2. SQL to populate/recalculate the day_type_2nd_order table
-- Safe alias 'outcomes' is used instead of 'do'.
-- -----------------------------------------------------------

-- CTE 1: Identify the Day Type for each trading_date
WITH daily_outcomes AS (
    -- Use the daily_views table for the final day outcome
    SELECT
        trading_date,
        asset_id,
        day_type 
    FROM
        daily_views 
    WHERE day_type IS NOT NULL
),
-- CTE 2: Join the Session Context with the Day Outcome
context_with_outcome AS (
    SELECT
        sc.trading_date,
        sc.asset_id,
        sc.ps2_bias_7,
        sc.ps1_bias_7,
        outcomes.day_type -- FIXED: using 'outcomes' alias
    FROM
        session_context sc
    JOIN
        daily_outcomes outcomes -- FIXED: changed alias from 'do' to 'outcomes'
        -- Join the context (ending session pattern) to the outcome (Day Type)
        ON sc.trading_date = outcomes.trading_date AND sc.asset_id = outcomes.asset_id
    WHERE
        sc.ps2_bias_7 IS NOT NULL -- Exclude records where a 2nd order pattern cannot be formed
),
-- CTE 3: Calculate Counts and Total Denominators
pattern_counts AS (
    SELECT
        asset_id,
        ps2_bias_7,
        ps1_bias_7,
        day_type,
        COUNT(*)::numeric AS count_of_occurrence,
        -- Total attempts is partitioned (grouped) by asset and pattern key
        SUM(COUNT(*)) OVER (PARTITION BY asset_id, ps2_bias_7, ps1_bias_7) AS total_count_for_pattern
    FROM
        context_with_outcome
    GROUP BY
        1, 2, 3, 4 -- Group by asset, pattern, and outcome
)
-- Step C: Insert the results into the day_type_2nd_order table
INSERT INTO day_type_2nd_order (
    asset_id, ps2_bias_7, ps1_bias_7, day_type, count_of_occurrence, probability
)
SELECT
    asset_id,
    ps2_bias_7,
    ps1_bias_7,
    day_type,
    count_of_occurrence::integer,
    ROUND(count_of_occurrence / NULLIF(total_count_for_pattern, 0), 4) AS probability
FROM
    pattern_counts
WHERE
    total_count_for_pattern > 0
ON CONFLICT (asset_id, ps2_bias_7, ps1_bias_7, day_type) DO UPDATE
SET
    count_of_occurrence = EXCLUDED.count_of_occurrence,
    probability = EXCLUDED.probability;
