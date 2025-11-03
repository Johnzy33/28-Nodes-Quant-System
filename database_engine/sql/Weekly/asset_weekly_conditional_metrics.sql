-------------------------
-- Weekly Asset Conditional Metrics
-------------------------

DROP TABLE IF EXISTS asset_weekly_conditional_metrics;

CREATE TABLE asset_weekly_conditional_metrics (
    asset_id TEXT NOT NULL,
    -- Conditional Key: The Prior Week's Outcome/Bias
    PW_Bias TEXT NOT NULL,                -- e.g., 'Strong_Bull', 'Neutral', 'Strong_Bear'
    -- W.M1: Weekly Structural Break Prob. (W.S.B.)
    W_M1_PWH_Prob DOUBLE PRECISION,       -- P(Break PWH | PW_Bias)
    W_M1_PWL_Prob DOUBLE PRECISION,       -- P(Break PWL | PW_Bias)
    -- W.M2: Weekly Break Follow-Through (W.B.F.) -> F_Breakout
    W_M2_Bullish_FT_Prob DOUBLE PRECISION, -- P(FT Bullish | PWH Break)
    W_M2_Bearish_FT_Prob DOUBLE PRECISION,  -- P(FT Bearish | PWL Break)
    -- W.M3: Weekly Trend Stability (W.T.S.) -> F_Commitment
    W_M3_Continuation_Prob DOUBLE PRECISION, -- P(Strong Trend Cont. | PW_Bias)
    -- W.M4: Weekly Reversal Risk (W.F.R.) -> F_Risk (Component 1)
    W_M4_Bullish_Reversal_Risk DOUBLE PRECISION,
    W_M4_Bearish_Reversal_Risk DOUBLE PRECISION,
    -- W.M5: Weekly Structural Battle Index (W.S.B.I.) -> W.B.S. & F_Time
    W_M5_Bullish_Efficiency_Index DOUBLE PRECISION, -- Normalized Effort to break PWH
    W_M5_Bearish_Efficiency_Index DOUBLE PRECISION, -- Normalized Effort to break PWL
    -- W.M6: Weekly Open Interest Takedown (W.O.I.T.) -> F_Risk (Component 2)
    W_M6_Exhaustion_Prob DOUBLE PRECISION,  -- P(Deep Pullback > 50% | PW_Bias)
    -- Metadata
    Total_Samples INTEGER NOT NULL,
    Calculation_Date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    -- Composite Key for Conditional Lookup
    PRIMARY KEY(asset_id, PW_Bias)
);

TRUNCATE asset_weekly_conditional_metrics;

