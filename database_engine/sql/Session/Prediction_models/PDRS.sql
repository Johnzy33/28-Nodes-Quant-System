----------------
-- Prediction Model: PDRS (Predictive Data Reliability Score)
----------------

DROP TABLE IF EXISTS asset_pdrs_score CASCADE;
CREATE TABLE IF NOT EXISTS asset_pdrs_score (
    asset_id TEXT NOT NULL,
    cs_ps1_fk BIGINT NOT NULL,
    ps1_name TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    prior_level TEXT NOT NULL,
    
    p_takedown DOUBLE PRECISION NOT NULL,
    p_follow_through DOUBLE PRECISION NOT NULL,
    sbi_duration_factor DOUBLE PRECISION NOT NULL,
    
    pdrs_score DOUBLE PRECISION NOT NULL,
    respect_label TEXT NOT NULL,
    
    PRIMARY KEY (cs_ps1_fk, prior_level)
);
TRUNCATE asset_pdrs_score;

-- Define Neutral SBI Constant (15 minutes in seconds)
-- Used as the threshold for 'Decisive' vs. 'Indecisive' battle.
WITH constants AS (
    SELECT 900.0 AS neutral_sbi_duration_s 
),
latest_context AS (
    -- 1. Identify all relevant PS1->CS patterns where a structural takedown is possible
    SELECT 
        uc.asset_id,
        uc.cs_ps1_fk,
        uc.ps1_name,
        uc.cs_name,
        uc.ps1_bias_7_state AS ps1_bias_7,
        t.prior_level
    FROM asset_session_context uc
    -- Join to M3/Conditional table to limit patterns to those with a Takedown probability
    JOIN asset_conditional_pdh_pdl t 
        ON uc.asset_id = t.asset_id AND uc.ps1_bias_7_state = t.ps1_bias_7 AND uc.cs_name = t.cs_name
),
full_components AS (
    -- 2. Join all required component scores (M3, M4, M12)
    SELECT
        lc.asset_id,
        lc.cs_ps1_fk,
        lc.ps1_name,
        lc.cs_name,
        lc.prior_level,
        
        -- M3: Takedown Probability (P_Takedown) - Commitment Base
        COALESCE(m3.takedown_prob, 0.0) AS p_takedown,
        
        -- M4: Follow-Through Probability (P_FT) - Validity Veto
        COALESCE(m4.follow_through_prob, 0.0) AS p_follow_through, 
        
        -- M12: Structural Battle Index (SBI Duration) - Efficiency Factor
        COALESCE(m12.avg_battle_duration_s, c.neutral_sbi_duration_s) AS sbi_duration,
        c.neutral_sbi_duration_s
        
    FROM latest_context lc
    CROSS JOIN constants c
    -- M3 Join
    LEFT JOIN asset_conditional_pdh_pdl m3 
        ON lc.asset_id = m3.asset_id 
        AND lc.ps1_bias_7 = m3.ps1_bias_7 
        AND lc.cs_name = m3.cs_name 
        AND lc.prior_level = m3.prior_level 
        
    -- M4 Join
    LEFT JOIN asset_takedown_outcome_matrix m4 
        ON lc.asset_id = m4.asset_id
        AND lc.cs_name = m4.breaker_session 
        AND lc.prior_level = m4.prior_level
        AND m4.outcome = 'Follow_Through' -- Filter M4 for P(Follow-Through) only
        
    -- M12 Join
    LEFT JOIN asset_structural_battle_index m12 
        ON lc.asset_id = m12.asset_id 
        AND lc.ps1_name = m12.ps1_name
        AND lc.cs_name = m12.cs_name 
        AND lc.prior_level = m12.prior_level 
)
-- 3. Final Calculation and Insertion
INSERT INTO asset_pdrs_score (
    asset_id, cs_ps1_fk, ps1_name, cs_name, prior_level, 
    p_takedown, p_follow_through, sbi_duration_factor, pdrs_score, respect_label
)
SELECT
    fc.asset_id,
    fc.cs_ps1_fk,
    fc.ps1_name,
    fc.cs_name,
    fc.prior_level,
    
    fc.p_takedown,
    fc.p_follow_through,
    
    -- Calculate the inverse efficiency factor: Neutral SBI / Actual Duration
    (fc.neutral_sbi_duration_s / fc.sbi_duration) AS sbi_duration_factor,
    
    -- P.D.R.S. = P_Takedown * (Neutral_SBI / Actual_SBI) * P_Follow_Through
    (fc.p_takedown * (fc.neutral_sbi_duration_s / fc.sbi_duration) * fc.p_follow_through) AS pdrs_score,
    
    -- Assign label
    CASE
        WHEN (fc.p_takedown * (fc.neutral_sbi_duration_s / fc.sbi_duration) * fc.p_follow_through) >= 1.0 THEN 'High Respect/Decisive Break'
        WHEN (fc.p_takedown * (fc.neutral_sbi_duration_s / fc.sbi_duration) * fc.p_follow_through) >= 0.75 THEN 'Moderate Respect/Commitment'
        ELSE 'Low Respect/Indecisive Break'
    END AS respect_label
FROM full_components fc
WHERE fc.p_takedown > 0.0;