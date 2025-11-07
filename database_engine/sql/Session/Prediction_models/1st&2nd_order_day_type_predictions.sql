----------------------------------------------------------
-- 1st & 2nd Order Day Type Predictions

DROP TABLE IF EXISTS asset_day_type_1st_order CASCADE;
CREATE TABLE IF NOT EXISTS asset_day_type_1st_order (
    asset_id TEXT NOT NULL,                     -- <<< ADDED for readability
    cs_ps1_fk BIGINT NOT NULL,
    ps_name TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_day_type DOUBLE PRECISION NOT NULL,
    success_dates DATE[] NOT NULL,
    PRIMARY KEY (cs_ps1_fk, daily_outcome_7) -- PK remains on the FK
);
TRUNCATE asset_day_type_1st_order;

WITH daily_outcome_7_state AS (
    -- Defines the 7-State Outcome for ALL assets
    SELECT
        trading_date,
        asset_id,
        CASE
            WHEN day_type IN ('Bullish', 'Bearish') THEN day_type
            WHEN day_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other' 
        END AS daily_outcome_7
    FROM daily_views
    WHERE day_type IS NOT NULL 
),
context_map AS (
    -- Get the unique FK and session metadata from the context table
    SELECT
        cs_ps1_fk,
        trading_date,
        asset_id,                  -- <<< Include asset_id
        ps1_name AS ps_name,
        cs_name
    FROM asset_session_context
    WHERE ps1_bias_3_state IS NOT NULL AND cs_bias_3_state IS NOT NULL
),
day_type_1st_order_successes AS (
    -- Matches the unique FK pattern to the Daily Outcome (N)
    SELECT
        c.asset_id,                -- <<< Include asset_id
        c.cs_ps1_fk,
        c.ps_name,
        c.cs_name,
        d.daily_outcome_7,
        COUNT(*) AS success_count,
        ARRAY_AGG(c.trading_date ORDER BY c.trading_date) AS success_dates
    FROM context_map c
    JOIN daily_outcome_7_state d
        ON c.trading_date = d.trading_date AND c.asset_id = d.asset_id
    GROUP BY 1, 2, 3, 4, 5
),
day_type_1st_order_D AS (
    -- Denominator D: Total attempts for each pattern (Grouped by FK)
    SELECT asset_id, cs_ps1_fk, COUNT(*) AS total_attempts -- <<< Group by asset_id
    FROM context_map
    GROUP BY 1, 2
)
-- INSERT INTO Table 5-A
INSERT INTO asset_day_type_1st_order (
    asset_id, cs_ps1_fk, ps_name, cs_name, daily_outcome_7, total_attempts, success_count, p_day_type, success_dates
)
SELECT
    s.asset_id,                -- <<< Select asset_id
    s.cs_ps1_fk,
    s.ps_name,
    s.cs_name,
    s.daily_outcome_7,
    d.total_attempts,
    s.success_count,
    ROUND((s.success_count::NUMERIC / NULLIF(d.total_attempts, 0)) * 100, 2) AS p_day_type,
    s.success_dates
FROM day_type_1st_order_successes s
JOIN day_type_1st_order_D d
    ON s.cs_ps1_fk = d.cs_ps1_fk AND s.asset_id = d.asset_id -- <<< Join on asset_id
WHERE d.total_attempts > 0;

-------------------------------------------
-- 2nd Order Day Type Predictions
-------------------------------------------
DROP TABLE IF EXISTS asset_day_type_2nd_order CASCADE;
CREATE TABLE IF NOT EXISTS asset_day_type_2nd_order (
    asset_id TEXT NOT NULL,                     -- <<< ADDED for readability
    cs_ps2_fk BIGINT NOT NULL,
    ps2_name TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_day_type DOUBLE PRECISION NOT NULL,
    success_dates DATE[] NOT NULL,
    PRIMARY KEY (cs_ps2_fk, daily_outcome_7) -- PK remains on the FK
);
TRUNCATE asset_day_type_2nd_order;

WITH daily_outcome_7_state AS (
    -- Defines the 7-State Outcome for ALL assets
    SELECT
        trading_date,
        asset_id,
        CASE
            WHEN day_type IN ('Bullish', 'Bearish') THEN day_type
            WHEN day_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other' 
        END AS daily_outcome_7
    FROM asset_daily_views
    WHERE day_type IS NOT NULL 
),
context_map AS (
    -- Get the unique FK and session metadata from the context table
    SELECT
        cs_ps2_fk,
        trading_date,
        asset_id,              -- <<< Include asset_id
        ps2_name,
        ps1_name,
        cs_name
    FROM asset_session_context
    WHERE cs_ps2_fk IS NOT NULL -- Ensures a valid PS2->PS1->CS triple exists
),
day_type_2nd_order_successes AS (
    -- Matches the unique FK pattern to the Daily Outcome (N)
    SELECT
        c.asset_id,            -- <<< Include asset_id
        c.cs_ps2_fk,
        c.ps2_name,
        c.ps1_name,
        c.cs_name,
        d.daily_outcome_7,
        COUNT(*) AS success_count,
        ARRAY_AGG(c.trading_date ORDER BY c.trading_date) AS success_dates
    FROM context_map c
    JOIN daily_outcome_7_state d
        ON c.trading_date = d.trading_date AND c.asset_id = d.asset_id
    GROUP BY 1, 2, 3, 4, 5, 6
),
day_type_2nd_order_D AS (
    -- Denominator D: Total attempts for each pattern (Grouped by FK)
    SELECT asset_id, cs_ps2_fk, COUNT(*) AS total_attempts -- <<< Group by asset_id
    FROM context_map
    GROUP BY 1, 2
)
-- INSERT INTO Table 5-B
INSERT INTO asset_day_type_2nd_order (
    asset_id, cs_ps2_fk, ps2_name, ps1_name, cs_name, daily_outcome_7, total_attempts, success_count, p_day_type, success_dates
)
SELECT
    s.asset_id,            -- <<< Select asset_id
    s.cs_ps2_fk,
    s.ps2_name,
    s.ps1_name,
    s.cs_name,
    s.daily_outcome_7,
    d.total_attempts,
    s.success_count,
    ROUND((s.success_count::NUMERIC / NULLIF(d.total_attempts, 0)) * 100, 2) AS p_day_type,
    s.success_dates
FROM day_type_2nd_order_successes s
JOIN day_type_2nd_order_D d
    ON s.cs_ps2_fk = d.cs_ps2_fk AND s.asset_id = d.asset_id -- <<< Join on asset_id
WHERE d.total_attempts > 0
ON CONFLICT ON CONSTRAINT asset_day_type_2nd_order_pkey DO UPDATE SET
    total_attempts = EXCLUDED.total_attempts,
    success_count = EXCLUDED.success_count,
    p_day_type = EXCLUDED.p_day_type,
    success_dates = EXCLUDED.success_dates;
