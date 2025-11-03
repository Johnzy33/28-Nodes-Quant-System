
---------------
-- Daily Matric Table D.M1, D.M2, D.M3, D.M4, D.M5
---------------
DROP TABLE IF EXISTS asset_daily_conditional_metrics;

CREATE TABLE asset_daily_conditional_metrics (
    asset_id TEXT NOT NULL,
    -- Conditional Keys (The composite primary key for the lookup)
    DOW_Transition TEXT NOT NULL,         -- The transition, e.g., 'Mon -> Tue' (from D.M1, D.M2, D.M3, D.M4)
    PD_Type TEXT NOT NULL,                -- Prior Day Type, e.g., 'Strong_Bull' (from D.M1, D.M2, D.M3)
    -- D.M1: Daily Pattern Bias
    D_M1_Bullish_Prob DOUBLE PRECISION,
    D_M1_Bearish_Prob DOUBLE PRECISION,
    -- D.M2: Daily Structural Takedown
    D_M2_PDH_Prob NUMERIC(10, 4),       -- Probability of Prior Day High break
    D_M2_PDL_Prob NUMERIC(10, 4),       -- Probability of Prior Day Low break
    D_M2_PWH_Prob NUMERIC(10, 4),       -- Probability of Prior Week High break
    D_M2_PWL_Prob NUMERIC(10, 4),       -- Probability of Prior Week Low break
    -- D.M3: Daily DOW Stability
    D_M3_Continuation_Prob NUMERIC(10, 4),
    -- D.M4: Daily FT/FR Outcome
    D_M4_Bullish_FT_Prob NUMERIC(10, 4),
    D_M4_Bearish_FT_Prob NUMERIC(10, 4),
    -- D.M5: Daily DOW Reversal Risk (Note: this is only conditional on DOW, so we must link it)
    -- We will insert D.M5 values into every row for the corresponding CD_DOW.
    D_M5_Bullish_Reversal_Risk NUMERIC(10, 4),
    D_M5_Bearish_Reversal_Risk NUMERIC(10, 4),

    Total_Transitions INTEGER NOT NULL,  -- Count of samples used for this key
    Calculation_Date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    PRIMARY KEY(asset_id, DOW_Transition, PD_Type)
);

TRUNCATE asset_daily_conditional_metrics;
-- NOTE: This CTE runs the calculations for all five D.Mn matrices simultaneously.
-- We use LEFT JOINs to merge all the conditional probability sets together.


