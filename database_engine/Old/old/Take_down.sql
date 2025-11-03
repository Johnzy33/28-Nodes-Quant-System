
Drop TABLE IF EXISTS us2000_structural_takedowns CASCADE;
-- Metric 4: Structural Takedowns Table (RE-CREATION)
CREATE TABLE IF NOT EXISTS us2000_structural_takedowns (
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    session_name TEXT NOT NULL,
    bias_7_state TEXT NOT NULL,
    is_pdh_break BOOLEAN NOT NULL,
    is_pdl_break BOOLEAN NOT NULL,
    PRIMARY KEY (asset_id, trading_date, session_name)
);
TRUNCATE us2000_structural_takedowns;

-- CTEs to calculate structural takedowns using a LAG function
WITH daily_extremes AS (
    -- Step 1: Calculate the overall High/Low for the day from session data
    SELECT
        asset_id,
        trading_date,
        MAX(high) AS daily_high,
        MIN(low) AS daily_low
    FROM us2000_session_views
    GROUP BY 1, 2
),
previous_day_extremes AS (
    -- Step 2: Use LAG to find the Previous Day High (PDH) and Low (PDL)
    SELECT
        asset_id,
        trading_date,
        daily_high,
        daily_low,
        -- Fetch the previous day's high (PDH)
        LAG(daily_high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdh,
        -- Fetch the previous day's low (PDL)
        LAG(daily_low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdl
    FROM daily_extremes
),
session_takedown_data AS (
    -- Step 3: Join session data with the calculated PDH/PDL
    SELECT
        sv.trading_date,
        sv.asset_id,
        sv.session_name,
        sv.high,
        sv.low,
        -- Reuse the 7-State logic for bias
        CASE
            WHEN sv.session_type IN ('Bullish', 'Bearish') THEN sv.session_type
            WHEN sv.session_type = 'Consolidation' THEN sv.consolidation_subtype
            ELSE 'Other'
        END AS bias_7_state,
        pde.pdh,
        pde.pdl
    FROM us2000_session_views sv
    JOIN previous_day_extremes pde
        ON sv.asset_id = pde.asset_id
        AND sv.trading_date = pde.trading_date
    WHERE sv.asset_id = 'assets:US2000'
    -- Exclude the very first day, as PDH/PDL will be NULL
    AND pde.pdh IS NOT NULL
)
-- Final Takedown Calculation
INSERT INTO us2000_structural_takedowns (
    asset_id, trading_date, session_name, bias_7_state, is_pdh_break, is_pdl_break
)
SELECT
    asset_id,
    trading_date,
    session_name,
    bias_7_state,
    -- PDH Break: Session High > PDH
    (high > pdh) AS is_pdh_break,
    -- PDL Break: Session Low < PDL
    (low < pdl) AS is_pdl_break
FROM session_takedown_data;
-----------------------------------------
-- Fix stuture takedown
----------------------------
DROP TABLE IF EXISTS us2000_structural_takedowns CASCADE;
CREATE TABLE IF NOT EXISTS us2000_structural_takedowns (
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    session_name TEXT NOT NULL,
    bias_7_state TEXT NOT NULL,
    is_pdh_break BOOLEAN NOT NULL,
    is_pdl_break BOOLEAN NOT NULL,
    PRIMARY KEY (asset_id, trading_date, session_name)
);
TRUNCATE us2000_structural_takedowns;

-- Intermediate CTEs from your existing code (Calculate PDH/PDL)
WITH daily_extremes AS (
    SELECT
        asset_id,
        trading_date,
        MAX(high) AS daily_high,
        MIN(low) AS daily_low
    FROM asset_session_views
    GROUP BY 1, 2
),
previous_day_extremes AS (
    SELECT
        asset_id,
        trading_date,
        LAG(daily_high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdh,
        LAG(daily_low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdl
    FROM daily_extremes
),
session_takedown_data AS (
    -- Step 3: Join session data with the calculated PDH/PDL and include start_ts for sequencing
    SELECT
        sv.trading_date,
        sv.asset_id,
        sv.session_name,
        sv.start_ts, -- CRITICAL: Used for sequencing
        sv.high,
        sv.low,
        CASE
            WHEN sv.session_type IN ('Bullish', 'Bearish') THEN sv.session_type
            WHEN sv.session_type = 'Consolidation' THEN sv.consolidation_subtype
            ELSE 'Other'
        END AS bias_7_state,
        pde.pdh,
        pde.pdl
    FROM asset_session_views sv
    JOIN previous_day_extremes pde
        ON sv.asset_id = pde.asset_id
        AND sv.trading_date = pde.trading_date
    WHERE sv.asset_id = 'assets:US2000'
    AND pde.pdh IS NOT NULL
),
-- NEW CTE: Identify the FIRST session of the day to break each level
first_break_detection AS (
    SELECT
        *,
        -- Step 4A: Check if the session *actually* broke PDH/PDL
        (high > pdh) AS potential_pdh_break,
        (low < pdl) AS potential_pdl_break,        
        -- Step 4B: Identify the start_ts of the *first* PDH break on this trading_date
        MIN(CASE WHEN (high > pdh) THEN start_ts END)
            OVER (PARTITION BY asset_id, trading_date) AS first_pdh_break_ts,
        -- Step 4C: Identify the start_ts of the *first* PDL break on this trading_date
        MIN(CASE WHEN (low < pdl) THEN start_ts END)
            OVER (PARTITION BY asset_id, trading_date) AS first_pdl_break_ts            
    FROM session_takedown_data
)
-- Final Takedown Calculation
INSERT INTO us2000_structural_takedowns (
    asset_id, trading_date, session_name, bias_7_state, is_pdh_break, is_pdl_break
)
SELECT
    asset_id,
    trading_date,
    session_name,
    bias_7_state,
    -- Flag TRUE only if this session CAUSED the break
    (potential_pdh_break = TRUE AND start_ts = first_pdh_break_ts) AS is_pdh_break,
    (potential_pdl_break = TRUE AND start_ts = first_pdl_break_ts) AS is_pdl_break
FROM first_break_detection;
-------------
--FRS
----------
-- Metric 7: Failure-to-Hold Reversal Score (FHR) Table

DROP TABLE IF EXISTS us2000_fhr_reversal_score CASCADE;
CREATE TABLE IF NOT EXISTS us2000_fhr_reversal_score (
    asset_id TEXT NOT NULL,
    pattern_type TEXT NOT NULL,                     -- e.g., 'PDH_Break_to_Bearish'
    cs_name TEXT NOT NULL,                         -- The session (CS) that made the break
    cs_bias_7 TEXT NOT NULL,                       -- The 7-state bias of the breaking session
    total_pdh_breaks BIGINT NOT NULL,              -- Denominator D: Total times the pattern broke PDH
    reversal_count BIGINT NOT NULL,                -- Numerator N: Count of breaks resulting in Reversal Day Type
    fhr_score DOUBLE PRECISION NOT NULL,           -- Reversal_Count / Total_Breaks
    insight_label TEXT NOT NULL,
    PRIMARY KEY (asset_id, pattern_type, cs_name, cs_bias_7)
);
TRUNCATE us2000_fhr_reversal_score;

-- CTEs for FHR Calculation
WITH pdh_break_and_day_outcome AS (
    -- 1. Combine Structural Takedowns (Metric 4) with Final Day Outcome (Metric 5)
    SELECT
        t.asset_id,
        t.trading_date,
        t.session_name AS cs_name,
        t.bias_7_state AS cs_bias_7,
        t.is_pdh_break,
        d.daily_outcome_7
    FROM us2000_structural_takedowns t
    -- Re-defining 'daily_outcome_7_state' logic for completeness
    JOIN (
        SELECT
            trading_date,
            asset_id,
            CASE
                WHEN daily_type IN ('Bullish', 'Bearish') THEN daily_type
                WHEN daily_type = 'Consolidation' THEN consolidation_subtype
                ELSE 'Other'
            END AS daily_outcome_7
        FROM us2000_daily_views
    ) d
        ON t.trading_date = d.trading_date AND t.asset_id = d.asset_id
    WHERE t.is_pdh_break = TRUE -- Filter only for successful PDH breaks
)
, fhr_calculation AS (
    -- 2. Aggregate the counts and calculate the FHR Score
    SELECT
        p.asset_id,
        p.cs_name,
        p.cs_bias_7,
        COUNT(*) AS total_pdh_breaks, -- Denominator (D)
        SUM(CASE
            -- Numerator (N): Count of contradictory outcomes (Bearish or Bearish_Reversal)
            WHEN p.daily_outcome_7 IN ('Bearish', 'Bearish_Reversal') THEN 1
            ELSE 0
        END) AS reversal_count,
        -- Cast to NUMERIC before rounding
        ROUND((SUM(CASE
            WHEN p.daily_outcome_7 IN ('Bearish', 'Bearish_Reversal') THEN 1
            ELSE 0
        END)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 2) AS fhr_score
    FROM pdh_break_and_day_outcome p
    GROUP BY 1, 2, 3
)
-- INSERT INTO Table 7
INSERT INTO us2000_fhr_reversal_score (
    asset_id, pattern_type, cs_name, cs_bias_7, total_pdh_breaks, reversal_count, fhr_score, insight_label
)
SELECT
    f.asset_id,
    'PDH_Break_to_Bearish',
    f.cs_name,
    f.cs_bias_7,
    f.total_pdh_breaks,
    f.reversal_count,
    f.fhr_score,
    CASE
        WHEN f.fhr_score >= 60.00 THEN 'High Reversal Risk (PDH Failure)'
        WHEN f.fhr_score <= 30.00 THEN 'Strong Continuation Signal (Reliable Break)'
        ELSE 'Neutral/Watch for Confirmation'
    END AS insight_label
FROM fhr_calculation f
WHERE f.total_pdh_breaks > 5;