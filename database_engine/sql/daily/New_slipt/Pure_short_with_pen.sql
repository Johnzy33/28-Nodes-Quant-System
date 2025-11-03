

TRUNCATE asset_daily_composite_score;

WITH Daily_Conditions AS (
    -- Collects the key transitions for joining to conditional metrics
    SELECT
        trading_date, asset_id,
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(dow, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || dow AS Current_DOW_Transition
    FROM asset_daily_views
),
Joined_Metrics AS (
    -- Joins daily conditions to the full set of conditional probabilities
    SELECT dc.trading_date, dc.asset_id,
        -- Key Inputs for Final DCS Calculation:
        cm.D_M1_Alignment_Prob, -- Temporal Alignment Factor (Multiplier)
        cm.D_M2_PDH_Prob_ST, cm.D_M2_PDL_Prob_ST, cm.D_M2_PWH_Prob_ST, cm.D_M2_PWL_Prob_ST, -- D.M2 ST for DBS
        cm.D_M5_Bullish_Reversal_Risk, cm.D_M5_Bearish_Reversal_Risk, -- D.M5 for Reversal Filter
        -- Factor Placeholders (M3 and M4 are included for insertion only, not used in the score)
        cm.D_M3_Continuation_Prob, 
        cm.D_M4_Bullish_FT_Prob, cm.D_M4_Bearish_FT_Prob
    FROM Daily_Conditions dc
    INNER JOIN asset_daily_conditional_metrics cm
    ON cm.asset_id = dc.asset_id AND cm.PD_Type = dc.Prior_Day_Type AND cm.DOW_Transition = dc.Current_DOW_Transition
    WHERE dc.Prior_Day_Type IS NOT NULL 
),
Combined_D_M_Scores AS (
    -- 1. Calculate the Raw Daily Base Score (DBS_Score_Raw) using only D.M2 Short-Term Momentum
    SELECT
        trading_date, asset_id,
        D_M1_Alignment_Prob,
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk,
        -- D.M2 Score: PURE SHORT-TERM (ST) STRUCTURAL CONVICTION (This is our DBS)
        (
            (D_M2_PWH_Prob_ST - D_M2_PWL_Prob_ST) * 0.60 
            + (D_M2_PDH_Prob_ST - D_M2_PDL_Prob_ST) * 0.40
        ) AS DBS_Score_Raw,
        -- Factor Placeholders for final insertion
        D_M3_Continuation_Prob AS F_Commitment_Prob, 
        D_M4_Bullish_FT_Prob AS F_Bullish_Sust_Prob, 
        D_M4_Bearish_FT_Prob AS F_Bearish_Sust_Prob
    FROM Joined_Metrics
),
DCS_Calculation AS (
    -- 2. Apply Multiplicative Filters (Alignment and Reversal) to the Raw DBS
    SELECT
        trading_date, asset_id, DBS_Score_Raw AS DBS, 
        -- F_Alignment (Amplifier): 
        (1.0 + D_M1_Alignment_Prob) AS F_Alignment,
        -- F_Reversal (Dampener/Filter): Max 20% penalty if reversal risk is 100%
        CASE
            WHEN DBS_Score_Raw >= 0 THEN (1.0 - 0.2 * D_M5_Bullish_Reversal_Risk)
            ELSE (1.0 - 0.2 * D_M5_Bearish_Reversal_Risk)
        END AS F_Reversal_Filter,
        -- FINAL DCS: DBS * F_Alignment * F_Reversal_Filter
        (DBS_Score_Raw * (1.0 + D_M1_Alignment_Prob) * CASE
            WHEN DBS_Score_Raw >= 0 THEN (1.0 - 0.2 * D_M5_Bullish_Reversal_Risk)
            ELSE (1.0 - 0.2 * D_M5_Bearish_Reversal_Risk)
         END
        ) AS DCS_Final_Score,
        -- Using raw probabilities for F_Commitment/F_Sustainability insertion (for full data logging)
        F_Commitment_Prob, F_Bullish_Sust_Prob, F_Bearish_Sust_Prob
    FROM Combined_D_M_Scores
)
-- 3. Final Insertion into the Composite Score Table
INSERT INTO asset_daily_composite_score (
    trading_date, asset_id, DBS, F_Commitment, F_Sustainability, F_Reversal, DCS, DCS_Classification
)
SELECT
    d.trading_date, d.asset_id,
    ROUND(d.DBS::NUMERIC, 4),
    -- Insert M3 Cont Prob as F_Commitment (Placeholder)
    ROUND(d.F_Commitment_Prob::NUMERIC, 4), 
    -- Insert M4 FT Prob (Bullish/Bearish based on final DCS sign) as F_Sustainability (Placeholder)
    ROUND(CASE WHEN d.DCS_Final_Score >= 0 THEN d.F_Bullish_Sust_Prob ELSE d.F_Bearish_Sust_Prob END::NUMERIC, 4), 
    -- Insert the actual F_Reversal_Filter value
    ROUND(d.F_Reversal_Filter::NUMERIC, 4),
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
    F_Commitment = EXCLUDED.F_Commitment, 
    F_Sustainability = EXCLUDED.F_Sustainability,
    F_Reversal = EXCLUDED.F_Reversal, 
    DCS = EXCLUDED.DCS, 
    DCS_Classification = EXCLUDED.DCS_Classification;