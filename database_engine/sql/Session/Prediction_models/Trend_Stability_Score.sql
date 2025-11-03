------------------------------
-- Trend Stability Score Calculation
------------------------------

DROP TABLE IF EXISTS asset_trend_stability_score CASCADE;
-- Metric 8: Trend Stability Score (TCS) Table
CREATE TABLE IF NOT EXISTS asset_trend_stability_score (
    asset_id TEXT NOT NULL,
    trend_direction TEXT NOT NULL,
    ps2_name TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps2_bias_7 TEXT NOT NULL,
    ps1_bias_7 TEXT NOT NULL,
    total_strong_patterns BIGINT NOT NULL,
    continuation_count BIGINT NOT NULL,
    tcs_score DOUBLE PRECISION NOT NULL,             -- NEW: Array of dates where the trend successfully continued
    insight_label TEXT NOT NULL,
    continuation_dates DATE[] NOT NULL,
    PRIMARY KEY (asset_id, trend_direction, ps2_name, ps1_name, ps2_bias_7, ps1_bias_7)
);
TRUNCATE asset_trend_stability_score;

-- =================================================================================
-- METRIC 8: TREND STABILITY SCORE (TCS) - GENERALIZED
-- =================================================================================
WITH session_sequences AS (
    -- 1. Define the unified 7-state bias source and identify sequential sessions
    SELECT
        trading_date,
        asset_id,
        session_name,
        start_ts,
        -- Define 7-State Bias
        CASE
            WHEN session_type IN ('Bullish', 'Bearish') THEN session_type
            WHEN session_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other'
        END AS cs_bias_7,
        -- Use LAG() to get the bias and name of the two prior sessions (PS1 and PS2)
        LAG(
            CASE
                WHEN session_type IN ('Bullish', 'Bearish') THEN session_type
                WHEN session_type = 'Consolidation' THEN consolidation_subtype
                ELSE 'Other'
            END, 1
        ) OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS ps1_bias_7,
        LAG(session_name, 1) OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS ps1_name,
        
        LAG(
            CASE
                WHEN session_type IN ('Bullish', 'Bearish') THEN session_type
                WHEN session_type = 'Consolidation' THEN consolidation_subtype
                ELSE 'Other'
            END, 2
        ) OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS ps2_bias_7,
        LAG(session_name, 2) OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS ps2_name
    FROM asset_session_views -- <<< GENERALIZED VIEW
    WHERE session_type IS NOT NULL
),
strong_bullish_states AS (
    -- 2. Define all states considered 'Strong Bullish'
    SELECT UNNEST(ARRAY['Bullish', 'Bullish_Reversal', 'Failed_Bearish', 'Bullish_Consolidation']) AS state
),
strong_bearish_states AS (
    -- 3. Define all states considered 'Strong Bearish'
    SELECT UNNEST(ARRAY['Bearish', 'Bearish_Reversal', 'Failed_Bullish', 'Bearish_Consolidation']) AS state
),
tcs_all_patterns AS (
    -- 4. Classify all relevant PS2->PS1->CS sequences
    SELECT
        s.asset_id,
        s.trading_date,
        s.cs_name,
        s.cs_bias_7,
        s.ps1_name,
        s.ps1_bias_7,
        s.ps2_name,
        s.ps2_bias_7,
        -- BULLISH TCS: PS2 and PS1 are Strong Bullish, and CS continues the pattern
        CASE 
            WHEN s.ps2_bias_7 IN (SELECT state FROM strong_bullish_states) 
             AND s.ps1_bias_7 IN (SELECT state FROM strong_bullish_states)
            THEN 'Bullish'
            ELSE NULL
        END AS bullish_pattern_match,
        -- BEARISH TCS: PS2 and PS1 are Strong Bearish, and CS continues the pattern
        CASE 
            WHEN s.ps2_bias_7 IN (SELECT state FROM strong_bearish_states) 
             AND s.ps1_bias_7 IN (SELECT state FROM strong_bearish_states)
            THEN 'Bearish'
            ELSE NULL
        END AS bearish_pattern_match,
        -- Continuation Flags
        CASE WHEN s.cs_bias_7 IN (SELECT state FROM strong_bullish_states) THEN 1 ELSE 0 END AS is_bullish_continuation,
        CASE WHEN s.cs_bias_7 IN (SELECT state FROM strong_bearish_states) THEN 1 ELSE 0 END AS is_bearish_continuation
        
    FROM session_sequences s
    WHERE 
        -- Must have a full triple sequence (PS2, PS1, CS)
        s.ps2_name IS NOT NULL 
        AND s.ps1_name IS NOT NULL
)
-- 5. Final Aggregation and INSERT
INSERT INTO asset_trend_stability_score (
    asset_id, trend_direction, ps2_name, ps1_name, ps2_bias_7, ps1_bias_7,
    total_strong_patterns, continuation_count, tcs_score, continuation_dates, insight_label
)
-- Aggregation for BULLISH TCS
SELECT
    t.asset_id,
    'Bullish' AS trend_direction,
    t.ps2_name,
    t.ps1_name,
    t.ps2_bias_7,
    t.ps1_bias_7,
    COUNT(*) AS total_strong_patterns, -- D
    SUM(t.is_bullish_continuation) AS continuation_count, -- N
    ROUND((SUM(t.is_bullish_continuation)::NUMERIC / NULLIF(COUNT(*), 0) * 100), 2) AS tcs_score,
    -- Array of dates where continuation occurred
    ARRAY_AGG(t.trading_date ORDER BY t.trading_date) 
        FILTER (WHERE t.is_bullish_continuation = 1) AS continuation_dates, 
    
    CASE
        WHEN ROUND((SUM(t.is_bullish_continuation)::NUMERIC / NULLIF(COUNT(*), 0) * 100), 2) >= 80.00 THEN 'Extreme Stability'
        WHEN ROUND((SUM(t.is_bullish_continuation)::NUMERIC / NULLIF(COUNT(*), 0) * 100), 2) <= 50.00 THEN 'Fragile Trend/Breakdown Risk'
        ELSE 'Stable Continuation'
    END AS insight_label