-- This CTE calculates and inserts all D.M1 through D.M5 conditional probabilities
-- into the asset_daily_conditional_metrics lookup table.
TRUNCATE asset_daily_conditional_metrics;
WITH D_M1_Probabilities AS (
    -- D.M1: Daily Pattern Bias (Conditional on asset_id, PD_Type and DOW Transition)
    SELECT
        asset_id, -- ADDED HERE
        PD_Type,
        PD_DOW || ' -> ' || CD_DOW AS DOW_Transition,
        COUNT(*) AS Total_Transitions,
        SUM(CASE WHEN NYAM_Bias = 'Bullish' THEN 1 ELSE 0 END)::DOUBLE PRECISION / COUNT(*) AS D_M1_Bullish_Prob,
        SUM(CASE WHEN NYAM_Bias = 'Bearish' THEN 1 ELSE 0 END)::DOUBLE PRECISION / COUNT(*) AS D_M1_Bearish_Prob
    FROM (
        SELECT LAG(Day_Type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS PD_Type,
               LAG(DOW, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS PD_DOW,
               DOW AS CD_DOW,
               NYAM_Bias,
               asset_id 
        FROM asset_daily_views
    ) AS D1_Base
    WHERE PD_Type IS NOT NULL AND asset_id IS NOT NULL 
    GROUP BY asset_id, PD_Type, DOW_Transition -- Use explicit names for clarity
    HAVING COUNT(*) >= 0
),
D_M2_Probabilities AS (
    -- D.M2: Daily Structural Takedown (Conditional on asset_id, PD_Type and DOW Transition)
    SELECT
        asset_id,
        PD_Type,
        PD_DOW || ' -> ' || CD_DOW AS DOW_Transition,
        SUM(PDH_Break)::DOUBLE PRECISION / COUNT(*) AS D_M2_PDH_Prob,
        SUM(PWH_Break)::DOUBLE PRECISION / COUNT(*) AS D_M2_PWH_Prob,
        SUM(PDL_Break)::DOUBLE PRECISION / COUNT(*) AS D_M2_PDL_Prob,
        SUM(PWL_Break)::DOUBLE PRECISION / COUNT(*) AS D_M2_PWL_Prob
    FROM (
        SELECT LAG(Day_Type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS PD_Type,
               LAG(DOW, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS PD_DOW,
               DOW AS CD_DOW,
               asset_id,
               CASE WHEN high > Prior_Day_High THEN 1 ELSE 0 END AS PDH_Break,
               CASE WHEN low < Prior_Day_Low THEN 1 ELSE 0 END AS PDL_Break,
               CASE WHEN high > Prior_Week_High THEN 1 ELSE 0 END AS PWH_Break,
               CASE WHEN low < Prior_Week_Low THEN 1 ELSE 0 END AS PWL_Break
        FROM asset_daily_views
    ) AS D2_Base
    WHERE PD_Type IS NOT NULL
    GROUP BY 1, 2, 3
),
D_M3_Probabilities AS (
    -- D.M3: Daily DOW Stability (Conditional on asset_id, PD_Type and DOW Transition, where PD_Type is Strong)
    SELECT
        asset_id,
        PD_Type,
        PD_DOW || ' -> ' || CD_DOW AS DOW_Transition,
        SUM(Continuation)::DOUBLE PRECISION / COUNT(*) AS D_M3_Continuation_Prob
    FROM (
        SELECT LAG(Day_Type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS PD_Type,
               LAG(DOW, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS PD_DOW,
               DOW AS CD_DOW,
               asset_id,
               CASE WHEN LAG(Day_Type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) = 'Bullish' AND Day_Type = 'Bullish' THEN 1
                    WHEN LAG(Day_Type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) = 'Bearish' AND Day_Type = 'Bearish' THEN 1
                    ELSE 0 END AS Continuation
        FROM asset_daily_views
    ) AS D3_Base
    WHERE PD_Type IN ('Bullish', 'Bearish')
    GROUP BY 1, 2, 3
),
D_M4_Probabilities AS (
    -- D.M4: Daily FT/FR Outcome (Conditional on asset_id, PD_Type and DOW Transition)
    SELECT
        asset_id,
        PD_Type,
        DOW_Transition,
        SUM(CASE WHEN Prior_Break_Type = 'Bullish_Break' AND Day_Close_Type = 'Bullish_Close' THEN 1 ELSE 0 END)::DOUBLE PRECISION /
            NULLIF(SUM(CASE WHEN Prior_Break_Type = 'Bullish_Break' THEN 1 ELSE 0 END), 0) AS D_M4_Bullish_FT_Prob,
        SUM(CASE WHEN Prior_Break_Type = 'Bearish_Break' AND Day_Close_Type = 'Bearish_Close' THEN 1 ELSE 0 END)::DOUBLE PRECISION /
            NULLIF(SUM(CASE WHEN Prior_Break_Type = 'Bearish_Break' THEN 1 ELSE 0 END), 0) AS D_M4_Bearish_FT_Prob
    FROM (
        SELECT Day_Close_Type, DOW, asset_id,
               LAG(Day_Type, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS PD_Type,
               LAG(DOW, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) || ' -> ' || DOW AS DOW_Transition,
               CASE WHEN LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) > LAG(Prior_Day_High, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) THEN 'Bullish_Break'
                    WHEN LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) < LAG(Prior_Day_Low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) THEN 'Bearish_Break'
                    ELSE 'No_Break' END AS Prior_Break_Type
        FROM asset_daily_views
    ) AS D4_Base
    WHERE Prior_Break_Type != 'No_Break'
    GROUP BY 1, 2, 3
),
D_M5_Probabilities AS (
    -- D.M5: Daily DOW Reversal Risk (Conditional ONLY on asset_id and Current Day DOW)
    SELECT
        asset_id,
        DOW AS CD_DOW,
        SUM(CASE WHEN high > Prior_Day_High AND (high - close) / (high - low) >= 0.70 THEN 1 ELSE 0 END)::DOUBLE PRECISION /
            NULLIF(SUM(CASE WHEN high > Prior_Day_High THEN 1 ELSE 0 END), 0) AS D_M5_Bullish_Reversal_Risk,
        SUM(CASE WHEN low < Prior_Day_Low AND (close - low) / (high - low) <= 0.30 THEN 1 ELSE 0 END)::DOUBLE PRECISION /
            NULLIF(SUM(CASE WHEN low < Prior_Day_Low THEN 1 ELSE 0 END), 0) AS D_M5_Bearish_Reversal_Risk
    FROM asset_daily_views
    GROUP BY 1, 2
),
Merged_Metrics AS (
    -- Merge D.M1, D.M2, D.M3, and D.M4 using the composite key (asset_id, DOW_Transition, PD_Type)
    SELECT
        M1.asset_id,
        M1.PD_Type,
        M1.DOW_Transition,
        M1.Total_Transitions,
        M1.D_M1_Bullish_Prob, M1.D_M1_Bearish_Prob,
        COALESCE(M2.D_M2_PDH_Prob, 0.5) AS D_M2_PDH_Prob,
        COALESCE(M2.D_M2_PDL_Prob, 0.5) AS D_M2_PDL_Prob,
        COALESCE(M2.D_M2_PWH_Prob, 0.5) AS D_M2_PWH_Prob,
        COALESCE(M2.D_M2_PWL_Prob, 0.5) AS D_M2_PWL_Prob,
        COALESCE(M3.D_M3_Continuation_Prob, 0.5) AS D_M3_Continuation_Prob,
        COALESCE(M4.D_M4_Bullish_FT_Prob, 0.5) AS D_M4_Bullish_FT_Prob,
        COALESCE(M4.D_M4_Bearish_FT_Prob, 0.5) AS D_M4_Bearish_FT_Prob
    FROM D_M1_Probabilities M1
    LEFT JOIN D_M2_Probabilities M2 ON M1.DOW_Transition = M2.DOW_Transition AND M1.PD_Type = M2.PD_Type AND M1.asset_id = M2.asset_id
    LEFT JOIN D_M3_Probabilities M3 ON M1.DOW_Transition = M3.DOW_Transition AND M1.PD_Type = M3.PD_Type AND M1.asset_id = M3.asset_id
    LEFT JOIN D_M4_Probabilities M4 ON M1.DOW_Transition = M4.DOW_Transition AND M1.PD_Type = M4.PD_Type AND M1.asset_id = M4.asset_id
)
-- Final Insertion: Merge D.M5 (DOW-only) with the other metrics
INSERT INTO asset_daily_conditional_metrics (
    asset_id, DOW_Transition, PD_Type, Total_Transitions,
    D_M1_Bullish_Prob, D_M1_Bearish_Prob,
    D_M2_PDH_Prob, D_M2_PDL_Prob, D_M2_PWH_Prob, D_M2_PWL_Prob,
    D_M3_Continuation_Prob,
    D_M4_Bullish_FT_Prob, D_M4_Bearish_FT_Prob,
    D_M5_Bullish_Reversal_Risk, D_M5_Bearish_Reversal_Risk
)
SELECT
    MM.asset_id,
    MM.DOW_Transition,
    MM.PD_Type,
    MM.Total_Transitions,
    ROUND(MM.D_M1_Bullish_Prob::NUMERIC, 4), ROUND(MM.D_M1_Bearish_Prob::NUMERIC, 4),
    ROUND(MM.D_M2_PDH_Prob::NUMERIC, 4), ROUND(MM.D_M2_PDL_Prob::NUMERIC, 4), ROUND(MM.D_M2_PWH_Prob::NUMERIC, 4), ROUND(MM.D_M2_PWL_Prob::NUMERIC, 4),
    ROUND(MM.D_M3_Continuation_Prob::NUMERIC, 4),
    ROUND(MM.D_M4_Bullish_FT_Prob::NUMERIC, 4), ROUND(MM.D_M4_Bearish_FT_Prob::NUMERIC, 4),
    -- Join D.M5 using the Current Day DOW derived from the DOW_Transition string
    M5.D_M5_Bullish_Reversal_Risk, M5.D_M5_Bearish_Reversal_Risk
FROM Merged_Metrics MM
LEFT JOIN D_M5_Probabilities M5
    ON M5.asset_id = MM.asset_id
    AND M5.CD_DOW = TRIM(SUBSTRING(MM.DOW_Transition FROM '-> (.*)'))
ON CONFLICT (asset_id, DOW_Transition, PD_Type)
DO UPDATE SET
    D_M1_Bullish_Prob = EXCLUDED.D_M1_Bullish_Prob,
    D_M1_Bearish_Prob = EXCLUDED.D_M1_Bearish_Prob,
    D_M2_PDH_Prob = EXCLUDED.D_M2_PDH_Prob,
    D_M2_PDL_Prob = EXCLUDED.D_M2_PDL_Prob,
    D_M3_Continuation_Prob = EXCLUDED.D_M3_Continuation_Prob,
    D_M4_Bullish_FT_Prob = EXCLUDED.D_M4_Bullish_FT_Prob,
    D_M4_Bearish_FT_Prob = EXCLUDED.D_M4_Bearish_FT_Prob,
    D_M5_Bullish_Reversal_Risk = EXCLUDED.D_M5_Bullish_Reversal_Risk,
    D_M5_Bearish_Reversal_Risk = EXCLUDED.D_M5_Bearish_Reversal_Risk,
    Total_Transitions = EXCLUDED.Total_Transitions,
    Calculation_Date = NOW();

