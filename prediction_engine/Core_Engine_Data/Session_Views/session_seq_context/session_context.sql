-- -----------------------------------------------------------
-- 1. Create the Final session_context Table
-- Primary Key ensures uniqueness per session.
-- -----------------------------------------------------------
DROP TABLE session_context; 
CREATE TABLE session_context (
    trading_date date NOT NULL,
    asset_id text NOT NULL,
    cs_name text NOT NULL,             -- Current Session Name (CS)
    -- 1st Order Context Keys (3-State Biases)
    cs_bias_3 text,
    ps1_name text,
    ps1_bias_3 text,
    -- 2nd Order Context Keys (7-State Biases)
    cs_bias_7 text,
    ps1_bias_7 text,
    ps2_name text,
    ps2_bias_7 text,
    -- Timestamp for sequencing
    session_end_ts timestamp with time zone NOT NULL,
    -- The PRIMARY KEY is the only necessary UNIQUE constraint
    PRIMARY KEY(trading_date, asset_id, cs_name)
);

-- FIX: Replacing the faulty UNIQUE index on session_end_ts with a composite, 
-- non-unique index to support efficient PARTITION BY asset_id and ORDER BY end_ts for LAG.

CREATE INDEX session_context_sequence_idx ON public.session_context USING btree (asset_id, session_end_ts DESC);


-- -----------------------------------------------------------
-- 2. SQL to populate/recalculate the session_context table
-- -----------------------------------------------------------

WITH session_with_keys AS (
    -- Step A: Derive the 7-State Bias from existing columns (session_type and consolidation_subtype)
    SELECT
        sv.trading_date,
        sv.asset_id,
        sv.session_name,
        sv.end_ts,
        sv.session_type AS bias_3, 
        CASE
            WHEN sv.session_type = 'Consolidation' THEN sv.consolidation_subtype
            ELSE sv.session_type
        END AS bias_7 
    FROM
        session_views sv
),
ordered_sessions AS (
    -- Step B: Apply LAG window functions to establish PS1 and PS2 context
    SELECT
        sk.trading_date,
        sk.asset_id,
        sk.session_name,
        sk.end_ts,
        sk.bias_3 AS cs_bias_3,
        sk.bias_7 AS cs_bias_7,
        -- PS1 (Preceding Session 1)
        LAG(sk.session_name, 1) OVER (PARTITION BY sk.asset_id ORDER BY sk.end_ts) AS ps1_name,
        LAG(sk.bias_3, 1) OVER (PARTITION BY sk.asset_id ORDER BY sk.end_ts) AS ps1_bias_3,
        LAG(sk.bias_7, 1) OVER (PARTITION BY sk.asset_id ORDER BY sk.end_ts) AS ps1_bias_7,
        -- PS2 (Preceding Session 2)
        LAG(sk.session_name, 2) OVER (PARTITION BY sk.asset_id ORDER BY sk.end_ts) AS ps2_name,
        LAG(sk.bias_7, 2) OVER (PARTITION BY sk.asset_id ORDER BY sk.end_ts) AS ps2_bias_7
    FROM
        session_with_keys sk
)
-- Step C: Insert the derived context into the session_context table
INSERT INTO session_context (
    trading_date, asset_id, cs_name, cs_bias_3, ps1_name, ps1_bias_3, cs_bias_7, ps1_bias_7, ps2_name, ps2_bias_7, session_end_ts
)
SELECT
    trading_date, asset_id, session_name, cs_bias_3, ps1_name, ps1_bias_3, cs_bias_7, ps1_bias_7, ps2_name, ps2_bias_7, end_ts
FROM
    ordered_sessions
WHERE
    ps1_name IS NOT NULL -- Must have at least a 1st preceding session
ON CONFLICT (trading_date, asset_id, cs_name) DO UPDATE
SET
    cs_bias_3 = EXCLUDED.cs_bias_3,
    ps1_name = EXCLUDED.ps1_name,
    ps1_bias_3 = EXCLUDED.ps1_bias_3,
    cs_bias_7 = EXCLUDED.cs_bias_7,
    ps1_bias_7 = EXCLUDED.ps1_bias_7,
    ps2_name = EXCLUDED.ps2_name,
    ps2_bias_7 = EXCLUDED.ps2_bias_7,
    session_end_ts = EXCLUDED.session_end_ts;



SELECT create_hypertable(
    'session_context', 
    by_range('session_end_ts', INTERVAL '7 days'), -- Partition by time (e.g., weekly chunks)
    create_default_indexes => TRUE,
    if_not_exists => TRUE
);

-- OPTIONAL: Add partitioning by asset_id (recommended)
SELECT add_dimension('session_context', by_hash('asset_id', 4)); -- Hash partition into 4 chunks per time-chunk

CREATE OR REPLACE PROCEDURE refresh_session_context()
LANGUAGE sql
AS $$
    -- 1. Truncate is the fastest way to delete all data and reset the table for a full refresh.
    TRUNCATE session_context;

    -- 2. SQL to populate/recalculate the session_context table (simplified INSERT)
    WITH session_with_keys AS (
        -- Step A: Derive the 7-State Bias from existing columns (session_type and consolidation_subtype)
        SELECT
            sv.trading_date,
            sv.asset_id,
            sv.session_name,
            sv.end_ts,
            sv.session_type AS bias_3, 
            CASE
                WHEN sv.session_type = 'Consolidation' THEN sv.consolidation_subtype
                ELSE sv.session_type
            END AS bias_7 
        FROM
            session_views sv -- Assumes session_views is the correct source
    ),
    ordered_sessions AS (
        -- Step B: Apply LAG window functions to establish PS1 and PS2 context
        SELECT
            sk.trading_date,
            sk.asset_id,
            sk.session_name,
            sk.end_ts,
            sk.bias_3 AS cs_bias_3,
            sk.bias_7 AS cs_bias_7,
            -- PS1 (Preceding Session 1)
            LAG(sk.session_name, 1) OVER (PARTITION BY sk.asset_id ORDER BY sk.end_ts) AS ps1_name,
            LAG(sk.bias_3, 1) OVER (PARTITION BY sk.asset_id ORDER BY sk.end_ts) AS ps1_bias_3,
            LAG(sk.bias_7, 1) OVER (PARTITION BY sk.asset_id ORDER BY sk.end_ts) AS ps1_bias_7,
            -- PS2 (Preceding Session 2)
            LAG(sk.session_name, 2) OVER (PARTITION BY sk.asset_id ORDER BY sk.end_ts) AS ps2_name,
            LAG(sk.bias_7, 2) OVER (PARTITION BY sk.asset_id ORDER BY sk.end_ts) AS ps2_bias_7
        FROM
            session_with_keys sk
    )
    -- Step C: Insert the derived context into the session_context table (simple INSERT)
    INSERT INTO session_context (
        trading_date, asset_id, cs_name, cs_bias_3, ps1_name, ps1_bias_3, cs_bias_7, ps1_bias_7, ps2_name, ps2_bias_7, session_end_ts
    )
    SELECT
        trading_date, asset_id, session_name, cs_bias_3, ps1_name, ps1_bias_3, cs_bias_7, ps1_bias_7, ps2_name, ps2_bias_7, end_ts
    FROM
        ordered_sessions
    WHERE
        ps1_name IS NOT NULL; -- Must have at least a 1st preceding session
$$;