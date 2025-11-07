-- Final Metric: Combined Confidence Score (CCM)
DROP TABLE IF EXISTS combined_confidence_metrics;
CREATE TABLE IF NOT EXISTS combined_confidence_metrics (
    asset_id TEXT NOT NULL,
    -- Full 2nd Order Pattern Keys (to link to PCS)
    ps2_name TEXT NOT NULL, ps2_bias_7 TEXT NOT NULL,
    ps1_name TEXT NOT NULL, ps1_bias_7 TEXT NOT NULL,
    cs_name TEXT NOT NULL, cs_bias_7 TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    -- 6 Month Lookback Scores
    pcs_score_6m DOUBLE PRECISION,        -- Raw PCS 2nd Order
    tcs_score_6m DOUBLE PRECISION,        -- Raw TCS 1st Order (Conforming Bias)
    ccm_score_6m DOUBLE PRECISION,        -- Final Calculated CCM
    -- 1 Year Lookback Scores
    pcs_score_1y DOUBLE PRECISION,
    tcs_score_1y DOUBLE PRECISION,
    ccm_score_1y DOUBLE PRECISION,
    -- ALL Lookback Scores
    pcs_score_all DOUBLE PRECISION,
    tcs_score_all DOUBLE PRECISION,
    ccm_score_all DOUBLE PRECISION,
    final_insight TEXT NOT NULL,          -- Categorical Insight based on best CCM score
    PRIMARY KEY(asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, daily_outcome_7)
);
CALL refresh_ccm_metrics()

CREATE OR REPLACE PROCEDURE refresh_ccm_metrics()
LANGUAGE plpgsql
AS $$
BEGIN

    RAISE NOTICE 'Starting Combined Confidence Metric (CCM) Calculation (Guaranteed Keys Fix)...';

    TRUNCATE combined_confidence_metrics;

    -- START the single executable block with INSERT and the WITH clause
    INSERT INTO combined_confidence_metrics (
        asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, daily_outcome_7,
        pcs_score_6m, tcs_score_6m, ccm_score_6m,
        pcs_score_1y, tcs_score_1y, ccm_score_1y,
        pcs_score_all, tcs_score_all, ccm_score_all,
        final_insight
    )
    
    -- 1. Combine PCS (2nd Order) and TCS (1st Order) for ALL lookbacks
    WITH CombinedScores AS (
        SELECT
            p.asset_id, p.ps2_name, p.ps2_bias_7, p.ps1_name, p.ps1_bias_7, p.cs_name, p.cs_bias_7, p.daily_outcome_7,
            p.lookback_period,
            p.pcs_score,
            COALESCE(t.tcs_score, 1.0) AS tcs_conforming_score 
        FROM predictive_confidence_score_2nd_order p
        
        -- LEFT JOIN to the 1st Order TCS
        LEFT JOIN tcs_1st_order t ON 
            p.asset_id = t.asset_id 
            AND p.lookback_period = t.lookback_period
            AND p.ps1_name = t.ps1_name
            AND p.cs_name = t.cs_name
            
            -- PS1 Bias Match: GUARANTEE p.ps1_bias_7 is non-NULL string for join
            AND COALESCE(p.ps1_bias_7, 'NO_BIAS_P') = t.ps1_bias
            
            -- CS Bias / Daily Outcome Match: GUARANTEE p.daily_outcome_7 is non-NULL string
            AND COALESCE(p.daily_outcome_7, 'NO_OUTCOME_P') = t.cs_bias
    ),
    
    -- 2. Apply the TCS Adjustment Factor and Calculate CCM
    CCM_Calculated AS (
        SELECT
            cs.asset_id, cs.ps2_name, cs.ps2_bias_7, cs.ps1_name, cs.ps1_bias_7, cs.cs_name, cs.cs_bias_7, cs.daily_outcome_7,
            cs.lookback_period,
            cs.pcs_score,
            cs.tcs_conforming_score,
            ROUND((cs.pcs_score * CASE
                WHEN cs.tcs_conforming_score >= 1.5 THEN 1.25
                WHEN cs.tcs_conforming_score >= 1.0 THEN 1.00
                WHEN cs.tcs_conforming_score < 0.75 THEN 0.75
                ELSE 0.90 
            END)::NUMERIC, 2)::DOUBLE PRECISION AS ccm_score
        FROM CombinedScores cs
    ),
    
    -- 3. Pivot the three lookbacks into a single row
    CCM_Final AS (
        SELECT
            asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, daily_outcome_7,
            -- 6M Scores
            MAX(CASE WHEN lookback_period = '6M' THEN pcs_score END) AS pcs_score_6m,
            MAX(CASE WHEN lookback_period = '6M' THEN tcs_conforming_score END) AS tcs_score_6m,
            MAX(CASE WHEN lookback_period = '6M' THEN ccm_score END) AS ccm_score_6m,
            -- 1Y Scores
            MAX(CASE WHEN lookback_period = '1Y' THEN pcs_score END) AS pcs_score_1y,
            MAX(CASE WHEN lookback_period = '1Y' THEN tcs_conforming_score END) AS tcs_score_1y,
            MAX(CASE WHEN lookback_period = '1Y' THEN ccm_score END) AS ccm_score_1y,
            -- ALL Scores
            MAX(CASE WHEN lookback_period = 'ALL' THEN pcs_score END) AS pcs_score_all,
            MAX(CASE WHEN lookback_period = 'ALL' THEN tcs_conforming_score END) AS tcs_score_all,
            MAX(CASE WHEN lookback_period = 'ALL' THEN ccm_score END) AS ccm_score_all
        FROM CCM_Calculated
        GROUP BY 1, 2, 3, 4, 5, 6, 7, 8
    )

    -- 4. Final SELECT statement consumes CCM_Final for the INSERT
    SELECT
        f.*,
        CASE
            WHEN f.ccm_score_6m >= 2.0 OR f.ccm_score_1y >= 2.0 THEN 'Extreme Confidence Signal (CCM >= 2.0)'
            WHEN f.ccm_score_6m >= 1.5 OR f.ccm_score_1y >= 1.5 THEN 'High-Confidence Signal (CCM >= 1.5)'
            WHEN f.ccm_score_6m < 0.75 AND f.ccm_score_1y < 0.75 THEN 'Strong Failure Warning (CCM < 0.75)'
            ELSE 'Confirmatory/Expected Range'
        END AS final_insight
    FROM CCM_Final f;

    RAISE NOTICE 'CCM Calculation complete.';

END;
$$;