WITH W_M1_Probabilities AS (
    -- W.M1: Weekly Structural Break Prob. (W.S.B.)
    -- Conditional on asset_id and Prior Week Bias (PW_Bias)
    SELECT
        asset_id,
        LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS PW_Bias,
        COUNT(*) AS Total_Transitions,
        SUM(CASE WHEN current_week_high > prior_week_high THEN 1 ELSE 0 END)::DOUBLE PRECISION / COUNT(*) AS W_M1_PWH_Prob,
        SUM(CASE WHEN current_week_low < prior_week_low THEN 1 ELSE 0 END)::DOUBLE PRECISION / COUNT(*) AS W_M1_PWL_Prob
    FROM (
        SELECT asset_id, week_ending_date, Week_Type, high AS current_week_high, low AS current_week_low,
               LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS prior_week_high,
               LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS prior_week_low
        FROM asset_weekly_views
    ) AS W1_Base
    WHERE LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) IS NOT NULL
    GROUP BY 1, 2
    HAVING COUNT(*) >= 0
),
W_M2_Probabilities AS (
    -- W.M2: Weekly Break Follow-Through (W.B.F.)
    -- Conditional on asset_id and Prior Week Bias (PW_Bias)
    SELECT
        asset_id,
        LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS PW_Bias,
        SUM(CASE WHEN Prior_Break_Type = 'Bullish_Break' AND Week_Close_Type = 'Bullish_Close' THEN 1 ELSE 0 END)::DOUBLE PRECISION /
            NULLIF(SUM(CASE WHEN Prior_Break_Type = 'Bullish_Break' THEN 1 ELSE 0 END), 0) AS W_M2_Bullish_FT_Prob,
        SUM(CASE WHEN Prior_Break_Type = 'Bearish_Break' AND Week_Close_Type = 'Bearish_Close' THEN 1 ELSE 0 END)::DOUBLE PRECISION /
            NULLIF(SUM(CASE WHEN Prior_Break_Type = 'Bearish_Break' THEN 1 ELSE 0 END), 0) AS W_M2_Bearish_FT_Prob
    FROM (
        SELECT Week_Close_Type, asset_id,
               LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS PW_Bias,
               LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS PWH,
               LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS PWL,
               CASE WHEN high > LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) THEN 'Bullish_Break'
                    WHEN low < LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) THEN 'Bearish_Break'
                    ELSE 'No_Break' END AS Prior_Break_Type
        FROM asset_weekly_views
    ) AS W2_Base
    WHERE Prior_Break_Type != 'No_Break'
    GROUP BY 1, 2
),
W_M3_Probabilities AS (
    -- W.M3: Weekly Trend Stability (W.T.S.)
    -- Conditional on asset_id and Prior Week Bias (PW_Bias, where PW_Bias is Strong)
    SELECT
        asset_id,
        LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS PW_Bias,
        SUM(Continuation)::DOUBLE PRECISION / COUNT(*) AS W_M3_Continuation_Prob
    FROM (
        SELECT Week_Type, asset_id,
               LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS PW_Type,
               CASE WHEN LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) = 'Strong_Bull' AND Week_Type = 'Strong_Bull' THEN 1
                    WHEN LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) = 'Strong_Bear' AND Week_Type = 'Strong_Bear' THEN 1
                    ELSE 0 END AS Continuation
        FROM asset_weekly_views
    ) AS W3_Base
    WHERE PW_Type IN ('Strong_Bull', 'Strong_Bear')
    GROUP BY 1, 2
),
W_M4_Probabilities AS (
    -- W.M4: Weekly Reversal Risk (W.F.R.)
    -- Conditional on asset_id and Current Week Bias (CW_Bias) - This risk is measured *on* the current week's structural move
    SELECT
        asset_id,
        Week_Type AS CW_Bias, -- Use Current Week Type as the conditional key
        SUM(CASE WHEN high > prior_week_high AND (high - close) / (high - low) >= 0.80 THEN 1 ELSE 0 END)::DOUBLE PRECISION /
            NULLIF(SUM(CASE WHEN high > prior_week_high THEN 1 ELSE 0 END), 0) AS W_M4_Bullish_Reversal_Risk,
        SUM(CASE WHEN low < prior_week_low AND (close - low) / (high - low) <= 0.20 THEN 1 ELSE 0 END)::DOUBLE PRECISION /
            NULLIF(SUM(CASE WHEN low < prior_week_low THEN 1 ELSE 0 END), 0) AS W_M4_Bearish_Reversal_Risk
    FROM (
        SELECT asset_id, Week_Type, high, low, close,
               LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS prior_week_high,
               LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS prior_week_low
        FROM asset_weekly_views
    ) AS W4_Base
    WHERE Week_Type IS NOT NULL
    GROUP BY 1, 2
),
W_M5_Probabilities AS (
    -- W.M5: Weekly Structural Battle Index (W.S.B.I.) -> Efficiency Index
    -- Conditional on asset_id and Prior Week Bias (PW_Bias)
    -- This measures how 'efficient' the previous break was, normalized 0-1.
    -- Assuming pre-calculated 'Time_To_PWH_Break' and 'Range_At_PWH_Break' columns exist in asset_weekly_views
    SELECT
        asset_id,
        LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS PW_Bias,
        -- W_M5_Bullish_Efficiency_Index: Higher is more efficient (e.g., lower time, smaller range needed)
        COALESCE(AVG(CASE WHEN W5_Base.Break_Type = 'PWH_Break' THEN W5_Base.Normalized_Efficiency_Score ELSE NULL END), 0.5) AS W_M5_Bullish_Efficiency_Index,
        -- W_M5_Bearish_Efficiency_Index: Higher is more efficient
        COALESCE(AVG(CASE WHEN W5_Base.Break_Type = 'PWL_Break' THEN W5_Base.Normalized_Efficiency_Score ELSE NULL END), 0.5) AS W_M5_Bearish_Efficiency_Index
    FROM (
        SELECT asset_id, Week_Type,
               LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS PW_Type,
               -- Example placeholder for Normalized_Efficiency_Score logic (must be calculated prior to this query)
               (1.0 - (Time_To_Break_Normalized * 0.5 + Range_Used_Normalized * 0.5)) AS Normalized_Efficiency_Score,
               CASE WHEN high > LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) THEN 'PWH_Break'
                    WHEN low < LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) THEN 'PWL_Break'
                    ELSE 'No_Break' END AS Break_Type
        FROM asset_weekly_views
    ) AS W5_Base
    WHERE W5_Base.PW_Type IS NOT NULL
    GROUP BY 1, 2
),
W_M6_Probabilities AS (
    -- W.M6: Weekly Open Interest Takedown (W.O.I.T.) -> Exhaustion Risk
    -- Conditional on asset_id and Prior Week Bias (PW_Bias)
    SELECT
        asset_id,
        LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS PW_Bias,
        SUM(Exhaustion)::DOUBLE PRECISION / COUNT(*) AS W_M6_Exhaustion_Prob
    FROM (
        SELECT asset_id,
               LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) AS PW_Type,
               -- Exhaustion is defined as a deep pullback (>50% of the range) in the *current* week after a strong *prior* week
               CASE WHEN LAG(Week_Type, 1) OVER (PARTITION BY asset_id ORDER BY week_ending_date) IN ('Strong_Bull', 'Strong_Bear')
                         AND Week_Pullback_Ratio > 0.50 THEN 1
                    ELSE 0 END AS Exhaustion
        FROM asset_weekly_views
    ) AS W6_Base
    WHERE PW_Type IS NOT NULL
    GROUP BY 1, 2
),
Merged_Metrics AS (
    -- Merge W.M1, W.M2, W.M3, W.M5, and W.M6 using the composite key (asset_id, PW_Bias)
    SELECT
        M1.asset_id,
        M1.PW_Bias,
        M1.Total_Transitions,
        ROUND(M1.W_M1_PWH_Prob::NUMERIC, 4) AS W_M1_PWH_Prob,
        ROUND(M1.W_M1_PWL_Prob::NUMERIC, 4) AS W_M1_PWL_Prob,
        
        COALESCE(ROUND(M2.W_M2_Bullish_FT_Prob::NUMERIC, 4), 0.5) AS W_M2_Bullish_FT_Prob,
        COALESCE(ROUND(M2.W_M2_Bearish_FT_Prob::NUMERIC, 4), 0.5) AS W_M2_Bearish_FT_Prob,
        
        COALESCE(ROUND(M3.W_M3_Continuation_Prob::NUMERIC, 4), 0.5) AS W_M3_Continuation_Prob,
        
        COALESCE(ROUND(M5.W_M5_Bullish_Efficiency_Index::NUMERIC, 4), 0.5) AS W_M5_Bullish_Efficiency_Index,
        COALESCE(ROUND(M5.W_M5_Bearish_Efficiency_Index::NUMERIC, 4), 0.5) AS W_M5_Bearish_Efficiency_Index,

        COALESCE(ROUND(M6.W_M6_Exhaustion_Prob::NUMERIC, 4), 0.0) AS W_M6_Exhaustion_Prob
    FROM W_M1_Probabilities M1
    LEFT JOIN W_M2_Probabilities M2 ON M1.PW_Bias = M2.PW_Bias AND M1.asset_id = M2.asset_id
    LEFT JOIN W_M3_Probabilities M3 ON M1.PW_Bias = M3.PW_Bias AND M1.asset_id = M3.asset_id
    LEFT JOIN W_M5_Probabilities M5 ON M1.PW_Bias = M5.PW_Bias AND M1.asset_id = M5.asset_id
    LEFT JOIN W_M6_Probabilities M6 ON M1.PW_Bias = M6.PW_Bias AND M1.asset_id = M6.asset_id
)
-- Final Insertion: Merge W.M4 (Current Week Bias) with the other metrics (Prior Week Bias)
INSERT INTO asset_weekly_conditional_metrics (
    asset_id, PW_Bias, Total_Transitions,
    W_M1_PWH_Prob, W_M1_PWL_Prob,
    W_M2_Bullish_FT_Prob, W_M2_Bearish_FT_Prob,
    W_M3_Continuation_Prob,
    W_M4_Bullish_Reversal_Risk, W_M4_Bearish_Reversal_Risk,
    W_M5_Bullish_Efficiency_Index, W_M5_Bearish_Efficiency_Index,
    W_M6_Exhaustion_Prob
)
SELECT
    MM.asset_id,
    MM.PW_Bias,
    MM.Total_Transitions,
    MM.W_M1_PWH_Prob, MM.W_M1_PWL_Prob,
    MM.W_M2_Bullish_FT_Prob, MM.W_M2_Bearish_FT_Prob,
    MM.W_M3_Continuation_Prob,
    -- Join W.M4 (Reversal Risk) using the PW_Bias as the conditional key
    COALESCE(M4.W_M4_Bullish_Reversal_Risk, 0.5), 
    COALESCE(M4.W_M4_Bearish_Reversal_Risk, 0.5),
    MM.W_M5_Bullish_Efficiency_Index, MM.W_M5_Bearish_Efficiency_Index,
    MM.W_M6_Exhaustion_Prob
