-- Session Transition CTEs for 1st and 2nd Order Predictions
DROP TABLE IF EXISTS asset_transition_1st_order CASCADE;
CREATE TABLE IF NOT EXISTS asset_transition_1st_order (
    asset_id TEXT NOT NULL,                     -- For readability/querying
    cs_ps1_fk BIGINT NOT NULL,                  -- FK defining the PS1 -> CS pattern (3-Bias)
    ps1_name TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_transition DOUBLE PRECISION NOT NULL,
    success_dates DATE[] NOT NULL,              -- Array of successful dates
    PRIMARY KEY (cs_ps1_fk)
);
TRUNCATE asset_transition_1st_order;

WITH transition_context AS (
    -- Get the unique FK for the PS1 -> CS pattern (3-Bias)
    SELECT
        cs_ps1_fk,
        trading_date,
        asset_id,
        ps1_name,
        cs_name
    FROM asset_session_context 
    -- Only include records where both biases (3-state) are defined
    WHERE ps1_bias_3_state IS NOT NULL AND cs_bias_3_state IS NOT NULL
),
transition_1st_order_successes AS (
    -- N (Numerator): Count of successful transitions for each unique FK
    SELECT
        asset_id,
        cs_ps1_fk,
        ps1_name,
        cs_name,
        COUNT(*) AS success_count,
        ARRAY_AGG(trading_date ORDER BY trading_date) AS success_dates
    FROM transition_context
    GROUP BY 1, 2, 3, 4
),
transition_1st_order_D AS (
    -- D (Denominator): Total attempts for each unique PS1_Name / PS1_Bias
    -- We need to group by the PS1 state (Name and 3-Bias), NOT the FK,
    -- because the Denominator is based on the PS1 state *regardless* of the CS state.
    SELECT
        asset_id,
        ps1_name,
        ps1_bias_3_state AS ps1_bias,
        COUNT(*) AS total_attempts
    FROM asset_session_context
    WHERE ps1_bias_3_state IS NOT NULL
    GROUP BY 1, 2, 3
)
INSERT INTO asset_transition_1st_order (
    asset_id, cs_ps1_fk, ps1_name, cs_name, total_attempts, success_count, p_transition, success_dates
)
SELECT
    s.asset_id,
    s.cs_ps1_fk,
    s.ps1_name,
    s.cs_name,
    d.total_attempts,
    s.success_count,
    ROUND((s.success_count::NUMERIC / NULLIF(d.total_attempts, 0)) * 100, 2) AS p_transition,
    s.success_dates
FROM transition_1st_order_successes s
-- Join Denominator on the *component* fields that make up the PS1 state
JOIN asset_session_context ctx ON s.cs_ps1_fk = ctx.cs_ps1_fk
JOIN transition_1st_order_D d
    ON ctx.asset_id = d.asset_id
    AND ctx.ps1_name = d.ps1_name
    AND ctx.ps1_bias_3_state = d.ps1_bias
WHERE d.total_attempts > 0;

-----------------------------------------------
-- 2nd Order Session Transition Predictions
-----------------------------------------------
DROP TABLE IF EXISTS asset_transition_2nd_order CASCADE;
CREATE TABLE IF NOT EXISTS asset_transition_2nd_order (
    asset_id TEXT NOT NULL,                     -- For readability/querying
    cs_ps2_fk BIGINT NOT NULL,                  -- FK defining the PS2 -> PS1 -> CS pattern (7-Bias)
    ps2_name TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_transition DOUBLE PRECISION NOT NULL,
    success_dates DATE[] NOT NULL,              -- Array of successful dates
    PRIMARY KEY (cs_ps2_fk)
);
TRUNCATE asset_transition_2nd_order;

WITH transition_context AS (
    -- Get the unique FK for the PS2 -> PS1 -> CS pattern (7-Bias)
    SELECT
        cs_ps2_fk,
        trading_date,
        asset_id,
        ps2_name,
        ps1_name,
        cs_name
    FROM asset_session_context 
    -- Only include records where all three 7-biases are defined (i.e., FK was created)
    WHERE cs_ps2_fk IS NOT NULL
),
transition_2nd_order_successes AS (
    -- N (Numerator): Count of successful transitions for each unique FK
    SELECT
        asset_id,
        cs_ps2_fk,
        ps2_name,
        ps1_name,
        cs_name,
        COUNT(*) AS success_count,
        ARRAY_AGG(trading_date ORDER BY trading_date) AS success_dates
    FROM transition_context
    GROUP BY 1, 2, 3, 4, 5
),
transition_2nd_order_D AS (
    -- D (Denominator): Total attempts for each unique PS2_Bias / PS1_Bias
    -- The Denominator is based on the PS2->PS1 state *regardless* of the CS state.
    -- We can use the ps2_ps1_fk for the denominator, but we'll stick to joining on fields for clarity and assurance.
    SELECT
        asset_id,
        ps2_name,
        ps2_bias_7_state AS ps2_bias,
        ps1_name,
        ps1_bias_7_state AS ps1_bias,
        COUNT(*) AS total_attempts
    FROM asset_session_context
    WHERE ps2_bias_7_state IS NOT NULL AND ps1_bias_7_state IS NOT NULL
    GROUP BY 1, 2, 3, 4, 5
)
-- INSERT INTO Table 3-B
INSERT INTO asset_transition_2nd_order (
    asset_id, cs_ps2_fk, ps2_name, ps1_name, cs_name, total_attempts, success_count, p_transition, success_dates
)
SELECT
    s.asset_id,
    s.cs_ps2_fk,
    s.ps2_name,
    s.ps1_name,
    s.cs_name,
    d.total_attempts,
    s.success_count,
    ROUND((s.success_count::NUMERIC / NULLIF(d.total_attempts, 0)) * 100, 2) AS p_transition,
    s.success_dates
FROM transition_2nd_order_successes s
-- Join Denominator on the PS2 -> PS1 component fields
JOIN asset_session_context ctx ON s.cs_ps2_fk = ctx.cs_ps2_fk
JOIN transition_2nd_order_D d
    ON ctx.asset_id = d.asset_id
    AND ctx.ps2_name = d.ps2_name
    AND ctx.ps2_bias_7_state = d.ps2_bias
    AND ctx.ps1_name = d.ps1_name
    AND ctx.ps1_bias_7_state = d.ps1_bias
WHERE d.total_attempts > 0;