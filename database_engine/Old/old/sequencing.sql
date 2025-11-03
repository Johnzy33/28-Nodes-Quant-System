-- 1. Define the sequence generators for our Foreign Keys
CREATE SEQUENCE IF NOT EXISTS cs_ps1_context_seq;
CREATE SEQUENCE IF NOT EXISTS ps2_ps1_context_seq;
-- 2. Create the master context table
CREATE TABLE us2000_session_context (
    -- Primary Keys for linking
    cs_ps1_fk INT NOT NULL DEFAULT nextval('cs_ps1_context_seq') PRIMARY KEY,
    ps2_ps1_fk INT NOT NULL DEFAULT nextval('ps2_ps1_context_seq') UNIQUE,
    -- Contextual Data
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    cs_name TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps2_name TEXT NOT NULL,
    ps1_7_state TEXT NOT NULL,
    ps2_7_state TEXT NOT NULL,
    -- Constraint to prevent duplicate context records
    UNIQUE (asset_id, trading_date, cs_name, ps1_name, ps2_name)
);


--------------------------------------
-- session sequnces 
---------------------

-- NOTE: We assume get_trading_date() and us2000_session_views table exist.
-- The existing us2000_session_views.trading_date column already uses the correct trading day logic.

WITH latest_data AS (
    -- 1. Find the latest trading date available in the session views
    SELECT MAX(trading_date) AS current_trading_date
    FROM us2000_session_views
),
session_with_7_state AS (
    -- 2. Pull all session data for the latest defined trading date
    SELECT
        asset_id,
        trading_date,
        session_name,
        session_type AS bias_3_state,
        COALESCE(consolidation_subtype, session_type) AS bias_7_state
    FROM us2000_session_views
    CROSS JOIN latest_data ld
    -- Filter on the corrected trading date column
    WHERE trading_date = ld.current_trading_date
),
session_sequence AS (
    -- 3. Define the sequential linkage (CS -> PS1 -> PS2) for all five sessions
    SELECT
        cs.asset_id,
        cs.trading_date,
        cs.session_name AS cs_name,
        -- PS1 (Immediate Preceding Session)
        CASE cs.session_name
            WHEN 'AS' THEN 'NYPM'
            WHEN 'LN' THEN 'AS'
            WHEN 'NYAM' THEN 'LN'
            WHEN 'NYL' THEN 'NYAM'
            WHEN 'NYPM' THEN 'NYL'
        END AS ps1_name,
        -- PS2 (Two Sessions Back)
        CASE cs.session_name
            WHEN 'AS' THEN 'NYL'
            WHEN 'LN' THEN 'NYPM'
            WHEN 'NYAM' THEN 'AS'
            WHEN 'NYL' THEN 'LN'
            WHEN 'NYPM' THEN 'NYAM'
        END AS ps2_name
    FROM session_with_7_state cs
    -- All 5 sessions can be a CS (Current Session) for a prediction
    WHERE cs.session_name IN ('AS', 'LN', 'NYAM', 'NYL', 'NYPM')
),
full_context AS (
    -- 4. Join the sequence to the live biases (7-state) for PS1 and PS2
    SELECT
        sq.asset_id,
        sq.trading_date,
        sq.cs_name,
        sq.ps1_name,
        sq.ps2_name,
        -- Get PS1's 7-state bias
        ps1.bias_7_state AS ps1_7_state,
        -- Get PS2's 7-state bias
        ps2.bias_7_state AS ps2_7_state
    FROM session_sequence sq    
    -- --- PS1 Join (Handling the AS cross-date) ---
    JOIN us2000_session_views ps1_base
        ON ps1_base.asset_id = sq.asset_id
        AND ps1_base.session_name = sq.ps1_name
        -- PS1 Date Logic: Prior date ONLY if CS is AS
        AND ps1_base.trading_date = CASE
            WHEN sq.cs_name = 'AS' THEN sq.trading_date - INTERVAL '1 day'
            ELSE sq.trading_date
        END
    JOIN session_with_7_state ps1
        ON ps1.asset_id = ps1_base.asset_id
        AND ps1.session_name = ps1_base.session_name
        AND ps1.trading_date = ps1_base.trading_date
    -- --- PS2 Join (Handling AS and LN cross-date) ---
    JOIN us2000_session_views ps2_base
        ON ps2_base.asset_id = sq.asset_id
        AND ps2_base.session_name = sq.ps2_name
        -- PS2 Date Logic: Prior date if CS is AS or LN
        AND ps2_base.trading_date = CASE
            WHEN sq.cs_name IN ('AS', 'LN') THEN sq.trading_date - INTERVAL '1 day'
            ELSE sq.trading_date
        END
    JOIN session_with_7_state ps2
        ON ps2.asset_id = ps2_base.asset_id
        AND ps2.session_name = ps2_base.session_name
        AND ps2.trading_date = ps2_base.trading_date
)
-- 5. Final Insertion into the Context Table
INSERT INTO us2000_session_context (
    asset_id, trading_date, cs_name, ps1_name, ps2_name, ps1_7_state, ps2_7_state
)
SELECT
    asset_id,
    trading_date,
    cs_name,
    ps1_name,
    ps2_name,
    ps1_7_state,
    ps2_7_state
FROM full_context
ON CONFLICT (asset_id, trading_date, cs_name, ps1_name, ps2_name)
DO NOTHING;
