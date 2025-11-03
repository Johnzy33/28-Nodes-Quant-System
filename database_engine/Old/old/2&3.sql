-- Table Creation

CREATE TABLE IF NOT EXISTS us2000_transition_1st_order_3b (
    ps_name TEXT NOT NULL,
    ps_bias TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_transition DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (ps_name, ps_bias, cs_name, cs_bias)
);
TRUNCATE us2000_transition_1st_order_3b;

-- =================================================================================
-- FOUNDATION: UNIFIED BIAS SOURCE (S1)
-- Creates both the 3-State and 7-State bias columns.
-- =================================================================================
WITH unified_bias_source AS (
    SELECT
        trading_date,
        asset_id,
        session_name,
        start_ts,
                -- 3-State Bias (Used for First-Order and Optional Second-Order)
        session_type AS bias_3_state,        
        -- 7-State Bias (Used for Second-Order Contextual Momentum)
        CASE 
            WHEN session_type IN ('Bullish', 'Bearish') 
                THEN session_type
            WHEN session_type = 'Consolidation' 
                THEN consolidation_subtype  -- Uses one of the five consolidation sub-types
            ELSE 'Other' -- Catches any unknown/NULL states
        END AS bias_7_state
        
    FROM asset_session_views
),
transition_1st_order AS (
    -- Generates all sequential pairs (PS1 -> CS)
    SELECT
        ps.session_name AS ps1_name,
        ps.bias_3_state AS ps1_bias,
        cs.session_name AS cs_name,
        cs.bias_3_state AS cs_bias
    FROM unified_bias_source ps -- Prior Session (PS1)
    JOIN unified_bias_source cs -- Current Session (CS)
        ON ps.asset_id = cs.asset_id
        AND ps.start_ts < cs.start_ts
        -- Sequential Session Logic (Handles intra-day and cross-day)
        AND (
            (ps.session_name = 'AS' AND cs.session_name = 'LN' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'LN' AND cs.session_name = 'NYAM' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYAM' AND cs.session_name = 'NYL' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYL' AND cs.session_name = 'NYPM' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYPM' AND cs.session_name = 'AS' AND cs.trading_date = ps.trading_date + INTERVAL '1 day') -- FIX: Cross-day transition
        )
    WHERE ps.bias_3_state IS NOT NULL AND cs.bias_3_state IS NOT NULL
),
transition_1st_order_D AS (
    -- Denominator (Total times PS1_Bias occurred)
    SELECT ps1_name, ps1_bias, COUNT(*) AS total_attempts
    FROM transition_1st_order
    GROUP BY 1, 2
)
-- Final Insert for Table 1
INSERT INTO us2000_transition_1st_order_3b (
    ps_name, ps_bias, cs_name, cs_bias, total_attempts, success_count, p_transition
)
SELECT
    t1.ps1_name,
    t1.ps1_bias,
    t1.cs_name,
    t1.cs_bias,
    td.total_attempts,
    COUNT(*) AS success_count, -- Numerator
    ROUND((COUNT(*)::NUMERIC / td.total_attempts) * 100, 2) AS p_transition
FROM transition_1st_order t1
JOIN transition_1st_order_D td
    ON t1.ps1_name = td.ps1_name AND t1.ps1_bias = td.ps1_bias
GROUP BY 1, 2, 3, 4, td.total_attempts;

-------------------------------------------
---2 3rd order
--------------------------------------

-- Table Creation
CREATE TABLE IF NOT EXISTS us2000_transition_2nd_order_7b (
    ps2_name TEXT NOT NULL,
    ps2_bias TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps1_bias TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_transition DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias)
);
TRUNCATE us2000_transition_2nd_order_7b;

