------------------
-- daily predictions
-------------------

-- Metric 5-A: First-Order Day Type Probability (3-Bias -> 3-Bias -> 7-State)
CREATE TABLE IF NOT EXISTS us2000_day_type_1st_order (
    asset_id TEXT NOT NULL,
    ps_name TEXT NOT NULL,
    ps_bias_3 TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias_3 TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_day_type DOUBLE PRECISION NOT NULL,
    success_dates DATE[] NOT NULL,
    PRIMARY KEY (asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7)
);
TRUNCATE us2000_day_type_1st_order;

-- Metric 5-B: Second-Order Day Type Probability (7-Bias -> 7-Bias -> 7-Bias -> 7-State)
CREATE TABLE IF NOT EXISTS us2000_day_type_2nd_order (
    asset_id TEXT NOT NULL,
    ps2_name TEXT NOT NULL,
    ps2_bias_7 TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps1_bias_7 TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias_7 TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_day_type DOUBLE PRECISION NOT NULL,
    success_dates DATE[] NOT NULL,
    PRIMARY KEY (asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, daily_outcome_7)
);
TRUNCATE us2000_day_type_2nd_order;

-- ASSUMPTION: unified_bias_source (with bias_3_state and bias_7_state) is defined prior to this.
-- If not, define it here:
WITH unified_bias_source AS (
    SELECT
        trading_date,
        asset_id,
        session_name,
        start_ts,
        session_type AS bias_3_state,
        CASE 
            WHEN session_type IN ('Bullish', 'Bearish') THEN session_type
            WHEN session_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other' 
        END AS bias_7_state
    FROM asset_session_views
),
daily_outcome_7_state AS (
    -- Defines the 7-State Outcome (Bullish/Bearish or one of the 5 consolidation subtypes)
    SELECT
        trading_date,
        asset_id,
        CASE
            WHEN day_type IN ('Bullish', 'Bearish') THEN day_type
            WHEN day_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other' 
        END AS daily_outcome_7
    FROM asset_daily_views
    WHERE day_type IS NOT NULL AND asset_id = 'assets:US2000' -- Filter for the specific asset
),
sequential_pairs_3b AS (
    -- Generates all sequential pairs (PS -> CS) with 3-Bias (used for D)
    SELECT
        ps.trading_date,
        ps.asset_id,
        ps.session_name AS ps_name,
        ps.bias_3_state AS ps_bias_3,
        cs.session_name AS cs_name,
        cs.bias_3_state AS cs_bias_3
    FROM unified_bias_source ps
    JOIN unified_bias_source cs
        ON ps.asset_id = cs.asset_id AND ps.start_ts < cs.start_ts
        -- Sequential Session Logic (Ensure PS is immediately before CS)
        AND (
            (ps.session_name = 'AS' AND cs.session_name = 'LN' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'LN' AND cs.session_name = 'NYAM' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYAM' AND cs.session_name = 'NYL' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYL' AND cs.session_name = 'NYPM' AND ps.trading_date = cs.trading_date) OR
            (ps.session_name = 'NYPM' AND cs.session_name = 'AS' AND cs.trading_date = ps.trading_date + INTERVAL '1 day')
        )
),
day_type_1st_order_successes AS (
    -- Matches the PS -> CS pattern to the Daily Outcome (N)
    SELECT
        p.asset_id,
        p.ps_name,
        p.ps_bias_3,
        p.cs_name,
        p.cs_bias_3,
        d.daily_outcome_7,
        COUNT(*) AS success_count, -- Numerator N
        ARRAY_AGG(p.trading_date ORDER BY p.trading_date) AS success_dates -- Successful dates
    FROM sequential_pairs_3b p
    JOIN daily_outcome_7_state d
        ON p.trading_date = d.trading_date AND p.asset_id = d.asset_id
    GROUP BY 1, 2, 3, 4, 5, 6
),
day_type_1st_order_D AS (
    -- Denominator D: Total attempts for each pattern
    SELECT asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, COUNT(*) AS total_attempts
    FROM sequential_pairs_3b
    GROUP BY 1, 2, 3, 4, 5
)
-- INSERT INTO Table 5-A
INSERT INTO us2000_day_type_1st_order (
    asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, total_attempts, success_count, p_day_type, success_dates
)
SELECT
    s.asset_id,
    s.ps_name,
    s.ps_bias_3,
    s.cs_name,
    s.cs_bias_3,
    s.daily_outcome_7,
    d.total_attempts,
    s.success_count,
    ROUND((s.success_count::NUMERIC / d.total_attempts) * 100, 2) AS p_day_type,
    s.success_dates
FROM day_type_1st_order_successes s
JOIN day_type_1st_order_D d
    ON s.asset_id = d.asset_id AND s.ps_name = d.ps_name
    AND s.ps_bias_3 = d.ps_bias_3 AND s.cs_name = d.cs_name
    AND s.cs_bias_3 = d.cs_bias_3
