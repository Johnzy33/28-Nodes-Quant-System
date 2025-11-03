

DROP TABLE IF EXISTS us2000_fhr_reversal_score CASCADE;
-- Metric 7: Failure-to-Hold Reversal Score (FHR) Table
CREATE TABLE IF NOT EXISTS us2000_fhr_reversal_score (
    asset_id TEXT NOT NULL,
    pattern_type TEXT NOT NULL,                     -- 'PDH_Break_to_Bearish' or 'PDL_Break_to_Bullish'
    cs_name TEXT NOT NULL,                         -- The session (CS) that made the break
    cs_bias_7 TEXT NOT NULL,                       -- The 7-state bias of the breaking session
    total_breaks BIGINT NOT NULL,                  -- Denominator D: Total times the structural break occurred
    reversal_count BIGINT NOT NULL,                -- Numerator N: Count of breaks resulting in Reversal Day Type
    fhr_score DOUBLE PRECISION NOT NULL,           -- Reversal_Count / Total_Breaks
    insight_label TEXT NOT NULL,
    PRIMARY KEY (asset_id, pattern_type, cs_name, cs_bias_7)
);
TRUNCATE us2000_fhr_reversal_score;

-- CTE 1: Define the Day Outcome Logic (for consistency)
WITH daily_outcome_7_state AS (
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
-- CTE 2: Get all relevant Structural Breaks and join to Session Bias and Day Outcome
structural_break_data AS (
    SELECT
        t.asset_id,
        t.trading_date,
        t.breaker_session AS cs_name,
        t.prior_level,  -- 'High' (PDH) or 'Low' (PDL)
        -- Get 7-State bias for the breaking session
        CASE
            WHEN sv.session_type IN ('Bullish', 'Bearish') THEN sv.session_type
            WHEN sv.session_type = 'Consolidation' THEN sv.consolidation_subtype
            ELSE 'Other'
        END AS cs_bias_7,
        d.daily_outcome_7
    FROM us2000_takedown_events t
    JOIN asset_session_views sv
        ON t.asset_id = sv.asset_id
        AND t.trading_date = sv.trading_date
        AND t.breaker_session = sv.session_name
    JOIN daily_outcome_7_state d
        ON t.asset_id = d.asset_id
        AND t.trading_date = d.trading_date
    WHERE
        t.asset_id = 'assets:US2000'
        AND t.prior_session = 'PrevDay-Daily' -- Only PDH/PDL breaks
),
-- CTE 3: Calculate FHR for PDH Break Failure (Bullish momentum reversal)
fhr_pdh_failure AS (
    SELECT
        asset_id,
        'PDH_Break_to_Bearish' AS pattern_type,
        cs_name,
        cs_bias_7,
        COUNT(*) AS total_breaks, -- D
        SUM(CASE
            -- Reversal condition for PDH break: Final outcome is Bearish
            WHEN daily_outcome_7 IN ('Bearish', 'Bearish_Reversal') THEN 1
            ELSE 0
        END) AS reversal_count, -- N
        ROUND((SUM(CASE
            WHEN daily_outcome_7 IN ('Bearish', 'Bearish_Reversal') THEN 1
            ELSE 0
        END)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 2) AS fhr_score
    FROM structural_break_data
    WHERE prior_level = 'High'
    GROUP BY 1, 2, 3, 4
),
-- CTE 4: Calculate FHR for PDL Break Failure (Bearish momentum reversal)
fhr_pdl_failure AS (
    SELECT
        asset_id,
        'PDL_Break_to_Bullish' AS pattern_type,
        cs_name,
        cs_bias_7,
        COUNT(*) AS total_breaks, -- D
        SUM(CASE
            -- Reversal condition for PDL break: Final outcome is Bullish
            WHEN daily_outcome_7 IN ('Bullish', 'Bullish_Reversal') THEN 1
            ELSE 0
        END) AS reversal_count, -- N
        ROUND((SUM(CASE
            WHEN daily_outcome_7 IN ('Bullish', 'Bullish_Reversal') THEN 1
            ELSE 0
        END)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 2) AS fhr_score
    FROM structural_break_data
    WHERE prior_level = 'Low'
    GROUP BY 1, 2, 3, 4
)
-- INSERT INTO Table 7 (Combining both PDH and PDL results)
INSERT INTO us2000_fhr_reversal_score (
    asset_id, pattern_type, cs_name, cs_bias_7, total_breaks, reversal_count, fhr_score, insight_label
)
SELECT
    f.asset_id,
    f.pattern_type,
    f.cs_name,
    f.cs_bias_7,
    f.total_breaks,
    f.reversal_count,
    f.fhr_score,
    CASE
        WHEN f.fhr_score >= 60.00 THEN 'High Reversal Risk'
        WHEN f.fhr_score <= 30.00 THEN 'Strong Continuation Signal'
        ELSE 'Neutral/Watch'
    END AS insight_label
FROM fhr_pdh_failure f
WHERE f.total_breaks > 10 -- Minimum sample size
UNION ALL
SELECT
    f.asset_id,
    f.pattern_type,
    f.cs_name,
    f.cs_bias_7,
    f.total_breaks,
    f.reversal_count,
    f.fhr_score,
    CASE
        WHEN f.fhr_score >= 60.00 THEN 'High Reversal Risk'
        WHEN f.fhr_score <= 30.00 THEN 'Strong Continuation Signal'
        ELSE 'Neutral/Watch'
    END AS insight_label
FROM fhr_pdl_failure f
WHERE f.total_breaks > 10; -- Minimum sample size

-----------------------------------------------------
-- FRH to add PS1
-----------------------------





DROP TABLE IF EXISTS us2000_fhr_reversal_score CASCADE;
CREATE TABLE IF NOT EXISTS us2000_fhr_reversal_score (
    asset_id TEXT NOT NULL,
    pattern_type TEXT NOT NULL,
    ps_name TEXT NOT NULL,                    -- New field: Name of the session
    ps_bias_7 TEXT NOT NULL,                    -- New field: Type of the session                     -- 'PDH_Break_to_Bearish' or 'PDL_Break_to_Bullish'
    cs_name TEXT NOT NULL,                         -- The session (CS) that made the break
    cs_bias_7 TEXT NOT NULL,                       -- The 7-state bias of the breaking session
    total_breaks BIGINT NOT NULL,                  -- Denominator D: Total times the structural break occurred
    reversal_count BIGINT NOT NULL,                -- Numerator N: Count of breaks resulting in Reversal Day Type
    fhr_score DOUBLE PRECISION NOT NULL,           -- Reversal_Count / Total_Breaks
    insight_label TEXT NOT NULL,
    PRIMARY KEY (asset_id, pattern_type, cs_name, cs_bias_7)
);
TRUNCATE us2000_fhr_reversal_score;

-- CTE 1: Define the Day Outcome Logic (for consistency)
WITH daily_outcome_7_state AS (
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
-- CTE 2: Get all relevant Structural Breaks and join to Session Bias and Day Outcome
structural_break_data AS (
    SELECT
        t.asset_id,
        t.trading_date,
        t.breaker_session AS cs_name,
        t.prior_level,  -- 'High' (PDH) or 'Low' (PDL)
        -- Get 7-State bias for the breaking session
        CASE
            WHEN sv.session_type IN ('Bullish', 'Bearish') THEN sv.session_type
            WHEN sv.session_type = 'Consolidation' THEN sv.consolidation_subtype
            ELSE 'Other'
        END AS cs_bias_7,
        d.daily_outcome_7,
        sv.session_name AS ps_name,         -- New field: Name of the session
        sv.session_type AS ps_bias_7          -- New field: Type of the session
    FROM us2000_takedown_events t
    JOIN asset_session_views sv
        ON t.asset_id = sv.asset_id
        AND t.trading_date = sv.trading_date
        AND t.breaker_session = sv.session_name
    JOIN daily_outcome_7_state d
        ON t.asset_id = d.asset_id
        AND t.trading_date = d.trading_date
    WHERE
        t.asset_id = 'assets:US2000'
        AND t.prior_session = 'PrevDay-Daily' -- Only PDH/PDL breaks
),
-- CTE 3: Calculate FHR for PDH Break Failure (Bullish momentum reversal)
fhr_pdh_failure AS (
    SELECT
        asset_id,
        'PDH_Break_to_Bearish' AS pattern_type,
        cs_name,
        cs_bias_7,
        ps_name,
        ps_bias_7,
        COUNT(*) AS total_breaks, -- D
        SUM(CASE
            -- Reversal condition for PDH break: Final outcome is Bearish
            WHEN daily_outcome_7 IN ('Bearish', 'Bearish_Reversal') THEN 1
            ELSE 0
        END) AS reversal_count, -- N
        ROUND((SUM(CASE
            WHEN daily_outcome_7 IN ('Bearish', 'Bearish_Reversal') THEN 1
            ELSE 0
        END)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 2) AS fhr_score
    FROM structural_break_data
    WHERE prior_level = 'High'
    GROUP BY 1, 2, 3, 4, 5, 6
),
-- CTE 4: Calculate FHR for PDL Break Failure (Bearish momentum reversal)
fhr_pdl_failure AS (
    SELECT
        asset_id,
        'PDL_Break_to_Bullish' AS pattern_type,
        cs_name,
        cs_bias_7,
        ps_name,
        ps_bias_7,
        COUNT(*) AS total_breaks, -- D
        SUM(CASE
            -- Reversal condition for PDL break: Final outcome is Bullish
            WHEN daily_outcome_7 IN ('Bullish', 'Bullish_Reversal') THEN 1
            ELSE 0
        END) AS reversal_count, -- N
        ROUND((SUM(CASE
            WHEN daily_outcome_7 IN ('Bullish', 'Bullish_Reversal') THEN 1
            ELSE 0
        END)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 2) AS fhr_score
    FROM structural_break_data
    WHERE prior_level = 'Low'
    GROUP BY 1, 2, 3, 4, 5, 6
)
-- INSERT INTO Table 7 (Combining both PDH and PDL results)
INSERT INTO us2000_fhr_reversal_score (
    asset_id, pattern_type, cs_name, cs_bias_7, ps_name, ps_bias_7, total_breaks, reversal_count, fhr_score, insight_label
)
SELECT
    f.asset_id,
    f.pattern_type,
    f.cs_name,
    f.cs_bias_7,
    f.ps_name,
    f.ps_bias_7,
    f.total_breaks,
    f.reversal_count,
    f.fhr_score,
    CASE
        WHEN f.fhr_score >= 60.00 THEN 'High Reversal Risk'
        WHEN f.fhr_score <= 30.00 THEN 'Strong Continuation Signal'
        ELSE 'Neutral/Watch'
    END AS insight_label
FROM fhr_pdh_failure f
--WHERE f.total_breaks > 5 -- Minimum sample size
UNION ALL
SELECT
    f.asset_id,
    f.pattern_type,
    f.cs_name,
    f.cs_bias_7,
    f.ps_name,
    f.ps_bias_7,
    f.total_breaks,
    f.reversal_count,
    f.fhr_score,
    CASE
        WHEN f.fhr_score >= 60.00 THEN 'High Reversal Risk'
        WHEN f.fhr_score <= 30.00 THEN 'Strong Continuation Signal'
        ELSE 'Neutral/Watch'
    END AS insight_label
FROM fhr_pdl_failure f
WHERE f.total_breaks > 10; -- Minimum sample size

------------------------ simplified
------------------------



-- Final Metric: Unified Context Score (UCS)
-- Combines PCS (Edge), TCS (Stability), CVI (Risk), and FHR (Reversal Override)

CREATE TABLE IF NOT EXISTS us2000_unified_context_score (
    asset_id TEXT NOT NULL,
    ps_name TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    ps_bias_3 TEXT NOT NULL,
    cs_bias_3 TEXT NOT NULL,
    -- Final Calculated Score
    ucs_score DOUBLE PRECISION NOT NULL,
    ucs_signal TEXT NOT NULL,
    -- Core Components (for validation)
    pcs_base DOUBLE PRECISION NOT NULL,
    tcs_bullish_score DOUBLE PRECISION,
    tcs_bearish_score DOUBLE PRECISION,
    cvi_score DOUBLE PRECISION,
    fhr_reversal_score DOUBLE PRECISION,
    PRIMARY KEY (asset_id, ps_name, cs_name, ps_bias_3, cs_bias_3)
);
TRUNCATE us2000_unified_context_score;

-- CTE 1: Pivot PCS Data to get Bullish and Bearish PCS for every transition (Base UCS key)
WITH pcs_directional AS (
    SELECT
        asset_id,
        ps_name,
        cs_name,
        ps_bias_3,
        cs_bias_3,
        MAX(CASE WHEN daily_outcome_7 IN ('Bullish', 'Bullish_Reversal') THEN pcs_score END) AS pcs_bullish,
        MAX(CASE WHEN daily_outcome_7 IN ('Bearish', 'Bearish_Reversal') THEN pcs_score END) AS pcs_bearish
    FROM us2000_predictive_confidence_score
    GROUP BY 1, 2, 3, 4, 5
),
-- CTE 2: Simplify FHR scores (Max score per PS->CS transition + pattern type)
fhr_simplified AS (
    SELECT 
        asset_id,cs_name, pattern_type,
        MAX(fhr_score) AS max_fhr_score 
    FROM us2000_fhr_reversal_score
    GROUP BY 1, 2, 3
),
-- CTE 3 (NEW): Simplify TCS scores (Max score per PS + trend direction)
tcs_simplified AS (
    SELECT
        asset_id,
        ps1_name, -- This is the 'PS' session name
        trend_direction,
        MAX(tcs_score) AS max_tcs_score
    FROM us2000_trend_stability_score
    GROUP BY 1, 2, 3
),
-- CTE 4 (NEW): Simplify CVI scores (Max score per PS->CS transition)
cvi_simplified AS (
    SELECT
        asset_id,
        ps_name,
        cs_name,
        MAX(cvi_score) AS max_cvi_score
    FROM us2000_consolidation_volatility_index
    GROUP BY 1, 2, 3
),
-- CTE 5: Join all Contextual Scores using the PS->CS transition
full_context_join AS (
    SELECT
        pd.asset_id, pd.ps_name, pd.cs_name, pd.ps_bias_3, pd.cs_bias_3,
        pd.pcs_bullish, pd.pcs_bearish,    
        -- 1. TCS (Trend Stability)
        tcs_b.max_tcs_score AS tcs_bullish,
        tcs_r.max_tcs_score AS tcs_bearish,
        -- 2. CVI (Volatility Risk)
        cvi.max_cvi_score AS cvi_score,    
        -- 3. FHR (Reversal Override)
        fhr_b.max_fhr_score AS fhr_bullish_reversal,
        fhr_r.max_fhr_score AS fhr_bearish_reversal
    FROM pcs_directional pd
    -- Join TCS - Bullish Direction (Using simplified CTE)
    LEFT JOIN tcs_simplified tcs_b
        ON pd.asset_id = tcs_b.asset_id
        AND pd.ps_name = tcs_b.ps1_name
        AND tcs_b.trend_direction = 'Bullish'
    -- Join TCS - Bearish Direction (Using simplified CTE)
    LEFT JOIN tcs_simplified tcs_r
        ON pd.asset_id = tcs_r.asset_id
        AND pd.ps_name = tcs_r.ps1_name
        AND tcs_r.trend_direction = 'Bearish'
    -- Join CVI (Using simplified CTE)
    LEFT JOIN cvi_simplified cvi
        ON pd.asset_id = cvi.asset_id
        AND pd.ps_name = cvi.ps_name
        AND pd.cs_name = cvi.cs_name    
    -- Join FHR (Using simplified CTE)
    LEFT JOIN fhr_simplified fhr_b
        ON pd.asset_id = fhr_b.asset_id
        AND pd.cs_name = fhr_b.cs_name
        AND fhr_b.pattern_type = 'PDL_Break_to_Bullish'
        
    LEFT JOIN fhr_simplified fhr_r
        ON pd.asset_id = fhr_r.asset_id
        AND pd.cs_name = fhr_r.cs_name
        AND fhr_r.pattern_type = 'PDH_Break_to_Bearish'
),
-- CTE 6: Calculate the necessary components (Base, Multiplier, Override Flag)
final_ucs_calculation AS (
    SELECT
        fc.*,
        -- 1. Determine the Base Signed PCS Score
        CASE
            WHEN COALESCE(pcs_bullish, 0.0) >= COALESCE(pcs_bearish, 0.0)
                THEN COALESCE(pcs_bullish, 1.0)
            ELSE -COALESCE(pcs_bearish, 1.0)
        END AS ucs_base,
        -- 2. Calculate Directional Confidence Multiplier (C_Mult)
        (
            1.0 -- Initialize Multiplier at 1.0            
            -- TCS Bonus/Penalty (Inverse Logic)
            + CASE
                WHEN COALESCE(pcs_bullish, 0.0) >= COALESCE(pcs_bearish, 0.0) THEN -- Bullish Direction
                    CASE
                        WHEN tcs_bullish >= 65.0 THEN 0.25
                        WHEN tcs_bullish <= 20.0 THEN -0.50
                        WHEN tcs_bullish <= 40.0 THEN -0.30
                        ELSE 0.0
                    END
                ELSE -- Bearish Direction
                    CASE
                        WHEN tcs_bearish >= 65.0 THEN 0.25
                        WHEN tcs_bearish <= 20.0 THEN -0.50
                        WHEN tcs_bearish <= 40.0 THEN -0.30
                        ELSE 0.0
                    END
            END            
            -- CVI Penalty/Bonus (Inverse Logic)
            + CASE
                WHEN cvi_score >= 70.0 THEN -0.20
                WHEN cvi_score <= 15.0 THEN 0.35
                ELSE 0.0
            END
        ) AS confidence_multiplier,        
        -- 3. FHR Override Flag
        CASE
            WHEN fhr_bearish_reversal >= 60.0 THEN 'BEARISH_REVERSAL_OVERRIDE'
            WHEN fhr_bullish_reversal >= 60.0 THEN 'BULLISH_REVERSAL_OVERRIDE'
            ELSE NULL
        END AS fhr_override_flag
    FROM full_context_join fc 
)
-- Final Selection and UCS Score Application (Calculation is performed here)
INSERT INTO us2000_unified_context_score (
    asset_id, ps_name, cs_name, ps_bias_3, cs_bias_3,
    ucs_score, ucs_signal, pcs_base, tcs_bullish_score, tcs_bearish_score, cvi_score, fhr_reversal_score
)
SELECT
    fuc.asset_id,
    fuc.ps_name,
    fuc.cs_name,
    fuc.ps_bias_3,
    fuc.cs_bias_3,
    -- 1. Final UCS Score: Calculated on the fly
    CASE
        WHEN fuc.fhr_override_flag = 'BEARISH_REVERSAL_OVERRIDE' THEN -2.0
        WHEN fuc.fhr_override_flag = 'BULLISH_REVERSAL_OVERRIDE' THEN 2.0
        ELSE fuc.ucs_base * fuc.confidence_multiplier
    END AS ucs_score,    
    -- 2. Final UCS Signal Label: Calculation repeated using the same logic for score
    CASE
        WHEN fuc.fhr_override_flag = 'BEARISH_REVERSAL_OVERRIDE' THEN 'Extreme Confidence SHORT (FADE)'
        WHEN fuc.fhr_override_flag = 'BULLISH_REVERSAL_OVERRIDE' THEN 'Extreme Confidence LONG (FADE)'
        -- Recalculate the score inline to check thresholds
        WHEN (CASE
            WHEN fuc.fhr_override_flag = 'BEARISH_REVERSAL_OVERRIDE' THEN -2.0
            WHEN fuc.fhr_override_flag = 'BULLISH_REVERSAL_OVERRIDE' THEN 2.0
            ELSE fuc.ucs_base * fuc.confidence_multiplier
        END) >= 1.5 THEN 'High Confidence LONG'
        WHEN (CASE
            WHEN fuc.fhr_override_flag = 'BEARISH_REVERSAL_OVERRIDE' THEN -2.0
            WHEN fuc.fhr_override_flag = 'BULLISH_REVERSAL_OVERRIDE' THEN 2.0
            ELSE fuc.ucs_base * fuc.confidence_multiplier
        END) <= -1.5 THEN 'High Confidence SHORT'
        WHEN ABS(CASE
            WHEN fuc.fhr_override_flag = 'BEARISH_REVERSAL_OVERRIDE' THEN -2.0
            WHEN fuc.fhr_override_flag = 'BULLISH_REVERSAL_OVERRIDE' THEN 2.0
            ELSE fuc.ucs_base * fuc.confidence_multiplier
        END) BETWEEN 1.0 AND 1.5 THEN 'Moderate Confidence'
        ELSE 'No Edge / High Conflict'
    END AS ucs_signal,    
    fuc.ucs_base,
    fuc.tcs_bullish,
    fuc.tcs_bearish,
    fuc.cvi_score,
    COALESCE(fuc.fhr_bullish_reversal, fuc.fhr_bearish_reversal)
FROM final_ucs_calculation fuc;
