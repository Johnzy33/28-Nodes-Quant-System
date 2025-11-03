-- Final Metric: Unified Context Score (UCS)
-- Combines PCS (Edge), TCS (Stability), CVI (Risk), and FHR (Reversal Override)
-- UCS is a signed score (+ve for Bullish, -ve for Bearish).

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

-- CTE 1: Pivot PCS Data to get Bullish and Bearish PCS for every transition
WITH pcs_directional AS (
    SELECT
        asset_id,
        ps_name,
        cs_name,
        ps_bias_3,
        cs_bias_3,
        -- Get the highest Bullish Lift (Bullish/Bullish_Reversal)
        MAX(CASE WHEN daily_outcome_7 IN ('Bullish', 'Bullish_Reversal') THEN pcs_score END) AS pcs_bullish,
        -- Get the highest Bearish Lift (Bearish/Bearish_Reversal)
        MAX(CASE WHEN daily_outcome_7 IN ('Bearish', 'Bearish_Reversal') THEN pcs_score END) AS pcs_bearish
    FROM us2000_predictive_confidence_score
    GROUP BY 1, 2, 3, 4, 5
),
-- CTE 2: Join all Contextual Scores using the PS->CS transition
full_context_join AS (
    SELECT
        pd.asset_id,
        pd.ps_name,
        pd.cs_name,
        pd.ps_bias_3,
        pd.cs_bias_3,
        pd.pcs_bullish,
        pd.pcs_bearish,        
        -- 1. TCS (Trend Stability): PS1 in TCS matches PS in PCS
        tcs_b.tcs_score AS tcs_bullish, -- TCS score for Bullish continuation (from a prior Bullish Trend)
        tcs_r.tcs_score AS tcs_bearish, -- TCS score for Bearish continuation (from a prior Bearish Trend)        
        -- 2. CVI (Volatility Risk): PS in CVI matches PS in PCS
        cvi.cvi_score,        
        -- 3. FHR (Reversal Override): Join on PS->CS
        fhr_b.fhr_score AS fhr_bullish_reversal, -- FHR score for a Bullish Reversal (from PDL break)
        fhr_r.fhr_score AS fhr_bearish_reversal  -- FHR score for a Bearish Reversal (from PDH break)
    FROM pcs_directional pd    
    -- Join TCS - Bullish Direction (Trend_Direction = 'Bullish')
    LEFT JOIN us2000_trend_stability_score tcs_b
        ON pd.asset_id = tcs_b.asset_id
        AND pd.ps_name = tcs_b.ps1_name -- The immediate prior session
        AND tcs_b.trend_direction = 'Bullish' -- Trend continuity logic    
    -- Join TCS - Bearish Direction (Trend_Direction = 'Bearish')
    LEFT JOIN us2000_trend_stability_score tcs_r
        ON pd.asset_id = tcs_r.asset_id
        AND pd.ps_name = tcs_r.ps1_name
        AND tcs_r.trend_direction = 'Bearish' -- Trend continuity logic    
    -- Join CVI 
    LEFT JOIN us2000_consolidation_volatility_index cvi
        ON pd.asset_id = cvi.asset_id
        AND pd.ps_name = cvi.ps_name
        AND pd.cs_name = cvi.cs_name        
    -- Join FHR - Bullish Reversal (Fails PDL Break -> Bullish Outcome)
    LEFT JOIN us2000_fhr_reversal_score fhr_b
        ON pd.asset_id = fhr_b.asset_id
        AND pd.ps_name = fhr_b.ps_name -- NEW ROBUST JOIN KEY
        AND pd.cs_name = fhr_b.cs_name
        AND fhr_b.pattern_type = 'PDL_Break_to_Bullish'        
    -- Join FHR - Bearish Reversal (Fails PDH Break -> Bearish Outcome)
    LEFT JOIN us2000_fhr_reversal_score fhr_r
        ON pd.asset_id = fhr_r.asset_id
        AND pd.ps_name = fhr_r.ps_name -- NEW ROBUST JOIN KEY
        AND pd.cs_name = fhr_r.cs_name
        AND fhr_r.pattern_type = 'PDH_Break_to_Bearish'
),
-- CTE 3: Calculate the Final Signed UCS Score
final_ucs_calculation AS (
    SELECT
        fc.*,
        -- 1. Determine the Base Signed PCS Score
        CASE
            -- Default to 1.0 if both are NULL/0, but typically one is > 1.0
            WHEN COALESCE(pcs_bullish, 0.0) >= COALESCE(pcs_bearish, 0.0)
                THEN COALESCE(pcs_bullish, 1.0) -- Stronger is Bullish, use positive score
            ELSE -COALESCE(pcs_bearish, 1.0) -- Stronger is Bearish, use negative score
        END AS ucs_base,
        -- 2. Calculate Directional Confidence Multiplier (C_Mult)
        (
            -- Initialize Multiplier at 1.0
            1.0            
            -- TCS Bonus/Penalty (Inverse Logic)
            + CASE
                -- Bullish Direction is Stronger (UCS_Base is positive)
                WHEN COALESCE(pcs_bullish, 0.0) >= COALESCE(pcs_bearish, 0.0) THEN
                    CASE
                        WHEN tcs_bullish >= 65.0 THEN 0.25 -- TCS Bonus: High Stability
                        WHEN tcs_bullish <= 20.0 THEN -0.50 -- TCS Inverse Penalty: Extreme Failure Risk
                        WHEN tcs_bullish <= 40.0 THEN -0.30 -- TCS Fragility Penalty: Low Stability
                        ELSE 0.0
                    END
                -- Bearish Direction is Stronger (UCS_Base is negative)
                ELSE 
                    CASE
                        WHEN tcs_bearish >= 65.0 THEN 0.25 -- TCS Bonus: High Stability
                        WHEN tcs_bearish <= 20.0 THEN -0.50 -- TCS Inverse Penalty: Extreme Failure Risk
                        WHEN tcs_bearish <= 40.0 THEN -0.30 -- TCS Fragility Penalty: Low Stability
                        ELSE 0.0
                    END
            END            
            -- CVI Penalty/Bonus (Inverse Logic)
            + CASE
                WHEN cvi_score >= 70.0 THEN -0.20 -- CVI Penalty: High Break Risk
                WHEN cvi_score <= 15.0 THEN 0.35 -- CVI Inverse Bonus: High Containment Confidence
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
-- Final Selection and UCS Score Application
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
    -- Final UCS Score: Apply Override or Multiplier
    CASE
        WHEN fuc.fhr_override_flag = 'BEARISH_REVERSAL_OVERRIDE' THEN -2.0 -- Fixed strong negative score
        WHEN fuc.fhr_override_flag = 'BULLISH_REVERSAL_OVERRIDE' THEN 2.0  -- Fixed strong positive score
        ELSE fuc.ucs_base * fuc.confidence_multiplier
    END AS ucs_score,    
    -- Final UCS Signal Label
    CASE
        WHEN fuc.fhr_override_flag = 'BEARISH_REVERSAL_OVERRIDE' THEN 'Extreme Confidence SHORT (FADE)'
        WHEN fuc.fhr_override_flag = 'BULLISH_REVERSAL_OVERRIDE' THEN 'Extreme Confidence LONG (FADE)'
        WHEN fuc.ucs_base * fuc.confidence_multiplier >= 1.5 THEN 'High Confidence LONG'
        WHEN fuc.ucs_base * fuc.confidence_multiplier <= -1.5 THEN 'High Confidence SHORT'
        WHEN ABS(fuc.ucs_base * fuc.confidence_multiplier) BETWEEN 1.0 AND 1.5 THEN 'Moderate Confidence'
        ELSE 'No Edge / High Conflict'
    END AS ucs_signal,    
    fuc.ucs_base,
    fuc.tcs_bullish,
    fuc.tcs_bearish,
    fuc.cvi_score,
    COALESCE(fuc.fhr_bullish_reversal, fuc.fhr_bearish_reversal) -- Store the relevant FHR score
FROM final_ucs_calculation fuc;
-----------------------------
--- ucs fix
---------------------

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

-- CTE 1: Pivot PCS Data to get Bullish and Bearish PCS for every transition
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
-- CTE 2: Simplify FHR scores by removing cs_bias_7 from the key (prevents duplicates)
fhr_simplified AS (
    SELECT 
        asset_id,
        ps_name,
        cs_name,
        pattern_type,
        MAX(fhr_score) AS max_fhr_score 
    FROM us2000_fhr_reversal_score
    GROUP BY 1, 2, 3, 4
),
-- CTE 3: Join all Contextual Scores using the PS->CS transition
full_context_join AS (
    SELECT
        pd.asset_id,
        pd.ps_name,
        pd.cs_name,
        pd.ps_bias_3,
        pd.cs_bias_3,
        pd.pcs_bullish,
        pd.pcs_bearish,        
        -- 1. TCS (Trend Stability)
        tcs_b.tcs_score AS tcs_bullish,
        tcs_r.tcs_score AS tcs_bearish,        
        -- 2. CVI (Volatility Risk)
        cvi.cvi_score,        
        -- 3. FHR (Reversal Override)
        fhr_b.max_fhr_score AS fhr_bullish_reversal,
        fhr_r.max_fhr_score AS fhr_bearish_reversal
    FROM pcs_directional pd
    
    LEFT JOIN us2000_trend_stability_score tcs_b
        ON pd.asset_id = tcs_b.asset_id AND pd.ps_name = tcs_b.ps1_name AND tcs_b.trend_direction = 'Bullish'
    
    LEFT JOIN us2000_trend_stability_score tcs_r
        ON pd.asset_id = tcs_r.asset_id AND pd.ps_name = tcs_r.ps1_name AND tcs_r.trend_direction = 'Bearish'
    
    LEFT JOIN us2000_consolidation_volatility_index cvi
        ON pd.asset_id = cvi.asset_id AND pd.ps_name = cvi.ps_name AND pd.cs_name = cvi.cs_name
        
    LEFT JOIN fhr_simplified fhr_b
        ON pd.asset_id = fhr_b.asset_id AND pd.ps_name = fhr_b.ps_name AND pd.cs_name = fhr_b.cs_name AND fhr_b.pattern_type = 'PDL_Break_to_Bullish'
        
    LEFT JOIN fhr_simplified fhr_r
        ON pd.asset_id = fhr_r.asset_id AND pd.ps_name = fhr_r.ps_name AND pd.cs_name = fhr_r.cs_name AND fhr_r.pattern_type = 'PDH_Break_to_Bearish'
),
-- CTE 4: Calculate the necessary components (Base, Multiplier, Override Flag)
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
-- Final Selection and UCS Score Application (Calculation is performed here, ensuring column visibility)
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

----------------------------
-- condens main
---------------------


-- Final Metric: Unified Context Score (UCS)
-- Combines PCS (Edge), TCS (Stability), CVI (Risk), and FHR (Reversal Override)

CREATE TABLE IF NOT EXISTS us2000_unified_context_score (
    asset_id TEXT NOT NULL,
    ps_name TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    ps_bias_3 TEXT NOT NULL,
    cs_bias_3 TEXT NOT NULL,
    ucs_score DOUBLE PRECISION NOT NULL,
    ucs_signal TEXT NOT NULL,
    pcs_base DOUBLE PRECISION NOT NULL,
    tcs_bullish_score DOUBLE PRECISION,
    tcs_bearish_score DOUBLE PRECISION,
    cvi_score DOUBLE PRECISION,
    fhr_reversal_score DOUBLE PRECISION,
    PRIMARY KEY (asset_id, ps_name, cs_name, ps_bias_3, cs_bias_3)
);
TRUNCATE us2000_unified_context_score;

-- CTE 1: Pivot PCS Data
WITH pcs_directional AS (
    SELECT
        asset_id, ps_name, cs_name, ps_bias_3, cs_bias_3,
        MAX(CASE WHEN daily_outcome_7 IN ('Bullish', 'Bullish_Reversal') THEN pcs_score END) AS pcs_bullish,
        MAX(CASE WHEN daily_outcome_7 IN ('Bearish', 'Bearish_Reversal') THEN pcs_score END) AS pcs_bearish
    FROM us2000_predictive_confidence_score
    GROUP BY 1, 2, 3, 4, 5
),
-- CTE 2: Simplify FHR scores (prevents duplicates)
fhr_simplified AS (
    SELECT 
        asset_id, ps_name, cs_name, pattern_type,
        MAX(fhr_score) AS max_fhr_score 
    FROM us2000_fhr_reversal_score
    GROUP BY 1, 2, 3, 4
),
-- CTE 3: Simplify TCS scores
tcs_simplified AS (
    SELECT asset_id, ps1_name, trend_direction, MAX(tcs_score) AS max_tcs_score
    FROM us2000_trend_stability_score
    GROUP BY 1, 2, 3
),
-- CTE 4: Simplify CVI scores
cvi_simplified AS (
    SELECT asset_id, ps_name, cs_name, MAX(cvi_score) AS max_cvi_score
    FROM us2000_consolidation_volatility_index
    GROUP BY 1, 2, 3
),
-- CTE 5: Join all Contextual Scores
full_context_join AS (
    SELECT
        pd.*,
        tcs_b.max_tcs_score AS tcs_bullish,
        tcs_r.max_tcs_score AS tcs_bearish,
        cvi.max_cvi_score AS cvi_score,
        fhr_b.max_fhr_score AS fhr_bullish_reversal,
        fhr_r.max_fhr_score AS fhr_bearish_reversal
    FROM pcs_directional pd
    LEFT JOIN tcs_simplified tcs_b ON pd.asset_id = tcs_b.asset_id AND pd.ps_name = tcs_b.ps1_name AND tcs_b.trend_direction = 'Bullish'
    LEFT JOIN tcs_simplified tcs_r ON pd.asset_id = tcs_r.asset_id AND pd.ps_name = tcs_r.ps1_name AND tcs_r.trend_direction = 'Bearish'
    LEFT JOIN cvi_simplified cvi ON pd.asset_id = cvi.asset_id AND pd.ps_name = cvi.ps_name AND pd.cs_name = cvi.cs_name
    LEFT JOIN fhr_simplified fhr_b ON pd.asset_id = fhr_b.asset_id AND pd.ps_name = fhr_b.ps_name AND pd.cs_name = fhr_b.cs_name AND fhr_b.pattern_type = 'PDL_Break_to_Bullish'
    LEFT JOIN fhr_simplified fhr_r ON pd.asset_id = fhr_r.asset_id AND pd.ps_name = fhr_r.ps_name AND pd.cs_name = fhr_r.cs_name AND fhr_r.pattern_type = 'PDH_Break_to_Bearish'
),
-- CTE 6: Calculate the Final Signed UCS Score (with COALESCE for NULLs)
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
            -- TCS Bonus/Penalty (Inverse Logic) - Default TCS to 50.0 (Neutral)
            + CASE
                WHEN COALESCE(pcs_bullish, 0.0) >= COALESCE(pcs_bearish, 0.0) THEN -- Bullish Direction
                    CASE
                        WHEN COALESCE(tcs_bullish, 50.0) >= 65.0 THEN 0.25
                        WHEN COALESCE(tcs_bullish, 50.0) <= 20.0 THEN -0.50
                        WHEN COALESCE(tcs_bullish, 50.0) <= 40.0 THEN -0.30
                        ELSE 0.0
                    END
                ELSE -- Bearish Direction
                    CASE
                        WHEN COALESCE(tcs_bearish, 50.0) >= 65.0 THEN 0.25
                        WHEN COALESCE(tcs_bearish, 50.0) <= 20.0 THEN -0.50
                        WHEN COALESCE(tcs_bearish, 50.0) <= 40.0 THEN -0.30
                        ELSE 0.0
                    END
            END            
            -- CVI Penalty/Bonus (Inverse Logic) - Default CVI to 50.0 (Moderate Risk)
            + CASE
                WHEN COALESCE(cvi_score, 50.0) >= 70.0 THEN -0.20
                WHEN COALESCE(cvi_score, 50.0) <= 15.0 THEN 0.35
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
-- Final Selection and UCS Score Application
INSERT INTO us2000_unified_context_score (
    asset_id, ps_name, cs_name, ps_bias_3, cs_bias_3,
    ucs_score, ucs_signal, pcs_base, tcs_bullish_score, tcs_bearish_score, cvi_score, fhr_reversal_score
)
SELECT
    fuc.asset_id, fuc.ps_name, fuc.cs_name, fuc.ps_bias_3, fuc.cs_bias_3,    
    -- 1. Final UCS Score: Calculated on the fly (applies override)
    CASE
        WHEN fuc.fhr_override_flag = 'BEARISH_REVERSAL_OVERRIDE' THEN -2.0
        WHEN fuc.fhr_override_flag = 'BULLISH_REVERSAL_OVERRIDE' THEN 2.0
        ELSE fuc.ucs_base * fuc.confidence_multiplier
    END AS ucs_score,    
    -- 2. Final UCS Signal Label: Uses the calculated score logic
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

-------------------------
-- UCS main 
-----------------

CREATE TABLE us2000_ucs_score (
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    ps1_name TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    trade_direction TEXT NOT NULL,
    pcs_base_score DOUBLE PRECISION NOT NULL,
    tcs_score DOUBLE PRECISION NOT NULL,
    cvi_score DOUBLE PRECISION NOT NULL,
    fhr_veto_score DOUBLE PRECISION NOT NULL,
    unified_confidence_score DOUBLE PRECISION NOT NULL,
    PRIMARY KEY(asset_id, trading_date, ps1_name, cs_name)
);

-- UCS SQL: Calculates the Unified Confidence Score using Foreign Keys for contextual lookups.

-- 1. Determine the latest trading date available in the system
WITH latest_date AS (
    SELECT MAX(trading_date) AS current_trading_date FROM us2000_session_context
),
-- 2. Target the PCS signals for the latest date and join the full context
target_signal_context AS (
    SELECT
        t.asset_id,
        t.trading_date,
        t.ps_name AS ps1_name,
        t.cs_name,
        t.daily_outcome_7,
        t.pcs_score AS pcs_base_score,
        t.cs_ps1_fk, -- The Foreign Key to the context table
        -- Map daily_outcome_7 to the Trade Direction
        CASE
            WHEN t.daily_outcome_7 IN ('Bullish', 'Bullish_Reversal', 'Failed_Bearish') THEN 'BULLISH'
            WHEN t.daily_outcome_7 IN ('Bearish', 'Bearish_Reversal', 'Failed_Bullish') THEN 'BEARISH'
            ELSE 'NEUTRAL'
        END AS trade_direction,
        -- Join 1: Directly pull PS2's key and PS1's 3-state bias from the context table
        uc.ps2_ps1_fk,
        uc.ps1_7_state,
        uc.ps2_7_state,
        ps1_view.session_type AS ps1_3_state -- Need the 3-state for the CVI condition
    FROM us2000_predictive_confidence_score t
    CROSS JOIN latest_date ld
    WHERE t.trading_date = ld.current_trading_date
    -- Join A: Link PCS to the context table using the common Foreign Key
    JOIN us2000_session_context uc ON t.cs_ps1_fk = uc.cs_ps1_fk      
  -- Join B: Re-fetch PS1's 3-state bias (since PCS only stores 3-state and context stores 7-state)
    JOIN us2000_session_views ps1_view
        ON ps1_view.asset_id = uc.asset_id
        AND ps1_view.session_name = uc.ps1_name
        AND ps1_view.trading_date = uc.trading_date
    WHERE t.daily_outcome_7 IS NOT NULL -- Exclude empty signals
      AND t.cs_ps1_fk IS NOT NULL       -- Ensure context was found
),
-- 3. Final UCS Calculation and Aggregation
final_ucs_results AS (
    SELECT
        s.trading_date,
        s.asset_id,
        s.ps1_name,
        s.cs_name,
        s.trade_direction,
        s.pcs_base_score,
        -- Join 2: TCS Score (Join by ps2_ps1_fk)
        COALESCE(tcs.tcs_score, 60.0) AS tcs_score,
        -- Join 3: CVI Score (Conditional Join by cs_ps1_fk, only used if PS1 was Consolidation)
        COALESCE(
            CASE WHEN s.ps1_3_state = 'Consolidation' THEN cvi.cvi_score ELSE NULL END, 
            60.0
        ) AS cvi_score,
        -- Join 4: FHR Score (Join by cs_ps1_fk)
        COALESCE(fhr.fhr_score, 0.0) AS fhr_veto_score,
        -- CALCULATE FINAL UCS
        s.pcs_base_score
        * (COALESCE(tcs.tcs_score, 60.0) / 60.0)
        * (
            COALESCE(
                CASE WHEN s.ps1_3_state = 'Consolidation' THEN cvi.cvi_score ELSE NULL END, 
                60.0
            ) / 60.0
        )
        * (CASE WHEN COALESCE(fhr.fhr_score, 0.0) >= 60.0 THEN 0.20 ELSE 1.0 END) AS unified_confidence_score
    FROM target_signal_context s
    -- Left Join TCS using the PS2->PS1 FK
    LEFT JOIN us2000_trend_stability_score tcs
        ON s.ps2_ps1_fk = tcs.ps2_ps1_fk
        AND s.trade_direction = tcs.trend_direction -- TCS still needs direction for the lookup
    -- Left Join CVI using the PS1->CS FK
    LEFT JOIN us2000_consolidation_volatility_index cvi
        ON s.cs_ps1_fk = cvi.cs_ps1_fk
    -- Left Join FHR using the PS1->CS FK
    LEFT JOIN us2000_fhr_reversal_score fhr
        ON s.cs_ps1_fk = fhr.cs_ps1_fk
        -- FHR still needs the specific pattern_type to match the takedown event
        -- (This matching logic is complex and skipped here, assuming the most relevant FHR score is pulled)
)
-- 4. INSERT the final results into the output table
INSERT INTO us2000_ucs_score (
    asset_id, trading_date, ps1_name, cs_name, trade_direction, pcs_base_score,
    tcs_score, cvi_score, fhr_veto_score, unified_confidence_score
)
SELECT * FROM final_ucs_results;