----------------------------------------------------------
-- Final Trade Confidence Score (FTCS) Calculation
----------------------------------------------------------

DROP TABLE IF EXISTS asset_final_trade_confidence_score CASCADE;
CREATE TABLE IF NOT EXISTS asset_final_trade_confidence_score (
    asset_id TEXT NOT NULL,
    cs_ps1_fk BIGINT NOT NULL,                        -- Foreign Key to asset_session_context (for pattern lookup)
    
    ucs_base_score DOUBLE PRECISION NOT NULL,         -- The original UCS score (e.g., 1.60)
    
    reversal_factor DOUBLE PRECISION,                 -- F_Reversal (from TERS)
    efficacy_factor DOUBLE PRECISION,                 -- F_Efficacy (from CBES)
    commitment_factor DOUBLE PRECISION,               -- F_Commitment (from PDRS)
    
    ftcs_score DOUBLE PRECISION NOT NULL,             -- The final aggregated score: UCS * F_Rev * F_Eff * F_Com
    final_signal TEXT NOT NULL,                       -- Categorical interpretation of the FTCS score
    
    PRIMARY KEY (cs_ps1_fk)
);
TRUNCATE asset_final_trade_confidence_score;

WITH all_scores AS (
    -- 1. Join UCS (M11) and the three new specialized scores
    SELECT
        uc.asset_id,
        uc.cs_ps1_fk,
        uc.cs_name,
        uc.ucs_score, -- UCS is the base score
        
        ters.ters_score,
        cbes.cbes_score,
        pdrs.pdrs_score
    FROM asset_unified_confidence_score uc -- Assuming M11 is stored here
    LEFT JOIN asset_ters_score ters ON uc.cs_ps1_fk = ters.cs_ps1_fk
    LEFT JOIN asset_cbes_score cbes ON uc.cs_ps1_fk = cbes.cs_ps1_fk
    LEFT JOIN asset_pdrs_score pdrs ON uc.cs_ps1_fk = pdrs.cs_ps1_fk
),
factor_calculation AS (
    -- 2. Apply conditional logic to determine the three adjustment factors
    SELECT
        s.*,
        -- A. Reversal Factor (F_Reversal) - TERS
        CASE
            WHEN s.ters_score IS NULL THEN 1.0 -- Neutral if TERS is unavailable
            WHEN s.ters_score >= 0.65 THEN 0.50
            WHEN s.ters_score > 0.40 THEN (1.0 - (s.ters_score / 1.30))
            ELSE 1.0
        END AS reversal_factor,
        -- B. Efficacy Factor (F_Efficacy) - CBES
        CASE
            WHEN s.cbes_score IS NULL THEN 1.0
            WHEN s.cbes_score >= 0.60 THEN 1.25
            WHEN s.cbes_score > 0.30 THEN (s.cbes_score / 0.60)
            ELSE 0.50
        END AS efficacy_factor,
        -- C. Commitment Factor (F_Commitment) - PDRS
        CASE
            WHEN s.pdrs_score IS NULL THEN 1.0
            WHEN s.pdrs_score >= 1.20 THEN 1.20
            WHEN s.pdrs_score > 0.75 THEN (s.pdrs_score / 1.0)
            ELSE 0.75
        END AS commitment_factor
        
    FROM all_scores s
)
-- 3. Final Calculation and Insertion (F.T.C.S.)
INSERT INTO asset_final_trade_confidence_score (
    asset_id, cs_ps1_fk, ucs_base_score, ftcs_score, final_signal
)
SELECT
    f.asset_id,
    f.cs_ps1_fk,
    f.ucs_score * 100.0,
    -- FTCS = UCS * F_Rev * F_Eff * F_Com
    (f.ucs_score * f.reversal_factor * f.efficacy_factor * f.commitment_factor) AS ftcs_score,
    
    -- Final Signal Assignment (Normalized to 1.0 = 100%)
    CASE
        WHEN (f.ucs_score * f.reversal_factor * f.efficacy_factor * f.commitment_factor) >= 1.50 THEN 'Aggressive Entry (Extreme Confidence)'
        WHEN (f.ucs_score * f.reversal_factor * f.efficacy_factor * f.commitment_factor) >= 1.20 THEN 'Standard Entry (High Confidence)'
        WHEN (f.ucs_score * f.reversal_factor * f.efficacy_factor * f.commitment_factor) >= 0.90 THEN 'Caution/Wait (Moderate Confidence)'
        ELSE 'Avoid Trade (Low Confidence)'
    END AS final_signal
    
FROM factor_calculation f;