-- =================================================================================
-- FOUNDATION: UNIFIED BIAS SOURCE (S1)
-- Creates both the 3-State and 7-State bias columns.
-- =================================================================================
WITH unified_bias_source AS (
    SELECT
        trading_date,
        asset_id,
        session_name,
        start_ts,                -- 3-State Bias (Used for First-Order and Optional Second-Order)
        session_type AS bias_3_state,        
        -- 7-State Bias (Used for Second-Order Contextual Momentum)
        CASE 
            WHEN session_type IN ('Bullish', 'Bearish') 
                THEN session_type
            WHEN session_type = 'Consolidation' 
                THEN consolidation_subtype  -- Uses one of the five consolidation sub-types
            ELSE 'Other' -- Catches any unknown/NULL states
        END AS bias_7_state
        
    FROM asset_session_views
),
transition_2nd_order AS (
    -- Generates all sequential triples (PS2 -> PS1 -> CS)
    SELECT
        ps2.session_name AS ps2_name,
        ps2.bias_7_state AS ps2_bias,
        ps1.session_name AS ps1_name,
        ps1.bias_7_state AS ps1_bias,
        cs.session_name AS cs_name,
        cs.bias_7_state AS cs_bias
    FROM unified_bias_source ps2 -- Prior Session 2
    JOIN unified_bias_source ps1 -- Prior Session 1 (PS2 -> PS1)
        ON ps2.asset_id = ps1.asset_id 
        AND ps2.start_ts < ps1.start_ts
    JOIN unified_bias_source cs  -- Current Session (PS1 -> CS)
        ON ps1.asset_id = cs.asset_id
        AND ps1.start_ts < cs.start_ts
    -- Sequential Logic Filter (Ensures PS2 is immediately before PS1, and PS1 is immediately before CS)
    WHERE 
        ps1.start_ts = (SELECT MIN(t.start_ts) FROM unified_bias_source t WHERE t.asset_id = ps2.asset_id AND t.start_ts > ps2.start_ts)
        AND cs.start_ts = (SELECT MIN(t.start_ts) FROM unified_bias_source t WHERE t.asset_id = ps1.asset_id AND t.start_ts > ps1.start_ts)
        -- Ensure all biases are defined
        AND ps2.bias_7_state IS NOT NULL 
        AND ps1.bias_7_state IS NOT NULL
        AND cs.bias_7_state IS NOT NULL
),
transition_2nd_order_D AS (
    -- Denominator (Total times PS2_Bias -> PS1_Bias sequence occurred)
    SELECT ps2_name, ps2_bias, ps1_name, ps1_bias, COUNT(*) AS total_attempts
    FROM transition_2nd_order
    GROUP BY 1, 2, 3, 4
)
-- Final Insert for Table 2
INSERT INTO us2000_transition_2nd_order_7b (
    ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, total_attempts, success_count, p_transition
)
SELECT
    t2.ps2_name,
    t2.ps2_bias,
    t2.ps1_name,
    t2.ps1_bias,
    t2.cs_name,
    t2.cs_bias,
    td.total_attempts,
    COUNT(*) AS success_count, -- Numerator
    ROUND((COUNT(*)::NUMERIC / td.total_attempts) * 100, 2) AS p_transition
FROM transition_2nd_order t2
JOIN transition_2nd_order_D td
    ON t2.ps2_name = td.ps2_name 
    AND t2.ps2_bias = td.ps2_bias
    AND t2.ps1_name = td.ps1_name
    AND t2.ps1_bias = td.ps1_bias
GROUP BY 1, 2, 3, 4, 5, 6, td.total_attempts;

------------------------------------------
-- PDH/PDL Calculation
------------------------------------------

-- =================================================================================
-- TABLE CREATION AND SETUP
-- =================================================================================

-- Metric 3-A: First-Order Transition Matrix (3-Bias)
CREATE TABLE IF NOT EXISTS us2000_transition_1st_order_3b (
    ps1_name TEXT NOT NULL,
    ps1_bias TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_transition DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (ps1_name, ps1_bias, cs_name, cs_bias)
);
TRUNCATE us2000_transition_1st_order_3b;

-- Metric 3-B: Second-Order Transition Matrix (7-State)
CREATE TABLE IF NOT EXISTS us2000_transition_2nd_order_7b (
    ps2_name TEXT NOT NULL,
    ps2_bias TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps1_bias TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_transition DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias)
);



