

-- Drop the old table and sequences (if they exist)
DROP TABLE IF EXISTS asset_session_context CASCADE;
DROP SEQUENCE IF EXISTS session_record_pk_seq;
DROP SEQUENCE IF EXISTS ps_cs_pattern_fk_seq;
DROP SEQUENCE IF EXISTS ps2_ps1_fk_seq;
DROP SEQUENCE IF EXISTS cs_ps2_fk_seq;

-- Create the new, true Primary Key Sequence (unique for every row)
CREATE SEQUENCE IF NOT EXISTS session_record_pk_seq START 1;

-- Create the sequences for the DENSE_RANK Pattern Type IDs (used as Foreign Keys in metrics)
CREATE SEQUENCE IF NOT EXISTS ps_cs_pattern_fk_seq START 1;
CREATE SEQUENCE IF NOT EXISTS ps2_ps1_fk_seq START 1;
CREATE SEQUENCE IF NOT EXISTS cs_ps2_fk_seq START 1;

CREATE TABLE IF NOT EXISTS asset_session_context (
    -- NEW: The unique Primary Key for every single historical session record
    session_record_pk BIGINT NOT NULL,
    -- RENAMED: This is the ID for the Pattern TYPE (used for joining/grouping in metrics)
    ps_cs_pattern_fk BIGINT NOT NULL,          -- FK for PS1 -> CS pattern TYPE
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    -- Current Session (CS) Details
    cs_name TEXT NOT NULL,
    cs_bias_3_state TEXT,
    cs_bias_7_state TEXT,
    -- Preceding Session 1 (PS1) Details
    ps1_name TEXT NOT NULL,
    ps1_bias_3_state TEXT,
    ps1_bias_7_state TEXT,
    -- Preceding Session 2 (PS2) Details
    ps2_name TEXT,
    ps2_bias_3_state TEXT,
    ps2_bias_7_state TEXT,
    -- Foreign Keys for 2nd and 3rd order pattern types (still DENSE_RANK IDs)
    ps2_ps1_fk BIGINT,                  
    cs_ps2_fk BIGINT,                   
    -- Constraints
    PRIMARY KEY(session_record_pk) -- The true unique identifier for the row
);

-- ---------------------------------------------------------------------------------------------------------------------------------
-- MANDATORY FIX: Truncate the table to prevent the "duplicate key value violates unique constraint" error.
-- This ensures the new INSERT starts clean and uses fresh sequence values for session_record_pk.
-- ---------------------------------------------------------------------------------------------------------------------------------
TRUNCATE TABLE asset_session_context;

