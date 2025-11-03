
TRUNCATE asset_daily_composite_score;

WITH Daily_Conditions AS (
    -- Collects the DOW and Day Type transitions
    SELECT
        trading_date, asset_id,
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(dow, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || dow AS Current_DOW_Transition
    FROM asset_daily_views
),
Joined_Metrics AS (
    -- Joins daily conditions to the full set of conditional probabilities
    SELECT dc.trading_date, dc.asset_id,
        cm.D_M1_Alignment_Prob, 
        cm.D_M2_PDH_Prob_ST, cm.D_M2_PDL_Prob_ST, cm.D_M2_PWH_Prob_ST, cm.D_M2_PWL_Prob_ST, 
        cm.D_M5_Bullish_Reversal_Risk, cm.D_M5_Bearish_Reversal_Risk,
        cm.D_M3_Continuation_Prob, cm.D_M4_Bullish_FT_Prob, cm.D_M4_Bearish_FT_Prob
    FROM Daily_Conditions dc
    INNER JOIN asset_daily_conditional_metrics cm
    ON cm.asset_id = dc.asset_id AND cm.PD_Type = dc.Prior_Day_Type AND cm.DOW_Transition = dc.Current_DOW_Transition
    WHERE dc.Prior_Day_Type IS NOT NULL 
),
Vol_Scaled_Metrics AS (
    -- Joins the Volatility Scaling Factor
    SELECT JM.*,
        COALESCE(AV.volatility_scale_factor, 1.0) AS V_R_F -- Default to 1.0 (no scaling) if factor is missing
    FROM Joined_Metrics JM
    LEFT JOIN asset_daily_volatility AV 
    ON JM.trading_date = AV.trading_date AND JM.asset_id = AV.asset_id
),
Combined_D_M_Scores AS (
    -- Calculates the Raw DBS (Pure Short-Term Structural Momentum)
    SELECT
        trading_date, asset_id, D_M1_Alignment_Prob,
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk, V_R_F,
        -- DBS_Score_Raw: PURE SHORT-TERM (ST) STRUCTURAL CONVICTION (our primary directional input)
        (
            (D_M2_PWH_Prob_ST - D_M2_PWL_Prob_ST) * 0.60 
            + (D_M2_PDH_Prob_ST - D_M2_PDL_Prob_ST) * 0.40
        ) AS DBS_Score_Raw,
        
        D_M3_Continuation_Prob AS F_Commitment_Prob, 
        D_M4_Bullish_FT_Prob AS F_Bullish_Sust_Prob, 
        D_M4_Bearish_FT_Prob AS F_Bearish_Sust_Prob
    FROM Vol_Scaled_Metrics
),
DCS_Calculation AS (
    SELECT
        trading_date, asset_id, DBS_Score_Raw AS DBS, 
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk, 
        -- Factors
        (1.0 + D_M1_Alignment_Prob) AS F_Alignment,
        -- The Reversal Multiplier (Dampener)
        CASE WHEN DBS_Score_Raw >= 0 THEN (1.0 - 0.2 * D_M5_Bullish_Reversal_Risk) ELSE (1.0 - 0.2 * D_M5_Bearish_Reversal_Risk) END AS F_Reversal_Filter,
        -- 1. Standard DCS (Structural Momentum, scaled by Volatility)
        (DBS_Score_Raw * (1.0 + D_M1_Alignment_Prob) * (CASE WHEN DBS_Score_Raw >= 0 THEN (1.0 - 0.2 * D_M5_Bullish_Reversal_Risk) ELSE (1.0 - 0.2 * D_M5_Bearish_Reversal_Risk) END)
         * V_R_F 
        ) AS DCS_Final_Score,
        -- 2. Contrarian DCS (Fading Logic)
        (
            (DBS_Score_Raw * -1.0) -- Flip the sign
            * CASE
                WHEN DBS_Score_Raw >= 0 THEN D_M5_Bullish_Reversal_Risk -- Bullish structural move -> Fade magnitude is Bullish Reversal Risk
                ELSE D_M5_Bearish_Reversal_Risk                          -- Bearish structural move -> Fade magnitude is Bearish Reversal Risk
            END
        ) AS DCS_Contrarian_Score,
        -- Placeholder Logging (used in final INSERT/UPDATE)
        F_Commitment_Prob, F_Bullish_Sust_Prob, F_Bearish_Sust_Prob,
        (CASE WHEN DBS_Score_Raw >= 0 THEN (1.0 - 0.2 * D_M5_Bullish_Reversal_Risk) ELSE (1.0 - 0.2 * D_M5_Bearish_Reversal_Risk) END) AS F_Reversal_Log    
    FROM Combined_D_M_Scores
)
-- Final Insertion
INSERT INTO asset_daily_composite_score (
    trading_date, asset_id, DBS, F_Commitment, F_Sustainability, F_Reversal, DCS, DCS_Classification,
    DCS_Contrarian, DCS_Contrarian_Classification 
)
SELECT
    d.trading_date, d.asset_id,
    ROUND(d.DBS::NUMERIC, 4) AS DBS,
    ROUND(d.F_Commitment_Prob::NUMERIC, 4) AS F_Commitment, 
    ROUND(CASE WHEN d.DCS_Final_Score >= 0 THEN d.F_Bullish_Sust_Prob ELSE d.F_Bearish_Sust_Prob END::NUMERIC, 4) AS F_Sustainability, 
    -- Corrected: Using the F_Reversal_Log alias for the F_Reversal column
    ROUND(d.F_Reversal_Log::NUMERIC, 4) AS F_Reversal, 
    
    ROUND(d.DCS_Final_Score::NUMERIC, 4) AS DCS,
    CASE
        WHEN d.DCS_Final_Score >= 0.50 THEN 'High Conviction Long'
        WHEN d.DCS_Final_Score > 0.25 THEN 'Medium Conviction Long'
        WHEN d.DCS_Final_Score < -0.50 THEN 'High Conviction Short'
        WHEN d.DCS_Final_Score < -0.25 THEN 'Medium Conviction Short'
        ELSE 'Neutral/Low Conviction'
    END AS DCS_Classification,
    -- Contrarian Insertion
    ROUND(d.DCS_Contrarian_Score::NUMERIC, 4) AS DCS_Contrarian,
    CASE
        WHEN d.DCS_Contrarian_Score >= 0.50 THEN 'High Conviction FADE Short' 
        WHEN d.DCS_Contrarian_Score > 0.25 THEN 'Medium Conviction FADE Short'
        WHEN d.DCS_Contrarian_Score < -0.50 THEN 'High Conviction FADE Long' 
        WHEN d.DCS_Contrarian_Score < -0.25 THEN 'Medium Conviction FADE Long'
        ELSE 'Neutral/Low Contrarian'
    END AS DCS_Contrarian_Classification
FROM DCS_Calculation d
ON CONFLICT (trading_date, asset_id) 
DO UPDATE SET
    DBS = EXCLUDED.DBS, F_Commitment = EXCLUDED.F_Commitment, F_Sustainability = EXCLUDED.F_Sustainability,
    F_Reversal = EXCLUDED.F_Reversal, DCS = EXCLUDED.DCS, DCS_Classification = EXCLUDED.DCS_Classification,
    DCS_Contrarian = EXCLUDED.DCS_Contrarian, DCS_Contrarian_Classification = EXCLUDED.DCS_Contrarian_Classification;