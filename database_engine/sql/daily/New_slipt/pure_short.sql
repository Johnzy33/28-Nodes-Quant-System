

TRUNCATE asset_daily_composite_score;

-- Daily_Conditions and Joined_Metrics CTEs remain the same, fetching all LT/ST metrics.

WITH Daily_Conditions AS (
    SELECT
        trading_date, asset_id,
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(dow, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || dow AS Current_DOW_Transition
    FROM asset_daily_views
),
Joined_Metrics AS (
    SELECT dc.trading_date, dc.asset_id,
        cm.D_M1_Alignment_Prob, -- We only need the Alignment Prob from D.M1
        cm.D_M2_PDH_Prob_ST, cm.D_M2_PDL_Prob_ST, cm.D_M2_PWH_Prob_ST, cm.D_M2_PWL_Prob_ST, 
        -- We no longer need D.M1 directional bias, D.M2 LT bias, or factors (M3, M4, M5)
        cm.D_M3_Continuation_Prob, cm.D_M4_Bullish_FT_Prob, cm.D_M4_Bearish_FT_Prob,
        cm.D_M5_Bullish_Reversal_Risk, cm.D_M5_Bearish_Reversal_Risk 
    FROM Daily_Conditions dc
    INNER JOIN asset_daily_conditional_metrics cm
    ON cm.asset_id = dc.asset_id AND cm.PD_Type = dc.Prior_Day_Type AND cm.DOW_Transition = dc.Current_DOW_Transition
    WHERE dc.Prior_Day_Type IS NOT NULL 
),
Combined_D_M_Scores AS (
    -- 3. Calculate D.M2 Score (The only component of DBS)
    SELECT
        trading_date, asset_id,
        D_M1_Alignment_Prob,
        -- 🔥 D.M2 Score: PURE SHORT-TERM (ST) STRUCTURAL CONVICTION 🔥
        (
            (D_M2_PWH_Prob_ST - D_M2_PWL_Prob_ST) * 0.60 
            + (D_M2_PDH_Prob_ST - D_M2_PDL_Prob_ST) * 0.40
        ) AS DBS_Score_Raw, -- Renaming this directly to Raw DBS
        -- Keep original factor probabilities for the final insertion only, they won't affect the score.
        D_M3_Continuation_Prob, D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob,
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk
    FROM Joined_Metrics
),
DCS_Calculation AS (
    -- 4. Apply the final D.C.S. multiplicative formula: DBS * Alignment Multiplier
    SELECT
        trading_date, asset_id, DBS_Score_Raw AS DBS, 
        (1.0 + D_M1_Alignment_Prob) AS F_Alignment, -- Our new, single "Factor"
        (DBS_Score_Raw * (1.0 + D_M1_Alignment_Prob)) AS DCS_Final_Score,
        -- Retaining old factor names for insertion compatibility
        D_M3_Continuation_Prob AS F_Commitment, 
        D_M4_Bullish_FT_Prob AS F_Sustainability,
        D_M5_Bullish_Reversal_Risk AS F_Reversal -- Using Bullish as a placeholder for insertion
    FROM Combined_D_M_Scores
)
-- 5. Final Insertion into the Composite Score Table
INSERT INTO asset_daily_composite_score (
    trading_date, asset_id, DBS, F_Commitment, F_Sustainability, F_Reversal, DCS, DCS_Classification
)
SELECT
    d.trading_date, d.asset_id,
    ROUND(d.DBS::NUMERIC, 4),
    -- Inserting placeholder values (or original M3/M4/M5 probabilities) for the factors
    ROUND(d.F_Commitment::NUMERIC, 4), 
    ROUND(d.F_Sustainability::NUMERIC, 4), 
    ROUND(d.F_Reversal::NUMERIC, 4),
    ROUND(d.DCS_Final_Score::NUMERIC, 4) AS DCS,
    CASE
        -- Final Thresholds
        WHEN d.DCS_Final_Score >= 0.50 THEN 'High Conviction Long'
        WHEN d.DCS_Final_Score > 0.25 THEN 'Medium Conviction Long'
        WHEN d.DCS_Final_Score < -0.50 THEN 'High Conviction Short'
        WHEN d.DCS_Final_Score < -0.25 THEN 'Medium Conviction Short'
        ELSE 'Neutral/Low Conviction'
    END AS DCS_Classification
FROM DCS_Calculation d
ON CONFLICT (trading_date, asset_id) 
DO UPDATE SET
    DBS = EXCLUDED.DBS, 
    -- We are updating F_Commitment, F_Sustainability, F_Reversal to be the M3/M4/M5 raw probs for storage
    F_Commitment = EXCLUDED.F_Commitment, F_Sustainability = EXCLUDED.F_Sustainability,
    F_Reversal = EXCLUDED.F_Reversal, 
    DCS = EXCLUDED.DCS, DCS_Classification = EXCLUDED.DCS_Classification;