FROM tcs_all_patterns t
WHERE t.bullish_pattern_match IS NOT NULL
GROUP BY 1, 2, 3, 4, 5, 6
HAVING COUNT(*) > 0
UNION ALL
-- Aggregation for BEARISH TCS
SELECT
    t.asset_id,
    'Bearish' AS trend_direction,
    t.ps2_name,
    t.ps1_name,
    t.ps2_bias_7,
    t.ps1_bias_7,
    COUNT(*) AS total_strong_patterns, -- D
    SUM(t.is_bearish_continuation) AS continuation_count, -- N
    ROUND((SUM(t.is_bearish_continuation)::NUMERIC / NULLIF(COUNT(*), 0) * 100), 2) AS tcs_score,
    -- Array of dates where continuation occurred
    ARRAY_AGG(t.trading_date ORDER BY t.trading_date) 
        FILTER (WHERE t.is_bearish_continuation = 1) AS continuation_dates, 
        
    CASE
        WHEN ROUND((SUM(t.is_bearish_continuation)::NUMERIC / NULLIF(COUNT(*), 0) * 100), 2) >= 80.00 THEN 'Extreme Stability'
        WHEN ROUND((SUM(t.is_bearish_continuation)::NUMERIC / NULLIF(COUNT(*), 0) * 100), 2) <= 50.00 THEN 'Fragile Trend/Breakdown Risk'
        ELSE 'Stable Continuation'
    END AS insight_label
FROM tcs_all_patterns t
WHERE t.bearish_pattern_match IS NOT NULL
GROUP BY 1, 2, 3, 4, 5, 6
HAVING COUNT(*) > 0;
