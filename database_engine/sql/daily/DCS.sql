---------------
-- D.C.S 
--------

DROP TABLE IF EXISTS asset_daily_composite_score;
CREATE TABLE asset_daily_composite_score (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    DBS NUMERIC(10, 4),             -- Daily Base Score (Weighted D.M1/D.M2)
    F_Commitment NUMERIC(10, 4),    -- Commitment Factor (D.M3 multiplier)
    F_Sustainability NUMERIC(10, 4),-- Sustainability Factor (D.M4 multiplier)
    F_Reversal NUMERIC(10, 4),      -- Reversal Penalty Factor (D.M5 multiplier)
    DCS NUMERIC(10, 4),             -- Final Daily Composite Score
    DCS_Classification TEXT,        -- Final categorical output (e.g., 'High Conviction Long')
    PRIMARY KEY (trading_date, asset_id)
);
TRUNCATE asset_daily_composite_score;

WITH Daily_Conditions AS (
    -- 1. Establish the lookup key for every single day in the asset_daily_views table
    SELECT
        trading_date,
        asset_id,
        LAG(day_type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS Prior_Day_Type,
        LAG(dow, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || dow AS Current_DOW_Transition
    FROM
        asset_daily_views
),
Joined_Metrics AS (
    -- 2. Join the daily date/asset pair to the static lookup table
    SELECT
        dc.trading_date,
        dc.asset_id,
        -- All D.M1 to D.M5 probabilities from the lookup table
        cm.D_M1_Bullish_Prob,
        cm.D_M1_Bearish_Prob,
        cm.D_M2_PDH_Prob,
        cm.D_M2_PDL_Prob,
        cm.D_M2_PWH_Prob,
        cm.D_M2_PWL_Prob,
        cm.D_M3_Continuation_Prob,
        cm.D_M4_Bullish_FT_Prob,
        cm.D_M4_Bearish_FT_Prob,
        cm.D_M5_Bullish_Reversal_Risk,
        cm.D_M5_Bearish_Reversal_Risk
    FROM
        Daily_Conditions dc
    INNER JOIN 
        asset_daily_conditional_metrics cm
    ON
        cm.asset_id = dc.asset_id
        AND cm.PD_Type = dc.Prior_Day_Type
        AND cm.DOW_Transition = dc.Current_DOW_Transition
    -- Filter out the first day where Prior_Day_Type is NULL
    WHERE dc.Prior_Day_Type IS NOT NULL 
),
Combined_D_M_Scores AS (
    -- 3. Calculate D.M1 and D.M2 scores and carry all factors
    SELECT
        trading_date,
        asset_id,
        -- D.M1 Score
        (D_M1_Bullish_Prob - D_M1_Bearish_Prob) AS D_M1_Score, 
        -- D.M2 Score
        ((D_M2_PDH_Prob + D_M2_PWH_Prob) - (D_M2_PDL_Prob + D_M2_PWL_Prob)) / 2.0 AS D_M2_Score, 
        
        D_M3_Continuation_Prob,
        D_M4_Bullish_FT_Prob,
        D_M4_Bearish_FT_Prob,
        D_M5_Bullish_Reversal_Risk,
        D_M5_Bearish_Reversal_Risk
    FROM Joined_Metrics
),
Base_Score AS (
    -- 4. Calculate the Daily Base Score (D.B.S.) using the 60/40 weighting
    SELECT
        trading_date,
        asset_id,
        (D_M1_Score * 0.60 + D_M2_Score * 0.40) AS DBS_Score,
        -- Carry all factor base metrics forward
        d_m3_continuation_prob, d_m4_bullish_ft_prob, d_m4_bearish_ft_prob,
        d_m5_bullish_reversal_risk, d_m5_bearish_reversal_risk
    FROM Combined_D_M_Scores
),
Factor_Calculation AS (
    -- 5. Convert probabilities into multiplicative factors
    SELECT
        trading_date,
        asset_id,
        DBS_Score,
        -- F_Commitment (D.M3): 
        (0.8 + 0.4 * d_m3_continuation_prob) AS F_Commitment, 
        -- F_Sustainability (D.M4): Ratio of Follow-Through to Failure
        CASE 
            WHEN DBS_Score >= 0 THEN 
                (d_m4_bullish_ft_prob + 0.1) / (1.0 - d_m4_bullish_ft_prob + 0.1) 
            ELSE 
                (d_m4_bearish_ft_prob + 0.1) / (1.0 - d_m4_bearish_ft_prob + 0.1) 
        END AS F_Sustainability,
        -- F_Reversal (D.M5): Penalty (1 - Risk)
        (1.0 - LEAST(d_m5_bullish_reversal_risk, d_m5_bearish_reversal_risk)) AS F_Reversal 
    FROM Base_Score
),
DCS_Calculation AS (
    -- 6. Apply the final D.C.S. multiplicative formula
    SELECT
        trading_date,
        asset_id,
        DBS_Score,
        F_Commitment,
        F_Sustainability,
        F_Reversal,
        (DBS_Score * F_Commitment * F_Sustainability * F_Reversal) AS DCS_Final_Score
    FROM Factor_Calculation
)
-- 7. Final Insertion into the Composite Score Table
INSERT INTO asset_daily_composite_score (
    trading_date, asset_id, DBS, F_Commitment, F_Sustainability, F_Reversal, DCS, DCS_Classification
)
SELECT
    d.trading_date, 
    d.asset_id,
    d.DBS_Score,
    d.F_Commitment,
    d.F_Sustainability,
    d.F_Reversal,
    d.DCS_Final_Score,
    CASE
        WHEN d.DCS_Final_Score >= 0.75 THEN 'High Conviction Long'
        WHEN d.DCS_Final_Score > 0.3 THEN 'Medium Conviction Long'
        WHEN d.DCS_Final_Score < -0.75 THEN 'High Conviction Short'
        WHEN d.DCS_Final_Score < -0.3 THEN 'Medium Conviction Short'
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