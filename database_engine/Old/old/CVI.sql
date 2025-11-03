-- Metric 9: Consolidation Volatility Index (CVI) Table
CREATE TABLE IF NOT EXISTS us2000_consolidation_volatility_index (
    asset_id TEXT NOT NULL,
    ps_name TEXT NOT NULL,
    ps_bias_7 TEXT NOT NULL,                       -- Consolidation Subtype
    cs_name TEXT NOT NULL,
    total_consolidation_patterns BIGINT NOT NULL,  -- Denominator D
    takedown_count BIGINT NOT NULL,                -- Numerator N
    cvi_score DOUBLE PRECISION NOT NULL,           -- Takedown_Count / Total_Patterns
    insight_label TEXT NOT NULL,
    PRIMARY KEY (asset_id, ps_name, ps_bias_7, cs_name)
);
TRUNCATE us2000_consolidation_volatility_index;

-- Define all consolidation states
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
consolidation_states AS (
    SELECT UNNEST(ARRAY['Failed_Bullish', 'Failed_Bearish', 'Pure_Indecision', 'Bullish_Reversal', 'Bearish_Reversal']) AS state
),
consolidation_takedowns AS (
    -- Join all session transitions (PS -> CS) where PS is a consolidation subtype
    SELECT
        ps.asset_id,
        ps.session_name AS ps_name,
        ps.bias_7_state AS ps_bias_7,
        cs.session_name AS cs_name,
        cs.trading_date,
        -- Check if the CS session broke PDH or PDL (using Metric 4 Takedown data)
        t.is_pdh_break OR t.is_pdl_break AS did_break_structural_level
    FROM unified_bias_source ps
    JOIN unified_bias_source cs ON ps.asset_id = cs.asset_id AND ps.start_ts < cs.start_ts
    -- Ensure CS is immediately after PS (Sequential logic as in Metric 5-A)
    LEFT JOIN us2000_structural_takedowns t
        ON cs.trading_date = t.trading_date AND cs.session_name = t.session_name
    WHERE ps.bias_7_state IN (SELECT state FROM consolidation_states)
    AND (
            (ps.session_name = 'AS' AND cs.session_name = 'LN' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'LN' AND cs.session_name = 'NYAM' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYAM' AND cs.session_name = 'NYL' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYL' AND cs.session_name = 'NYPM' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYPM' AND cs.session_name = 'AS' AND cs.trading_date = ps.trading_date + INTERVAL '1 day')
    )
)
, cvi_calculation AS (
    SELECT
        c.asset_id,
        c.ps_name,
        c.ps_bias_7,
        c.cs_name,
        COUNT(*) AS total_consolidation_patterns, -- D
        SUM(CASE WHEN c.did_break_structural_level = TRUE THEN 1 ELSE 0 END) AS takedown_count, -- N
        ROUND(SUM(CASE WHEN c.did_break_structural_level = TRUE THEN 1 ELSE 0 END)::NUMERIC / COUNT(*), 4) * 100 AS cvi_score
    FROM consolidation_takedowns c
    GROUP BY 1, 2, 3, 4
)
-- INSERT INTO Table 9
INSERT INTO us2000_consolidation_volatility_index (
    asset_id, ps_name, ps_bias_7, cs_name, total_consolidation_patterns, takedown_count, cvi_score, insight_label
)
SELECT
    c.asset_id,
    c.ps_name,
    c.ps_bias_7,
    c.cs_name,
    c.total_consolidation_patterns,
    c.takedown_count,
    c.cvi_score,
    CASE
        WHEN c.cvi_score >= 70.00 THEN 'Highly Volatile (High Breakout Risk)'
        WHEN c.cvi_score <= 40.00 THEN 'Low Volatility (Consolidation Likely)'
        ELSE 'Neutral Tension'
    END AS insight_label
FROM cvi_calculation c
WHERE c.total_consolidation_patterns > 5;