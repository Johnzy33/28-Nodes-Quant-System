-- Metric 8: Session Sequence Stability (SSS) Table - DEBUGGING SCRIPT
-- This script calculates stability metrics for *ALL* PS2 -> PS1 state combinations,
-- not just "Strong" trends, to ensure table population with small datasets.
DROP TABLE  IF EXISTS us2000_trend_stability_score CASCADE;
CREATE TABLE IF NOT EXISTS us2000_trend_stability_score (
    asset_id TEXT NOT NULL,
    trend_direction TEXT NOT NULL,                 -- Bullish or Bearish
    ps2_name TEXT NOT NULL,                        -- Session 2 back
    ps1_name TEXT NOT NULL,                        -- Session 1 back
    ps2_bias_7 TEXT NOT NULL,                      -- 7-State bias of PS2
    ps1_bias_7 TEXT NOT NULL,                      -- 7-State bias of PS1
    total_strong_patterns BIGINT NOT NULL,         -- Denominator D: Total sequences observed
    continuation_count BIGINT NOT NULL,            -- Numerator N: Count of sequences that continued (i.e., resulted in a strong trend)
    tcs_score DOUBLE PRECISION NOT NULL,           -- Continuation_Count / Total_Patterns
    insight_label TEXT NOT NULL,
    PRIMARY KEY (asset_id, trend_direction, ps2_name, ps1_name, ps2_bias_7, ps1_bias_7)
);
TRUNCATE us2000_trend_stability_score;

-- CTE 1: Defines all states considered 'Strong Bullish'
WITH strong_bullish_states AS (
    SELECT UNNEST(ARRAY['Bullish', 'Bullish_Reversal', 'Failed_Bearish', 'Bullish_Consolidation']) AS state
),
-- CTE 2: Defines all states considered 'Strong Bearish'
strong_bearish_states AS (
    SELECT UNNEST(ARRAY['Bearish', 'Bearish_Reversal', 'Failed_Bullish', 'Bearish_Consolidation']) AS state
),
-- CTE 3: Create the triple-session sequence with 7-state biases
sequential_triples_7b AS (
    SELECT
        t2.asset_id,
        t2.session_name AS ps2_name,
        t2.bias_7_state AS ps2_bias_7,
        t1.session_name AS ps1_name,
        t1.bias_7_state AS ps1_bias_7,
        cs.bias_7_state AS cs_bias_7
    FROM us2000_unified_bias_source t2
    JOIN us2000_unified_bias_source t1 ON t2.asset_id = t1.asset_id AND t2.end_ts = t1.start_ts
    JOIN us2000_unified_bias_source cs ON t1.asset_id = cs.asset_id AND t1.end_ts = cs.start_ts
    WHERE t2.asset_id = 'assets:US2000'
),
-- CTE 4: Calculate SSS for ALL Sequences, measuring outcome against a Strong BULLISH result in CS
tcs_bullish_outcome_calculation AS (
    SELECT
        t.asset_id,
        'Bullish' AS trend_direction, -- Labeling the measured *outcome* direction
        t.ps2_name,
        t.ps1_name,
        t.ps2_bias_7,
        t.ps1_bias_7,
        COUNT(*) AS total_strong_patterns, -- Total sequences observed for this PS2/PS1 combination
        SUM(CASE
            -- Continuation is defined as the CS session being a Strong Bullish state
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bullish_states) THEN 1
            ELSE 0
        END) AS continuation_count,
        ROUND((SUM(CASE
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bullish_states) THEN 1
            ELSE 0
        END)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 2) AS tcs_score -- Probability of a Strong Bullish Outcome
    FROM sequential_triples_7b t
    -- No JOIN on strong_bullish_states/strong_bearish_states here! We include ALL PS2/PS1 states.
    GROUP BY 1, 2, 3, 4, 5, 6
),
-- CTE 5: Calculate SSS for ALL Sequences, measuring outcome against a Strong BEARISH result in CS
tcs_bearish_outcome_calculation AS (
    SELECT
        t.asset_id,
        'Bearish' AS trend_direction, -- Labeling the measured *outcome* direction
        t.ps2_name,
        t.ps1_name,
        t.ps2_bias_7,
        t.ps1_bias_7,
        COUNT(*) AS total_strong_patterns, -- Total sequences observed for this PS2/PS1 combination
        SUM(CASE
            -- Continuation is defined as the CS session being a Strong Bearish state
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bearish_states) THEN 1
            ELSE 0
        END) AS continuation_count,
        ROUND((SUM(CASE
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bearish_states) THEN 1
            ELSE 0
        END)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 2) AS tcs_score -- Probability of a Strong Bearish Outcome
    FROM sequential_triples_7b t
    -- No JOIN on strong_bullish_states/strong_bearish_states here! We include ALL PS2/PS1 states.
    GROUP BY 1, 2, 3, 4, 5, 6
)
-- INSERT INTO Table 8 (Combining both Bullish and Bearish results)
INSERT INTO us2000_trend_stability_score (
    asset_id, trend_direction, ps2_name, ps1_name, ps2_bias_7, ps1_bias_7,
    total_strong_patterns, continuation_count, tcs_score, insight_label
)
-- Select Bullish outcome results and calculate insight_label
SELECT
    t.asset_id,
    t.trend_direction,
    t.ps2_name,
    t.ps1_name,
    t.ps2_bias_7,
    t.ps1_bias_7,
    t.total_strong_patterns,
    t.continuation_count,
    t.tcs_score,
    CASE
        WHEN t.tcs_score >= 80.00 THEN 'Extreme Bullish Propensity'
        WHEN t.tcs_score <= 20.00 THEN 'Strong Bearish Propensity'
        ELSE 'Neutral Propensity'
    END AS insight_label