-- ---------------------------------------------------------------------------------------------------------------------------------
-- STAGE 1: Prepare Raw Historical Sessions and Assign UNIQUE Primary Key to EVERY single row
-- ---------------------------------------------------------------------------------------------------------------------------------
WITH sequenced_sessions AS (
    -- Get all raw sessions with a session_num for LAG function
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
        END AS bias_7_state,
        ROW_NUMBER() OVER (PARTITION BY asset_id ORDER BY start_ts) AS session_num
    FROM asset_session_views
),
context_pairs AS (
    -- Use LAG to create every historical pattern instance (PS2 -> PS1 -> CS)
    SELECT
        cs.trading_date, cs.asset_id,
        cs.session_name AS cs_name, cs.bias_3_state AS cs_bias_3_state, cs.bias_7_state AS cs_bias_7_state,
        LAG(cs.session_name, 1) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps1_name,
        LAG(cs.bias_3_state, 1) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps1_bias_3_state,
        LAG(cs.bias_7_state, 1) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps1_bias_7_state,
        LAG(cs.session_name, 2) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps2_name,
        LAG(cs.bias_3_state, 2) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps2_bias_3_state,
        LAG(cs.bias_7_state, 2) OVER (PARTITION BY cs.asset_id ORDER BY cs.start_ts) AS ps2_bias_7_state,
        -- ASSIGN THE UNIQUE PRIMARY KEY NOW, guaranteeing a unique ID for every historical row
        nextval('session_record_pk_seq') AS calculated_session_record_pk
    FROM sequenced_sessions cs
),
filtered_patterns AS (
    -- FIX: Filter is applied here, using the aliased column 'ps1_name'
    SELECT * FROM context_pairs 
    WHERE ps1_name IS NOT NULL
),
-- ---------------------------------------------------------------------------------------------------------------------------------
-- STAGE 2: Calculate the Pattern Type Foreign Keys (DENSE_RANK only)
-- ---------------------------------------------------------------------------------------------------------------------------------
unique_patterns AS (
    -- Select the five fields that define the pattern type
    SELECT DISTINCT
        asset_id, ps1_name, cs_name, ps1_bias_3_state, cs_bias_3_state, ps2_name, ps2_bias_7_state, ps1_bias_7_state, cs_bias_7_state,
        -- FK for the PS1 -> CS pattern TYPE (DENSE_RANK ID)
        DENSE_RANK() OVER (
            ORDER BY 
                asset_id, ps1_name, cs_name, 
                ps1_bias_3_state, cs_bias_3_state
        ) AS ps_cs_drank,
        -- FK for the PS2 -> PS1 pair TYPE (Only calculate if PS2 exists)
        CASE WHEN ps2_name IS NOT NULL THEN 
             DENSE_RANK() OVER (
                ORDER BY asset_id, ps2_name, ps1_name, ps2_bias_7_state, ps1_bias_7_state
            ) 
        END AS ps2_ps1_drank,
        -- FK for the PS2 -> PS1 -> CS triple TYPE
        CASE WHEN ps2_name IS NOT NULL THEN
            DENSE_RANK() OVER (
                ORDER BY asset_id, ps2_name, ps1_name, cs_name, 
                    ps2_bias_7_state, ps1_bias_7_state, cs_bias_7_state
            ) 
        END AS cs_ps2_drank
        
    FROM filtered_patterns
),
final_unique_keys AS (
    -- Assign the sequence to the DENSE_RANK results once per pattern type
    SELECT 
        asset_id, ps1_name, cs_name, ps1_bias_3_state, cs_bias_3_state,
        ps2_name, ps2_bias_7_state, ps1_bias_7_state, cs_bias_7_state,
        ps_cs_drank + nextval('ps_cs_pattern_fk_seq') AS calculated_ps_cs_pattern_fk,
        ps2_ps1_drank + nextval('ps2_ps1_fk_seq') AS calculated_ps2_ps1_fk,
        cs_ps2_drank + nextval('cs_ps2_fk_seq') AS calculated_cs_ps2_fk
    FROM unique_patterns
)
-- ---------------------------------------------------------------------------------------------------------------------------------
-- STAGE 3: Final Insert (Join the Unique Pattern Keys back to the full history)
-- ---------------------------------------------------------------------------------------------------------------------------------
INSERT INTO asset_session_context (
    session_record_pk, ps_cs_pattern_fk, ps2_ps1_fk, cs_ps2_fk,
    trading_date, asset_id, cs_name, cs_bias_3_state, 
    cs_bias_7_state, ps1_name, ps1_bias_3_state, ps1_bias_7_state, 
    ps2_name, ps2_bias_3_state, ps2_bias_7_state
)
SELECT
    fp.calculated_session_record_pk, fuk.calculated_ps_cs_pattern_fk, fuk.calculated_ps2_ps1_fk, fuk.calculated_cs_ps2_fk,
    fp.trading_date, fp.asset_id, fp.cs_name, fp.cs_bias_3_state, 
    fp.cs_bias_7_state, fp.ps1_name, fp.ps1_bias_3_state, fp.ps1_bias_7_state, 
    fp.ps2_name, fp.ps2_bias_3_state, fp.ps2_bias_7_state
FROM filtered_patterns fp
JOIN final_unique_keys fuk 
    ON fp.asset_id = fuk.asset_id
    AND fp.ps1_name = fuk.ps1_name
    AND fp.cs_name = fuk.cs_name
    AND fp.ps1_bias_3_state = fuk.ps1_bias_3_state
    AND fp.cs_bias_3_state = fuk.cs_bias_3_state;