-- =================================================================================
-- 0. FOUNDATION: UNIFIED BIAS SOURCE (S0)
-- =================================================================================
WITH unified_bias_source AS (
    SELECT
        trading_date,
        asset_id,
        session_name,
        start_ts,
        
        session_type AS bias_3_state,
        
        CASE 
            WHEN session_type IN ('Bullish', 'Bearish') 
                THEN session_type
            WHEN session_type = 'Consolidation' 
                THEN consolidation_subtype
            ELSE 'Other' 
        END AS bias_7_state
        
    FROM us2000_session_views
),

-- =================================================================================
-- 1. METRIC 3-A: FIRST-ORDER TRANSITION MATRIX (3-BIAS)
-- =================================================================================
transition_1st_order AS (
    SELECT
        ps.session_name AS ps1_name,
        ps.bias_3_state AS ps1_bias,
        cs.session_name AS cs_name,
        cs.bias_3_state AS cs_bias,
        cs.trading_date AS cs_trading_date -- Add for the insert below
    FROM unified_bias_source ps
    JOIN unified_bias_source cs
        ON ps.asset_id = cs.asset_id
        AND ps.start_ts < cs.start_ts
        -- Sequential Session Logic (Handles intra-day and cross-day)
        AND (
            (ps.session_name = 'AS' AND cs.session_name = 'LN' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'LN' AND cs.session_name = 'NYAM' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYAM' AND cs.session_name = 'NYL' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYL' AND cs.session_name = 'NYPM' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYPM' AND cs.session_name = 'AS' AND cs.trading_date = ps.trading_date + INTERVAL '1 day')
        )
),

transition_1st_order_D AS (
    SELECT ps1_name, ps1_bias, COUNT(*) AS total_attempts
    FROM transition_1st_order
    GROUP BY 1, 2
),

transition_1st_order_results AS (
    SELECT
        t1.ps1_name,
        t1.ps1_bias,
        t1.cs_name,
        t1.cs_bias,
        td.total_attempts,
        COUNT(*) AS success_count,
        ROUND((COUNT(*)::NUMERIC / td.total_attempts) * 100, 2) AS p_transition
    FROM transition_1st_order t1
    JOIN transition_1st_order_D td
        ON t1.ps1_name = td.ps1_name AND t1.ps1_bias = td.ps1_bias
    GROUP BY 1, 2, 3, 4, td.total_attempts
)

-- INSERT INTO Table 3-A
INSERT INTO us2000_transition_1st_order_3b (
    ps1_name, ps1_bias, cs_name, cs_bias, total_attempts, success_count, p_transition
)
SELECT * FROM transition_1st_order_results;


-- =================================================================================
-- 2. METRIC 3-B: SECOND-ORDER TRANSITION MATRIX (7-STATE)
-- =================================================================================
, transition_2nd_order AS (
    SELECT
        ps2.session_name AS ps2_name,
        ps2.bias_7_state AS ps2_bias,
        ps1.session_name AS ps1_name,
        ps1.bias_7_state AS ps1_bias,
        cs.session_name AS cs_name,
        cs.bias_7_state AS cs_bias
    FROM unified_bias_source ps2
    JOIN unified_bias_source ps1
        ON ps2.asset_id = ps1.asset_id 
        AND ps2.start_ts < ps1.start_ts
    JOIN unified_bias_source cs
        ON ps1.asset_id = cs.asset_id
        AND ps1.start_ts < cs.start_ts
    WHERE 
        ps1.start_ts = (SELECT MIN(t.start_ts) FROM unified_bias_source t WHERE t.asset_id = ps2.asset_id AND t.start_ts > ps2.start_ts)
        AND cs.start_ts = (SELECT MIN(t.start_ts) FROM unified_bias_source t WHERE t.asset_id = ps1.asset_id AND t.start_ts > ps1.start_ts)
        AND ps2.bias_7_state != 'Other' AND ps1.bias_7_state != 'Other' AND cs.bias_7_state != 'Other'
),

transition_2nd_order_D AS (
    SELECT ps2_name, ps2_bias, ps1_name, ps1_bias, COUNT(*) AS total_attempts
    FROM transition_2nd_order
    GROUP BY 1, 2, 3, 4
),

transition_2nd_order_results AS (
    SELECT
        t2.ps2_name,
        t2.ps2_bias,
        t2.ps1_name,
        t2.ps1_bias,
        t2.cs_name,
        t2.cs_bias,
        td.total_attempts,
        COUNT(*) AS success_count,
        ROUND((COUNT(*)::NUMERIC / td.total_attempts) * 100, 2) AS p_transition
    FROM transition_2nd_order t2
    JOIN transition_2nd_order_D td
        ON t2.ps2_name = td.ps2_name 
        AND t2.ps2_bias = td.ps2_bias
        AND t2.ps1_name = td.ps1_name
        AND t2.ps1_bias = td.ps1_bias
    GROUP BY 1, 2, 3, 4, 5, 6, td.total_attempts
)

-- INSERT INTO Table 3-B
INSERT INTO us2000_transition_2nd_order_7b (
    ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, total_attempts, success_count, p_transition
)
SELECT * FROM transition_2nd_order_results;


-- =================================================================================
-- 3. METRIC 4: CONDITIONAL PDH/PDL TAKEDOWN PROBABILITY (3-BIAS)
-- =================================================================================
, takedown_events_pdh_pdl AS (
    SELECT
        trading_date,
        breaker_session,
        prior_level
    FROM us2000_takedown_events
    WHERE 
        prior_session = 'PrevDay-Daily'
        AND prior_level IN ('High', 'Low')
),

pdh_pdl_attempts AS (
    -- Reuses the D from the 1st_order CTE
    SELECT
        ps1_name AS ps_name,
        ps1_bias AS ps_bias,
        cs_name,
        cs_bias,
        cs_trading_date AS attempt_date
    FROM transition_1st_order -- Reusing the pairs calculated for 1st-order
),

pdh_pdl_denominators AS (
    -- Denominator D: Total times the PS_Bias -> CS_Bias pattern occurred (Same as transition_1st_order_D)
    SELECT ps_name, ps_bias, cs_name, cs_bias, COUNT(*) AS total_attempts
    FROM pdh_pdl_attempts
    GROUP BY 1, 2, 3, 4
),

pdh_pdl_successes AS (
    SELECT
        pa.ps_name,
        pa.ps_bias,
        pa.cs_name,
        pa.cs_bias,
        tde.prior_level,
        COUNT(tde.trading_date) AS break_count
    FROM pdh_pdl_attempts pa
    JOIN takedown_events_pdh_pdl tde
        ON pa.attempt_date = tde.trading_date 
        AND pa.cs_name = tde.breaker_session
    GROUP BY 1, 2, 3, 4, 5
)
-- INSERT INTO Table 4
INSERT INTO us2000_conditional_pdh_pdl (
    date, asset_id, ps_name, ps_bias, cs_name, cs_bias, prior_level, total_attempts, break_count, p_takedown
)
SELECT
    tde.attempt_date,
    s.asset_id,
    s.ps_name,
    s.ps_bias,
    s.cs_name,
    s.cs_bias,
    s.prior_level,
    d.total_attempts,
    s.break_count,
    ROUND((s.break_count::NUMERIC / d.total_attempts) * 100, 2) AS p_takedown
FROM pdh_pdl_successes s
JOIN pdh_pdl_denominators d
    ON s.ps_name = d.ps_name
    AND s.ps_bias = d.ps_bias
    AND s.cs_name = d.cs_name
    AND s.cs_bias = d.cs_bias;

-- =================================================================================
-- FINAL OUTPUT (VIEW ALL RESULTS)
-- =================================================================================
-- Selects all calculated data from the three new tables
SELECT * FROM us2000_transition_1st_order_3b
UNION ALL
SELECT * FROM us2000_transition_2nd_order_7b
UNION ALL
SELECT * FROM us2000_conditional_pdh_pdl;

----------------
--PDL
---------

DROP TABLE IF EXISTS us2000_conditional_pdh_pdl CASCADE;
-- Metric 4: Conditional PDH/PDL Takedown Probability (3-Bias)
CREATE TABLE IF NOT EXISTS us2000_conditional_pdh_pdl (
    ps_name TEXT NOT NULL,
    ps_bias TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias TEXT NOT NULL,
    prior_level TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    break_count BIGINT NOT NULL,
    p_takedown DOUBLE PRECISION NOT NULL,
    break_dates DATE[] NOT NULL,
    PRIMARY KEY (ps_name, ps_bias, cs_name, cs_bias, prior_level)
);
TRUNCATE us2000_conditional_pdh_pdl;

WITH session_bias_source AS (
    SELECT
        trading_date,
        asset_id,
        session_name,
        start_ts,
        session_type AS bias_3_state -- Using 3-state for statistical stability
    FROM asset_session_views
),
takedown_events_pdh_pdl AS (
    SELECT
        trading_date,
        prior_session,
        breaker_session,
        prior_level
    FROM us2000_takedown_events
    WHERE 
        prior_session = 'PrevDay-Daily' -- Ensures the level originated from the Previous Day's full range
        AND prior_level IN ('High', 'Low') -- We look at the actual PDH and PDL levels
),
pattern_attempts AS (
    -- Generates all sequential pairs (PS -> CS) and their biases
    SELECT
        ps.session_name AS ps_name,
        ps.bias_3_state AS ps_bias,
        cs.session_name AS cs_name,
        cs.bias_3_state AS cs_bias,
        -- The date of the Breaker Session (CS) is the date of the attempt
        cs.trading_date AS attempt_date
    FROM session_bias_source ps -- Prior Session (PS)
    JOIN session_bias_source cs -- Current/Breaker Session (CS)
        ON ps.asset_id = cs.asset_id
        AND ps.start_ts < cs.start_ts
        -- Sequential Session Logic (RE-VALIDATED LOGIC)
        AND (
            (ps.session_name = 'AS' AND cs.session_name = 'LN' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'LN' AND cs.session_name = 'NYAM' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYAM' AND cs.session_name = 'NYL' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYL' AND cs.session_name = 'NYPM' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYPM' AND cs.session_name = 'AS' AND cs.trading_date = ps.trading_date + INTERVAL '1 day') -- Cross-day fix
        )
    WHERE ps.bias_3_state IS NOT NULL AND cs.bias_3_state IS NOT NULL
),
pattern_denominators AS (
    -- Groups the attempts to get the total D count for each PS_Bias -> CS_Bias pattern
    SELECT
        ps_name,
        ps_bias,
        cs_name,
        cs_bias,
        COUNT(*) AS total_attempts
    FROM pattern_attempts
    GROUP BY 1, 2, 3, 4
),
pattern_successes AS (
    -- Matches the successful TDEs (N) to the Pattern Attempts (D)
    SELECT
        pa.ps_name,
        pa.ps_bias,
        pa.cs_name,
        pa.cs_bias,
        tde.prior_level, -- Specifies PDH or PDL
        COUNT(tde.trading_date) AS break_count,
        ARRAY_AGG(tde.trading_date ORDER BY tde.trading_date) AS break_dates
    FROM pattern_attempts pa
    JOIN takedown_events_pdh_pdl tde
        ON pa.attempt_date = tde.trading_date -- Takedown occurred on the CS date
        AND pa.cs_name = tde.breaker_session -- The Breaker Session is the CS
    GROUP BY 1, 2, 3, 4, 5
)
INSERT INTO us2000_conditional_pdh_pdl (
    ps_name, ps_bias, cs_name, cs_bias, prior_level, total_attempts, break_count, p_takedown, break_dates
)
-- Final Output: Calculates P(Takedown) = N / D
SELECT

    s.ps_name,
    s.ps_bias,
    s.cs_name,
    s.cs_bias,
    s.prior_level,
    d.total_attempts,
    s.break_count,
    ROUND((s.break_count::NUMERIC / d.total_attempts) * 100, 2) AS p_takedown,
    s.break_dates
FROM pattern_successes s
JOIN pattern_denominators d
    ON s.ps_name = d.ps_name
    AND s.ps_bias = d.ps_bias
    AND s.cs_name = d.cs_name
    AND s.cs_bias = d.cs_bias
WHERE d.total_attempts > 0 -- Ensure we only show patterns that actually occurred
ORDER BY p_takedown DESC, total_attempts DESC;