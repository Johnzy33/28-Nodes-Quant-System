---------------------
-- Failure to Hold Reversal Score (FHR)
---------------------

DROP TABLE IF EXISTS asset_fhr_reversal_score CASCADE;
-- Metric 7: Failure-to-Hold Reversal Score (FHR) Table
CREATE TABLE IF NOT EXISTS asset_fhr_reversal_score (
    asset_id TEXT NOT NULL,
    pattern_type TEXT NOT NULL,                     -- 'PDH_Break_to_Bearish' or 'PDL_Break_to_Bullish'
    cs_name TEXT NOT NULL,
    cs_bias_7 TEXT NOT NULL,
    total_breaks BIGINT NOT NULL,
    reversal_count BIGINT NOT NULL,
    fhr_score DOUBLE PRECISION NOT NULL,           -- Reversal_Count / Total_Breaks
    reversal_dates DATE[] NOT NULL,                -- NEW: Array of dates where the reversal occurred
    insight_label TEXT NOT NULL,
    PRIMARY KEY (asset_id, pattern_type, cs_name, cs_bias_7)
);
TRUNCATE asset_fhr_reversal_score;

-- =================================================================================
-- METRIC 7: FAILURE-TO-HOLD REVERSAL SCORE (FHR) - GENERALIZED WITH REVERSAL DATES
-- =================================================================================
WITH daily_outcome_7_state AS (
    -- 1. Define the Daily Outcome Logic (7-state)
    SELECT
        trading_date,
        asset_id,
        CASE
            WHEN day_type IN ('Bullish', 'Bearish') THEN day_type
            WHEN day_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other'
        END AS daily_outcome_7
    FROM asset_daily_views
),
structural_break_data AS (
    -- 2. Get all relevant Structural Breaks (PDH/PDL only) and join to 7-Bias and Day Outcome
    SELECT
        t.asset_id,
        t.trading_date,
        t.cs_session AS cs_name,
        t.prior_level,
        -- Get 7-State bias for the breaking session
        CASE
            WHEN sv.session_type IN ('Bullish', 'Bearish') THEN sv.session_type
            WHEN sv.session_type = 'Consolidation' THEN sv.consolidation_subtype
            ELSE 'Other'
        END AS cs_bias_7,
        d.daily_outcome_7,
        -- Define the Reversal Flag (1 if a reversal day, 0 otherwise)
        CASE
            -- Reversal: PDH Break (High) -> Bearish Day
            WHEN t.prior_level = 'High' AND d.daily_outcome_7 IN ('Bearish', 'Bearish_Reversal') THEN 1
            -- Reversal: PDL Break (Low) -> Bullish Day
            WHEN t.prior_level = 'Low' AND d.daily_outcome_7 IN ('Bullish', 'Bullish_Reversal') THEN 1
            ELSE 0
        END AS is_reversal
    FROM asset_takedown_events t
    JOIN asset_session_views sv
        ON t.asset_id = sv.asset_id
        AND t.trading_date = sv.trading_date
        AND t.cs_session = sv.session_name
    JOIN daily_outcome_7_state d
        ON t.asset_id = d.asset_id
        AND t.trading_date = d.trading_date
    WHERE
        t.ps1_session = 'PrevDay-Daily' -- Only PDH/PDL breaks
),
fhr_scores AS (
    -- 3. Calculate FHR and Aggregate Reversal Dates
    SELECT
        asset_id,
        cs_name,
        cs_bias_7,
        prior_level,
        CASE
            WHEN prior_level = 'High' THEN 'PDH_Break_to_Bearish'
            WHEN prior_level = 'Low' THEN 'PDL_Break_to_Bullish'
            ELSE NULL
        END AS pattern_type,
        
        COUNT(*) AS total_breaks, -- D
        SUM(is_reversal) AS reversal_count, -- N       
        -- Collect the dates where is_reversal = 1
        ARRAY_AGG(trading_date ORDER BY trading_date) FILTER (WHERE is_reversal = 1) AS reversal_dates 
        
    FROM structural_break_data
    GROUP BY 1, 2, 3, 4, 5
)
-- 4. FINAL INSERT: Calculate Score and Insert
INSERT INTO asset_fhr_reversal_score (
    asset_id, pattern_type, cs_name, cs_bias_7, total_breaks, reversal_count, fhr_score, reversal_dates, insight_label
)
SELECT
    f.asset_id,
    f.pattern_type,
    f.cs_name,
    f.cs_bias_7,
    f.total_breaks,
    f.reversal_count,
    ROUND((f.reversal_count::NUMERIC / NULLIF(f.total_breaks, 0) * 100), 2) AS fhr_score,
    f.reversal_dates, -- The new date array
    CASE
        WHEN (f.reversal_count::NUMERIC / NULLIF(f.total_breaks, 0) * 100) >= 60.00 THEN 'High Reversal Risk'
        WHEN (f.reversal_count::NUMERIC / NULLIF(f.total_breaks, 0) * 100) <= 30.00 THEN 'Strong Continuation Signal'
        ELSE 'Neutral/Watch'
    END AS insight_label
FROM fhr_scores f
WHERE f.total_breaks > 0;
