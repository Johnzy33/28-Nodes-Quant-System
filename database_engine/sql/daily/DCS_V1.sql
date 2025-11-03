
TRUNCATE asset_daily_composite_score;

WITH Daily_Conditions AS (
    SELECT
        trading_date, asset_id,
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(dow, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || dow AS Current_DOW_Transition
    FROM asset_daily_views
),
Joined_Metrics AS (
    SELECT
        dc.trading_date, dc.asset_id,
        cm.D_M1_Bullish_Prob, cm.D_M1_Bearish_Prob, cm.D_M2_PDH_Prob, cm.D_M2_PDL_Prob, 
        cm.D_M2_PWH_Prob, cm.D_M2_PWL_Prob, cm.D_M3_Continuation_Prob, cm.D_M4_Bullish_FT_Prob,
        cm.D_M4_Bearish_FT_Prob, cm.D_M5_Bullish_Reversal_Risk, cm.D_M5_Bearish_Reversal_Risk
    FROM Daily_Conditions dc
    INNER JOIN asset_daily_conditional_metrics cm
    ON cm.asset_id = dc.asset_id AND cm.PD_Type = dc.Prior_Day_Type AND cm.DOW_Transition = dc.Current_DOW_Transition
    WHERE dc.Prior_Day_Type IS NOT NULL 
),
Combined_D_M_Scores AS (
    -- 3. Calculate D.M1 and D.M2 scores
    SELECT
        trading_date, asset_id,
        (D_M1_Bullish_Prob - D_M1_Bearish_Prob) AS D_M1_Score, 
        -- REVISED D.M2 Score: Take the Maximum structural conviction difference
        GREATEST(
            (D_M2_PDH_Prob - D_M2_PDL_Prob),
            (D_M2_PWH_Prob - D_M2_PWL_Prob)
        ) AS D_M2_Score, 
        
        D_M3_Continuation_Prob, D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob,
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk
    FROM Joined_Metrics
),
Base_Score AS (
    -- 4. Calculate the Daily Base Score (D.B.S.) using the 60/40 weighting
    SELECT
        trading_date, asset_id,
        (D_M1_Score * 0.60 + D_M2_Score * 0.40) AS DBS_Score,
        d_m3_continuation_prob, d_m4_bullish_ft_prob, d_m4_bearish_ft_prob,
        d_m5_bullish_reversal_risk, d_m5_bearish_reversal_risk
    FROM Combined_D_M_Scores
),
Factor_Calculation AS (
    -- 5. Convert probabilities into multiplicative factors (from previous fix)
    SELECT
        trading_date, asset_id, DBS_Score,
        (0.5 + 1.0 * d_m3_continuation_prob) AS F_Commitment, 
        CASE 
            WHEN DBS_Score >= 0 THEN (0.7 + 1.8 * d_m4_bullish_ft_prob) 
            ELSE (0.7 + 1.8 * d_m4_bearish_ft_prob) 
        END AS F_Sustainability,
        CASE
            WHEN DBS_Score >= 0 THEN (1.0 - d_m5_bullish_reversal_risk)
            ELSE (1.0 - d_m5_bearish_reversal_risk)
        END AS F_Reversal 
    FROM Base_Score
),
DCS_Calculation AS (
    -- 6. Apply the final D.C.S. multiplicative formula
    SELECT
        trading_date, asset_id, DBS_Score, F_Commitment, F_Sustainability, F_Reversal,
        (DBS_Score * F_Commitment * F_Sustainability * F_Reversal) AS DCS_Final_Score
    FROM Factor_Calculation
)
-- 7. Final Insertion into the Composite Score Table
INSERT INTO asset_daily_composite_score (
    trading_date, asset_id, DBS, F_Commitment, F_Sustainability, F_Reversal, DCS, DCS_Classification
)
SELECT
    d.trading_date, d.asset_id,
    ROUND(d.DBS_Score::NUMERIC, 4) AS DBS,
    ROUND(d.F_Commitment::NUMERIC, 4) AS F_Commitment,
    ROUND(d.F_Sustainability::NUMERIC, 4) AS F_Sustainability,
    ROUND(d.F_Reversal::NUMERIC, 4) AS F_Reversal,
    ROUND(d.DCS_Final_Score::NUMERIC, 4) AS DCS,
    CASE
        -- REVISED THRESHOLDS
        WHEN d.DCS_Final_Score >= 0.50 THEN 'High Conviction Long'
        WHEN d.DCS_Final_Score > 0.25 THEN 'Medium Conviction Long'
        WHEN d.DCS_Final_Score < -0.50 THEN 'High Conviction Short'
        WHEN d.DCS_Final_Score < -0.25 THEN 'Medium Conviction Short'
        ELSE 'Neutral/Low Conviction'
    END AS DCS_Classification
FROM DCS_Calculation d
ON CONFLICT (trading_date, asset_id) 
DO UPDATE SET
    DBS = EXCLUDED.DBS, F_Commitment = EXCLUDED.F_Commitment, F_Sustainability = EXCLUDED.F_Sustainability,
    F_Reversal = EXCLUDED.F_Reversal, DCS = EXCLUDED.DCS, DCS_Classification = EXCLUDED.DCS_Classification;