WHERE d.total_attempts > 0;
----------------------------
-- 2nd order day predictions
----------------------------
-- ASSUMPTION: unified_bias_source (with bias_3_state and bias_7_state) is defined prior to this.
-- If not, define it here:
WITH unified_bias_source AS (
    SELECT
        trading_date,
        asset_id,
        session_name,
        start_ts,
        session_type AS bias_3_state,
        CASE 
            WHEN session_type IN ('Bullish', 'Bearish') THEN session_type
            WHEN session_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other' 
        END AS bias_7_state
    FROM us2000_session_views
),
daily_outcome_7_state AS (
    -- Defines the 7-State Outcome (Bullish/Bearish or one of the 5 consolidation subtypes)
    SELECT
        trading_date,
        asset_id,
        CASE
            WHEN daily_type IN ('Bullish', 'Bearish') THEN daily_type
            WHEN daily_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other' 
        END AS daily_outcome_7
    FROM us2000_daily_views
    WHERE daily_type IS NOT NULL AND asset_id = 'assets:US2000' -- Filter for the specific asset
),
sequential_triples_7b AS (
    -- Generates all sequential triples (PS2 -> PS1 -> CS) with 7-Bias
    SELECT
        ps2.trading_date,
        ps2.asset_id,
        ps2.session_name AS ps2_name,
        ps2.bias_7_state AS ps2_bias_7,
        ps1.session_name AS ps1_name,
        ps1.bias_7_state AS ps1_bias_7,
        cs.session_name AS cs_name,
        cs.bias_7_state AS cs_bias_7
    FROM unified_bias_source ps2 -- Prior Session 2
    JOIN unified_bias_source ps1 -- Prior Session 1 (PS2 -> PS1)
        ON ps2.asset_id = ps1.asset_id AND ps2.start_ts < ps1.start_ts
    JOIN unified_bias_source cs  -- Current Session (PS1 -> CS)
        ON ps1.asset_id = cs.asset_id AND ps1.start_ts < cs.start_ts
    -- Sequential Logic Filter (Ensures PS2 is immediately before PS1, and PS1 is immediately before CS)
    WHERE 
        ps1.start_ts = (SELECT MIN(t.start_ts) FROM unified_bias_source t WHERE t.asset_id = ps2.asset_id AND t.start_ts > ps2.start_ts)
        AND cs.start_ts = (SELECT MIN(t.start_ts) FROM unified_bias_source t WHERE t.asset_id = ps1.asset_id AND t.start_ts > ps1.start_ts)
        AND ps2.bias_7_state != 'Other' AND ps1.bias_7_state != 'Other' AND cs.bias_7_state != 'Other'
),
day_type_2nd_order_successes AS (
    -- Matches the PS2 -> PS1 -> CS pattern to the Daily Outcome (N)
    SELECT
        t.asset_id,
        t.ps2_name,
        t.ps2_bias_7,
        t.ps1_name,
        t.ps1_bias_7,
        t.cs_name,
        t.cs_bias_7,
        d.daily_outcome_7,
        COUNT(*) AS success_count, -- Numerator N
        ARRAY_AGG(t.trading_date ORDER BY t.trading_date) AS success_dates -- Successful dates
    FROM sequential_triples_7b t
    JOIN daily_outcome_7_state d
        ON t.trading_date = d.trading_date AND t.asset_id = d.asset_id
    GROUP BY 1, 2, 3, 4, 5, 6, 7, 8
),
day_type_2nd_order_D AS (
    -- Denominator D: Total attempts for each pattern
    SELECT asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, COUNT(*) AS total_attempts
    FROM sequential_triples_7b
    GROUP BY 1, 2, 3, 4, 5, 6, 7
)
-- INSERT INTO Table 5-B
INSERT INTO us2000_day_type_2nd_order (
    asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, daily_outcome_7, total_attempts, success_count, p_day_type, success_dates
)
SELECT
    s.asset_id,
    s.ps2_name,
    s.ps2_bias_7,
    s.ps1_name,
    s.ps1_bias_7,
    s.cs_name,
    s.cs_bias_7,
    s.daily_outcome_7,
    d.total_attempts,
    s.success_count,
    ROUND((s.success_count::NUMERIC / d.total_attempts) * 100, 2) AS p_day_type,
    s.success_dates
FROM day_type_2nd_order_successes s
JOIN day_type_2nd_order_D d
    ON s.asset_id = d.asset_id AND s.ps2_name = d.ps2_name
    AND s.ps2_bias_7 = d.ps2_bias_7 AND s.ps1_name = d.ps1_name
    AND s.ps1_bias_7 = d.ps1_bias_7 AND s.cs_name = d.cs_name
    AND s.cs_bias_7 = d.cs_bias_7
WHERE d.total_attempts > 0;