FROM tcs_bullish_outcome_calculation t
WHERE t.total_strong_patterns > 0 -- Return any pattern observed
UNION ALL
-- Select Bearish outcome results and calculate insight_label
SELECT
    t.asset_id,
    t.trend_direction,
    t.ps2_name,
    t.ps1_name,
    t.ps2_bias_7,
    t.ps1_bias_7,
    t.total_strong_patterns,
    t.continuation_count,
    t.tcs_score,
    CASE
        WHEN t.tcs_score >= 80.00 THEN 'Extreme Bearish Propensity'
        WHEN t.tcs_score <= 20.00 THEN 'Strong Bullish Propensity'
        ELSE 'Neutral Propensity'
    END AS insight_label
FROM tcs_bearish_outcome_calculation t
WHERE t.total_strong_patterns > 0; -- Return any pattern observed
-----------------------------------
---s ssss fix
-------------------

-- Metric 8: Session Sequence Stability (SSS) Table - FINAL, CORRECTED LOGIC
-- This script now includes the necessary CTE (unified_bias_source) and the robust
-- sequential linking logic (MIN(start_ts) subquery) to guarantee data population.
CREATE TABLE IF NOT EXISTS us2000_trend_stability_score (
    asset_id TEXT NOT NULL,
    trend_direction TEXT NOT NULL,                 -- Bullish or Bearish (based on outcome)
    ps2_name TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps2_bias_7 TEXT NOT NULL,
    ps1_bias_7 TEXT NOT NULL,
    total_strong_patterns BIGINT NOT NULL,         -- Denominator D: Total sequences observed
    continuation_count BIGINT NOT NULL,            -- Numerator N: Count of sequences that resulted in a strong trend
    tcs_score DOUBLE PRECISION NOT NULL,           -- Continuation_Count / Total_Patterns
    insight_label TEXT NOT NULL,
    PRIMARY KEY (asset_id, trend_direction, ps2_name, ps1_name, ps2_bias_7, ps1_bias_7)
);
TRUNCATE us2000_trend_stability_score;

