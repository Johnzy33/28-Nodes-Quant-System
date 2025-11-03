----------------------------------
-- Consolidation Volatility Index (CVI) Calculation
----------------------------------

DROP TABLE IF EXISTS asset_consolidation_volatility_index CASCADE;
-- Metric 9: Consolidation Volatility Index (CVI) Table
CREATE TABLE IF NOT EXISTS asset_consolidation_volatility_index (
    asset_id TEXT NOT NULL,
    cs_ps1_fk BIGINT NOT NULL,                     -- Foreign Key linking PS bias and CS bias (7-Bias)
    ps1_name TEXT NOT NULL,
    ps1_bias_7 TEXT NOT NULL,                      -- Consolidation Subtype
    cs_name TEXT NOT NULL,
    total_consolidation_patterns BIGINT NOT NULL,  -- Denominator D
    takedown_count BIGINT NOT NULL,                -- Numerator N
    cvi_score DOUBLE PRECISION NOT NULL,           -- Takedown_Count / Total_Patterns
    insight_label TEXT NOT NULL,
    takedown_dates DATE[],
    PRIMARY KEY (cs_ps1_fk)                        -- Simplified PK using FK
);
TRUNCATE asset_consolidation_volatility_index;

-- =================================================================================
-- METRIC 9: CONSOLIDATION VOLATILITY INDEX (CVI) - GENERALIZED WITH TAKEDOWN DATES
-- =================================================================================
WITH consolidation_states AS (
    -- 1. Define all states considered Consolidation Subtypes (7-state)
    SELECT UNNEST(ARRAY['Failed_Bullish', 'Failed_Bearish', 'Pure_Indecision', 'Bullish_Reversal', 'Bearish_Reversal', 'Bullish_Consolidation', 'Bearish_Consolidation']) AS state
),
consolidation_patterns AS (
    -- 2. Identify all sequential patterns (PS1 -> CS) where PS1 is a consolidation state
    SELECT
        ctx.asset_id,
        ctx.trading_date,
        ctx.cs_ps1_fk,
        ctx.ps1_name,
        ctx.ps1_bias_7_state AS ps1_bias_7,
        ctx.cs_name,
        -- Join to the Structural Takedowns table (Metric 4 output)
        t.is_pdh_break,
        t.is_pdl_break,
        -- Define the Takedown Flag (1 if a structural break occurred, 0 otherwise)
        (t.is_pdh_break = TRUE OR t.is_pdl_break = TRUE) AS is_takedown
    FROM asset_session_context ctx
    LEFT JOIN asset_structural_takedowns t
        ON ctx.asset_id = t.asset_id
        AND ctx.trading_date = t.trading_date
        AND ctx.cs_name = t.session_name
    WHERE ctx.ps1_bias_7_state IN (SELECT state FROM consolidation_states)
),
cvi_calculation AS (
    -- 3. Aggregate patterns, count takedowns (N), and collect takedown dates
    SELECT
        c.asset_id,
        c.cs_ps1_fk,
        c.ps1_name,
        c.ps1_bias_7,
        c.cs_name,
        COUNT(*) AS total_consolidation_patterns, -- D
        SUM(CASE WHEN c.is_takedown = TRUE THEN 1 ELSE 0 END) AS takedown_count, -- N
        -- Collect the dates where is_takedown = TRUE
        ARRAY_AGG(c.trading_date ORDER BY c.trading_date) 
            FILTER (WHERE c.is_takedown = TRUE) AS takedown_dates
            
    FROM consolidation_patterns c
    GROUP BY 1, 2, 3, 4, 5
)
-- 4. FINAL INSERT: Calculate CVI Score and Insight Label
INSERT INTO asset_consolidation_volatility_index (
    asset_id, cs_ps1_fk, ps1_name, ps1_bias_7, cs_name, 
    total_consolidation_patterns, takedown_count, cvi_score, takedown_dates, insight_label
)
SELECT
    c.asset_id,
    c.cs_ps1_fk,
    c.ps1_name,
    c.ps1_bias_7,
    c.cs_name,
    c.total_consolidation_patterns,
    c.takedown_count,
    ROUND((c.takedown_count::NUMERIC / NULLIF(c.total_consolidation_patterns, 0)) * 100, 2) AS cvi_score,
    c.takedown_dates, -- The new date array
    CASE
        WHEN ROUND((c.takedown_count::NUMERIC / NULLIF(c.total_consolidation_patterns, 0)) * 100, 2) >= 70.00 THEN 'Highly Volatile (High Breakout Risk)'
        WHEN ROUND((c.takedown_count::NUMERIC / NULLIF(c.total_consolidation_patterns, 0)) * 100, 2) <= 40.00 THEN 'Low Volatility (Consolidation Likely)'
        ELSE 'Neutral Tension'
    END AS insight_label
FROM cvi_calculation c
WHERE c.total_consolidation_patterns > 5;