FROM Merged_Metrics MM
LEFT JOIN W_M4_Probabilities M4
    -- We assume the M4 risk observed on the CW_Bias applies as the risk factor
    -- when transitioning *from* that bias (PW_Bias) to the next week.
    ON M4.asset_id = MM.asset_id
    AND M4.CW_Bias = MM.PW_Bias
ON CONFLICT (asset_id, PW_Bias)
DO UPDATE SET
    W_M1_PWH_Prob = EXCLUDED.W_M1_PWH_Prob,
    W_M1_PWL_Prob = EXCLUDED.W_M1_PWL_Prob,
    W_M2_Bullish_FT_Prob = EXCLUDED.W_M2_Bullish_FT_Prob,
    W_M2_Bearish_FT_Prob = EXCLUDED.W_M2_Bearish_FT_Prob,
    W_M3_Continuation_Prob = EXCLUDED.W_M3_Continuation_Prob,
    W_M4_Bullish_Reversal_Risk = EXCLUDED.W_M4_Bullish_Reversal_Risk,
    W_M4_Bearish_Reversal_Risk = EXCLUDED.W_M4_Bearish_Reversal_Risk,
    W_M5_Bullish_Efficiency_Index = EXCLUDED.W_M5_Bullish_Efficiency_Index,
    W_M5_Bearish_Efficiency_Index = EXCLUDED.W_M5_Bearish_Efficiency_Index,
    W_M6_Exhaustion_Prob = EXCLUDED.W_M6_Exhaustion_Prob,
    Total_Transitions = EXCLUDED.Total_Transitions,
    Calculation_Date = NOW();