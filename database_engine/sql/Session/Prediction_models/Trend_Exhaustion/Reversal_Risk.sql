----------------------------------
-- Trend Exhaustion Reversal Risk Model
----------------------------------

DROP TABLE IF EXISTS asset_ters_score CASCADE;
CREATE TABLE IF NOT EXISTS asset_ters_score (
    asset_id TEXT NOT NULL,
    cs_ps1_fk BIGINT NOT NULL,
    ps1_name TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    trend_direction TEXT NOT NULL,                -- Predicted direction of the risk
    
    oit_score DOUBLE PRECISION NOT NULL,          -- M14: Internal Exhaustion Risk
    fhr_score DOUBLE PRECISION NOT NULL,          -- M10: Structural Reversal Risk
    tcs_score DOUBLE PRECISION NOT NULL,          -- M8: Trend Stability (Normalized Denominator)
    
    ters_score DOUBLE PRECISION NOT NULL,         -- Final Aggregated Score
    risk_label TEXT NOT NULL,                     -- 'High Alert', 'Moderate Risk', 'Stable'
    
    PRIMARY KEY (cs_ps1_fk)
);
TRUNCATE asset_ters_score;

WITH component_join AS (
    -- 1. Join all required component scores (M14, M10, M8)
    SELECT
        uc.asset_id,
        uc.cs_ps1_fk,
        uc.ps1_name,
        uc.cs_name,
        uc.ps1_bias_7_state AS ps1_bias_7,
        -- Determine the trend direction for joining FHR and OIT
        CASE
            WHEN uc.ps1_bias_7_state LIKE 'Bullish%' THEN 'BULLISH'
            WHEN uc.ps1_bias_7_state LIKE 'Bearish%' THEN 'BEARISH'
            ELSE 'NEUTRAL'
        END AS trend_direction,
        -- M14: OIT Score (Internal Exhaustion)
        COALESCE(m14.oit_score, 0.0) AS oit_score,
        -- M10: FHR Score (Structural Reversal Precedent)
        -- NOTE: M10 uses the Breaker Session (CS) to measure reversal risk
        COALESCE(m10.fhr_score / 100.0, 0.0) AS fhr_score, 
        -- M8: TCS Score (Trend Stability)
        COALESCE(m8.tcs_score / 100.0, 0.60) AS tcs_score_norm -- Default to 0.60 (Neutral) if not found
    FROM asset_session_context uc
    -- M14 Join (OIT)
    LEFT JOIN asset_open_interest_takedown m14 
        ON uc.asset_id = m14.asset_id AND uc.ps1_bias_7_state = m14.ps1_bias_7 AND uc.cs_name = m14.cs_name
    -- M10 Join (FHR)
    LEFT JOIN asset_fhr_reversal_score m10 
        ON uc.asset_id = m10.asset_id AND uc.cs_name = m10.breaker_session -- FHR is indexed by the breaker session
    -- M8 Join (TCS)
    LEFT JOIN asset_trend_stability_score m8 
        ON uc.asset_id = m8.asset_id AND uc.cs_ps1_fk = m8.cs_ps1_fk
    WHERE uc.ps1_bias_7_state NOT IN ('Pure_Indecision', 'Other')
)
-- 2. Final Calculation and Insertion
INSERT INTO asset_ters_score (
    asset_id, cs_ps1_fk, ps1_name, cs_name, trend_direction, 
    oit_score, fhr_score, tcs_score, ters_score, risk_label
)
SELECT
    c.asset_id,
    c.cs_ps1_fk,
    c.ps1_name,
    c.cs_name,
    c.trend_direction,
    
    c.oit_score,
    c.fhr_score,
    c.tcs_score_norm,
    -- TERS = (OIT * FHR) / TCS_Norm
    (c.oit_score * c.fhr_score) / c.tcs_score_norm AS ters_score,
    -- Assign risk label (Thresholds are normalized, e.g., > 0.65 is High Risk)
    CASE
        WHEN (c.oit_score * c.fhr_score) / c.tcs_score_norm >= 0.65 THEN 'HIGH ALERT (Trend Exhaustion Likely)'
        WHEN (c.oit_score * c.fhr_score) / c.tcs_score_norm >= 0.40 THEN 'MODERATE RISK (Caution)'
        ELSE 'STABLE'
    END AS risk_label
FROM component_join c
WHERE c.trend_direction != 'NEUTRAL';