-- CTE 1: Define the unified source (as used successfully in your 2nd-order query)
WITH unified_bias_source AS (
    SELECT
        trading_date,
        asset_id,
        session_name,
        start_ts,
        end_ts, -- Added end_ts just in case, though start_ts is used for sequencing
        session_type AS bias_3_state,
        -- 7-State Bias (Used for Second-Order Contextual Momentum)
        CASE
            WHEN session_type IN ('Bullish', 'Bearish')
                THEN session_type
            WHEN session_type = 'Consolidation'
                THEN consolidation_subtype  -- Uses one of the five consolidation sub-types
            ELSE 'Other' -- Catches any unknown/NULL states
        END AS bias_7_state
    FROM us2000_session_views
),
-- CTE 2: Defines all states considered 'Strong Bullish'
strong_bullish_states AS (
    SELECT UNNEST(ARRAY['Bullish', 'Bullish_Reversal', 'Failed_Bearish', 'Bullish_Consolidation']) AS state
),
-- CTE 3: Defines all states considered 'Strong Bearish'
strong_bearish_states AS (
    SELECT UNNEST(ARRAY['Bearish', 'Bearish_Reversal', 'Failed_Bullish', 'Bearish_Consolidation']) AS state
),
-- CTE 4: Create the triple-session sequence with 7-state biases (USING ROBUST JOIN LOGIC)
sequential_triples_7b_FIXED AS (
    SELECT
        ps2.asset_id,
        ps2.session_name AS ps2_name,
        ps2.bias_7_state AS ps2_bias_7,
        ps1.session_name AS ps1_name,
        ps1.bias_7_state AS ps1_bias_7,
        cs.session_name AS cs_name,
        cs.bias_7_state AS cs_bias_7
    FROM unified_bias_source ps2 -- Prior Session 2
    JOIN unified_bias_source ps1 -- Prior Session 1
        ON ps2.asset_id = ps1.asset_id
        AND ps2.start_ts < ps1.start_ts
    JOIN unified_bias_source cs -- Current Session
        ON ps1.asset_id = cs.asset_id
        AND ps1.start_ts < cs.start_ts
    -- Sequential Logic Filter (Ensures PS2 is immediately before PS1, and PS1 is immediately before CS)
    WHERE
        ps1.start_ts = (SELECT MIN(t.start_ts) FROM unified_bias_source t WHERE t.asset_id = ps2.asset_id AND t.start_ts > ps2.start_ts)
        AND cs.start_ts = (SELECT MIN(t.start_ts) FROM unified_bias_source t WHERE t.asset_id = ps1.asset_id AND t.start_ts > ps1.start_ts)
        AND ps2.asset_id = 'assets:US2000'
),
-- CTE 5: Calculate SSS for ALL Sequences, measuring outcome against a Strong BULLISH result in CS
tcs_bullish_outcome_calculation AS (
    SELECT
        t.asset_id,
        'Bullish' AS trend_direction, -- Labeling the measured *outcome* direction
        t.ps2_name,
        t.ps1_name,
        t.ps2_bias_7,
        t.ps1_bias_7,
        COUNT(*) AS total_strong_patterns, -- Total sequences observed for this PS2/PS1 combination
        SUM(CASE
            -- Continuation is defined as the CS session being a Strong Bullish state
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bullish_states) THEN 1
            ELSE 0
        END) AS continuation_count,
        ROUND((SUM(CASE
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bullish_states) THEN 1
            ELSE 0
        END)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 2) AS tcs_score -- Probability of a Strong Bullish Outcome
    FROM sequential_triples_7b_FIXED t
    GROUP BY 1, 2, 3, 4, 5, 6
),
-- CTE 6: Calculate SSS for ALL Sequences, measuring outcome against a Strong BEARISH result in CS
tcs_bearish_outcome_calculation AS (
    SELECT
        t.asset_id,
        'Bearish' AS trend_direction, -- Labeling the measured *outcome* direction
        t.ps2_name,
        t.ps1_name,
        t.ps2_bias_7,
        t.ps1_bias_7,
        COUNT(*) AS total_strong_patterns, -- Total sequences observed for this PS2/PS1 combination
        SUM(CASE
            -- Continuation is defined as the CS session being a Strong Bearish state
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bearish_states) THEN 1
            ELSE 0
        END) AS continuation_count,
        ROUND((SUM(CASE
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bearish_states) THEN 1
            ELSE 0
        END)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 2) AS tcs_score -- Probability of a Strong Bearish Outcome
    FROM sequential_triples_7b_FIXED t
    GROUP BY 1, 2, 3, 4, 5, 6
)
-- INSERT INTO Table 8 (Combining both Bullish and Bearish results)
INSERT INTO us2000_trend_stability_score (
    asset_id, trend_direction, ps2_name, ps1_name, ps2_bias_7, ps1_bias_7,
    total_strong_patterns, continuation_count, tcs_score, insight_label
)
-- Select Bullish outcome results and calculate insight_label
SELECT
    t.asset_id,
    t.trend_direction,
    t.ps2_name,
    t.ps1_name,
    t.ps2_bias_7,
    t.ps1_bias_7,
    t.total_strong_patterns,
    t.continuation_count,
    t.tcs_score,
    CASE
        WHEN t.tcs_score >= 80.00 THEN 'Extreme Bullish Propensity'
        WHEN t.tcs_score <= 20.00 THEN 'Strong Bearish Propensity'
        ELSE 'Neutral Propensity'
    END AS insight_label
FROM tcs_bullish_outcome_calculation t
WHERE t.total_strong_patterns > 0
UNION ALL
-- Select Bearish outcome results and calculate insight_label
SELECT
    t.asset_id,
    t.trend_direction,
    t.ps2_name,
    t.ps1_name,
    t.ps2_bias_7,
    t.ps1_bias_7,
    t.total_strong_patterns,
    t.continuation_count,
    t.tcs_score,
    CASE
        WHEN t.tcs_score >= 80.00 THEN 'Extreme Bearish Propensity'
        WHEN t.tcs_score <= 20.00 THEN 'Strong Bullish Propensity'
        ELSE 'Neutral Propensity'
    END AS insight_label
FROM tcs_bearish_outcome_calculation t
WHERE t.total_strong_patterns > 0;
