-- Metric 8: Trend Continuation Stability (TCS) Table

----------------------------------
---- TCS sequencing fix
-----------------------------------

-- Metric 8: Trend Continuation Stability (TCS) - FINAL LOGIC
-- This script uses the strict TCS definition: it only counts sequences where PS2 and PS1
-- are BOTH Strong Bullish/Bearish states. It uses the robust sequential linking logic.
DROP TABLE us2000_trend_stability_score;

CREATE TABLE IF NOT EXISTS us2000_trend_stability_score (
    asset_id TEXT NOT NULL,
    trend_direction TEXT NOT NULL,                 -- Bullish or Bearish (based on the trend being measured)
    ps2_name TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps2_bias_7 TEXT NOT NULL,
    ps1_bias_7 TEXT NOT NULL,
    total_strong_patterns BIGINT NOT NULL,         -- Denominator D: Total sequences where PS2 and PS1 were "Strong"
    continuation_count BIGINT NOT NULL,            -- Numerator N: Count of sequences that continued "Strong"
    tcs_score DOUBLE PRECISION NOT NULL,           -- Continuation_Count / Total_Patterns
    insight_label TEXT NOT NULL,
    PRIMARY KEY (asset_id, trend_direction, ps2_name, ps1_name, ps2_bias_7, ps1_bias_7)
);
TRUNCATE us2000_trend_stability_score;

-- CTE 1: Define the unified source
WITH unified_bias_source AS (
    SELECT
        trading_date,
        asset_id,
        session_name,
        start_ts,
        session_type AS bias_3_state,
        -- 7-State Bias
        CASE
            WHEN session_type IN ('Bullish', 'Bearish')
                THEN session_type
            WHEN session_type = 'Consolidation'
                THEN consolidation_subtype
            ELSE 'Other'
        END AS bias_7_state
    FROM asset_session_views
),
-- CTE 2: Defines all states considered 'Strong Bullish'
strong_bullish_states AS (
    SELECT UNNEST(ARRAY['Bullish', 'Bullish_Reversal', 'Failed_Bearish', 'Bullish_Consolidation']) AS state
),
-- CTE 3: Defines all states considered 'Strong Bearish'
strong_bearish_states AS (
    SELECT UNNEST(ARRAY['Bearish', 'Bearish_Reversal', 'Failed_Bullish', 'Bearish_Consolidation']) AS state
),
-- CTE 4: Create the triple-session sequence (using robust join logic)
sequential_triples_7b AS (
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
-- CTE 5: Calculate TCS for Strong BULLISH Sequences (PS2 Strong Bullish -> PS1 Strong Bullish)
tcs_bullish_calculation AS (
    SELECT
        t.asset_id,
        'Bullish' AS trend_direction,
        t.ps2_name,
        t.ps1_name,
        t.ps2_bias_7,
        t.ps1_bias_7,
        COUNT(*) AS total_strong_patterns, -- Denominator: Total Bullish-Bullish sequences observed
        SUM(CASE
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bullish_states) THEN 1
            ELSE 0
        END) AS continuation_count,
        ROUND((SUM(CASE
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bullish_states) THEN 1
            ELSE 0
        END)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 2) AS tcs_score
    FROM sequential_triples_7b t
    -- TCS CONDITION: PS2 and PS1 MUST be in a strong Bullish state
    JOIN strong_bullish_states s2 ON t.ps2_bias_7 = s2.state
    JOIN strong_bullish_states s1 ON t.ps1_bias_7 = s1.state
    GROUP BY 1, 2, 3, 4, 5, 6
),
-- CTE 6: Calculate TCS for Strong BEARISH Sequences (PS2 Strong Bearish -> PS1 Strong Bearish)
tcs_bearish_calculation AS (
    SELECT
        t.asset_id,
        'Bearish' AS trend_direction,
        t.ps2_name,
        t.ps1_name,
        t.ps2_bias_7,
        t.ps1_bias_7,
        COUNT(*) AS total_strong_patterns, -- Denominator: Total Bearish-Bearish sequences observed
        SUM(CASE
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bearish_states) THEN 1
            ELSE 0
        END) AS continuation_count,
        ROUND((SUM(CASE
            WHEN t.cs_bias_7 IN (SELECT state FROM strong_bearish_states) THEN 1
            ELSE 0
        END)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 2) AS tcs_score
    FROM sequential_triples_7b t
    -- TCS CONDITION: PS2 and PS1 MUST be in a strong Bearish state
    JOIN strong_bearish_states s2 ON t.ps2_bias_7 = s2.state
    JOIN strong_bearish_states s1 ON t.ps1_bias_7 = s1.state
    GROUP BY 1, 2, 3, 4, 5, 6
)
-- INSERT INTO Table 8 (Combining both Bullish and Bearish results)
INSERT INTO us2000_trend_stability_score (
    asset_id, trend_direction, ps2_name, ps1_name, ps2_bias_7, ps1_bias_7,
    total_strong_patterns, continuation_count, tcs_score, insight_label
)
-- Select Bullish results and calculate insight_label
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
        WHEN t.tcs_score >= 80.00 THEN 'Extreme Stability'
        WHEN t.tcs_score <= 50.00 THEN 'Fragile Trend/Breakdown Risk'
        ELSE 'Stable Continuation'
    END AS insight_label
FROM tcs_bullish_calculation t
WHERE t.total_strong_patterns > 10
UNION ALL
-- Select Bearish results and calculate insight_label
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
        WHEN t.tcs_score >= 80.00 THEN 'Extreme Stability'
        WHEN t.tcs_score <= 50.00 THEN 'Fragile Trend/Breakdown Risk'
        ELSE 'Stable Continuation'
    END AS insight_label
FROM tcs_bearish_calculation t
WHERE t.total_strong_patterns > 10;
