

TRUNCATE asset_daily_composite_score;

WITH Daily_Conditions AS (
    -- Collects the DOW and Day Type transitions (assuming asset_daily_views exists)
    SELECT
        trading_date, asset_id,
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(dow, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || dow AS Current_DOW_Transition
    FROM asset_daily_views
),
Joined_Metrics AS (
    -- Joins daily conditions to the full set of conditional probabilities (assuming asset_daily_conditional_metrics exists)
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
    -- Joins the Volatility Scaling Factor (assuming asset_daily_volatility exists)
    SELECT JM.*,
        COALESCE(AV.volatility_scale_factor, 1.0) AS V_R_F -- Default to 1.0 if factor is missing
    FROM Joined_Metrics JM
    LEFT JOIN asset_daily_volatility AV 
    ON JM.trading_date = AV.trading_date AND JM.asset_id = AV.asset_id
),
Combined_D_M_Scores AS (
    -- Calculates the Raw DBS (Pure Short-Term Structural Momentum)
    SELECT
        trading_date, asset_id, D_M1_Alignment_Prob,
        D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk, V_R_F,
        -- DBS_Score_Raw: PURE SHORT-TERM (ST) STRUCTURAL CONVICTION
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
        d.trading_date, d.asset_id, d.DBS_Score_Raw AS DBS, 
        -- F_Alignment Factor
        (1.0 + d.D_M1_Alignment_Prob) AS F_Alignment,
        -- F_Reversal Factor (The Dampener)
        (CASE WHEN d.DBS_Score_Raw >= 0 THEN (1.0 - 0.2 * d.D_M5_Bullish_Reversal_Risk) ELSE (1.0 - 0.2 * d.D_M5_Bearish_Reversal_Risk) END) AS F_Reversal_Log,
        -- F_Confidence Factor (Geometric Mean of Commitment and Sustainability)
        SQRT(
            d.F_Commitment_Prob * (CASE 
                WHEN d.DBS_Score_Raw >= 0 THEN d.F_Bullish_Sust_Prob 
                ELSE d.F_Bearish_Sust_Prob 
            END)
        ) AS F_Confidence_Factor,
        -- 1. Standard DCS (Multiplied by ALL factors: Alignment, Reversal, Volatility, and Confidence)
        (d.DBS_Score_Raw * (1.0 + d.D_M1_Alignment_Prob) * (CASE WHEN d.DBS_Score_Raw >= 0 THEN (1.0 - 0.2 * d.D_M5_Bullish_Reversal_Risk) ELSE (1.0 - 0.2 * d.D_M5_Bearish_Reversal_Risk) END)
         * d.V_R_F 
         * SQRT(d.F_Commitment_Prob * (CASE WHEN d.DBS_Score_Raw >= 0 THEN d.F_Bullish_Sust_Prob ELSE d.F_Bearish_Sust_Prob END))
        ) AS DCS_Final_Score,
        -- 2. Contrarian DCS (Fading Logic)
        (
            (d.DBS_Score_Raw * -1.0) 
            * CASE
                WHEN d.DBS_Score_Raw >= 0 THEN d.D_M5_Bullish_Reversal_Risk
                ELSE d.D_M5_Bearish_Reversal_Risk                          
            END
        ) AS DCS_Contrarian_Score,
        -- Placeholder Logging (Re-aliasing to F_Commitment_Log and F_Sustainability_Log)
        d.F_Commitment_Prob AS F_Commitment_Log, 
        (CASE WHEN d.DBS_Score_Raw >= 0 THEN d.F_Bullish_Sust_Prob ELSE d.F_Bearish_Sust_Prob END) AS F_Sustainability_Log
    FROM Combined_D_M_Scores d
)
-- Final Insertion
INSERT INTO asset_daily_composite_score (
    trading_date, asset_id, DBS, F_Commitment, F_Sustainability, F_Reversal, DCS, DCS_Classification,
    DCS_Contrarian, DCS_Contrarian_Classification 
)
SELECT
    d.trading_date, d.asset_id,
    ROUND(d.DBS::NUMERIC, 4) AS DBS,
    ROUND(d.F_Commitment_Log::NUMERIC, 4) AS F_Commitment, 
    ROUND(d.F_Sustainability_Log::NUMERIC, 4) AS F_Sustainability, 
    ROUND(d.F_Reversal_Log::NUMERIC, 4) AS F_Reversal, 
    
    ROUND(d.DCS_Final_Score::NUMERIC, 4) AS DCS,
    CASE
        WHEN d.DCS_Final_Score >= 0.50 THEN 'High Conviction Long'
        WHEN d.DCS_Final_Score > 0.25 THEN 'Medium Conviction Long'
        WHEN d.DCS_Final_Score < -0.50 THEN 'High Conviction Short'
        WHEN d.DCS_Final_Score < -0.25 THEN 'Medium Conviction Short'
        ELSE 'Neutral/Low Conviction'
    END AS DCS_Classification,
    
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