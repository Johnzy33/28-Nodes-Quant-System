------------------------
-- Daily Score
-----------------------

DROP TABLE asset_daily_composite_score;
-- D.C.S Target Table Definition (Ensures all logging columns are present)
CREATE TABLE IF NOT EXISTS asset_daily_composite_score (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,                   -- Final Daily Composite Score (Calculated with D.M2, D.M5, D.M1)
    DCS_Classification TEXT,                -- Final categorical output
    DCS TEXT,
    F_Reversal TEXT,              -- Active D.M5 Factor (1.0 - 0.2 * Risk)
    F_Commitment TEXT,            -- D.M3 Factor (0.8 + 0.4 * Prob)
    F_Sustainability TEXT,
    F_DM1_Factor TEXT,        -- D.M4 Factor (Follow-Through Ratio)
    
    PRIMARY KEY (trading_date, asset_id)
);
TRUNCATE TABLE asset_daily_composite_score;
WITH Metrics_Snapshot_Lookup AS (
        D.trading_date, 
        D.asset_id,
        S.Prior_Weekly_Type, 
        -- Pre-calculated DBS Scores (D.M2)
        S.DBS_ST_Score, 
        S.DBS_LT_Score, 
        S.DBS_YTD_Score,
        -- Raw M_n Probabilities (D.M3, D.M4, D.M5)
        S.D_M5_Bullish_Reversal_Risk, 
        S.D_M5_Bearish_Reversal_Risk,
        S.D_M3_Continuation_Prob,
        S.D_M4_Bullish_FT_Prob,
        S.D_M4_Bearish_FT_Prob
    FROM asset_daily_views D
    -- Join to the assumed pre-calculated metrics snapshot table
    INNER JOIN asset_daily_metrics_snapshot S 
        ON S.asset_id = D.asset_id 
        AND S.trading_date = D.trading_date
   -- WHERE D.trading_date = (SELECT MAX(trading_date) FROM asset_daily_views) -- Run only for the latest day
),
Daily_Composite_Score AS (
    -- This CTE performs all the final logic: DBS selection, factor calculation, and final score.
    SELECT
        D.trading_date, 
        D.asset_id,
        -- Step 2A: Select the DBS_Strongest_Signal (D.M2)
        (CASE
            -- Apply minimum threshold for a signal to be considered
            WHEN GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) > 0.25 
            THEN 
                (CASE
                    WHEN ABS(DBS_ST_Score) = GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) THEN DBS_ST_Score
                    WHEN ABS(DBS_LT_Score) = GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) THEN DBS_LT_Score
                    ELSE DBS_YTD_Score
                END)
            ELSE 0.0 
        END) AS DBS_Strongest_Signal,
        -- Step 2B: Calculate all Factors (F_Reversal and F_DM1 are ACTIVE; F_Commitment and F_Sustainability are LOGGING)
        -- F_Reversal (D.M5) - ACTIVE FILTER
        (CASE 
            WHEN DBS_ST_Score >= 0 THEN (1.0 - 0.2 * D_M5_Bullish_Reversal_Risk) 
            ELSE (1.0 - 0.2 * D_M5_Bearish_Reversal_Risk) 
        END) AS F_Reversal_Factor, 
        -- F_DM1_Factor (D.M1) - ACTIVE FILTER
        (CASE
            WHEN Prior_Weekly_Type IN ('Bullish') AND DBS_ST_Score >= 0 THEN 1.05
            WHEN Prior_Weekly_Type IN ('Bullish') AND DBS_ST_Score < 0 THEN 0.95
            WHEN Prior_Weekly_Type IN ('Bearish') AND DBS_ST_Score < 0 THEN 1.05
            WHEN Prior_Weekly_Type IN ('Bearish') AND DBS_ST_Score >= 0 THEN 0.95
            ELSE 1.00 
        END) AS F_DM1_Factor,
        -- F_Commitment (D.M3) - LOGGING ONLY
        (0.8 + 0.4 * D_M3_Continuation_Prob) AS F_Commitment, 
        -- F_Sustainability (D.M4) - LOGGING ONLY
        CASE 
            WHEN DBS_ST_Score >= 0 THEN 
                (D_M4_Bullish_FT_Prob + 0.1) / (1.0 - D_M4_Bullish_FT_Prob + 0.1) 
            ELSE 
                (D_M4_Bearish_FT_Prob + 0.1) / (1.0 - D_M4_Bearish_FT_Prob + 0.1) 
        END AS F_Sustainability,
        D_M5_Bullish_Reversal_Risk, 
        D_M5_Bearish_Reversal_Risk
    FROM Metrics_Snapshot_Lookup D
)
-- FINAL INSERT
INSERT INTO asset_daily_composite_score (
    trading_date, asset_id, DCS, DCS_Classification, 
    F_Reversal, F_DM1_Factor, F_Commitment, F_Sustainability
)
SELECT
    d.trading_date, d.asset_id,
    -- Step 2C: Final Multiplication
    ROUND((d.DBS_Strongest_Signal * d.F_Reversal_Factor * d.F_DM1_Factor * 100)::NUMERIC, 2):: TEXT || '%' AS DCS,    -- Step 2D: Final Classification
    CASE
        WHEN d.DBS_Strongest_Signal * d.F_Reversal_Factor * d.F_DM1_Factor >= 0.50 THEN 'High Conviction Long'
        WHEN d.DBS_Strongest_Signal * d.F_Reversal_Factor * d.F_DM1_Factor > 0.25 THEN 'Medium Conviction Long'
        WHEN d.DBS_Strongest_Signal * d.F_Reversal_Factor * d.F_DM1_Factor < -0.50 THEN 'High Conviction Short'
        WHEN d.DBS_Strongest_Signal * d.F_Reversal_Factor * d.F_DM1_Factor < -0.25 THEN 'Medium Conviction Short'
        ELSE 'Neutral/Low Conviction'
    END AS DCS_Classification,
    CASE
        WHEN d.DBS_Strongest_Signal >= 0 THEN ROUND((d.D_M5_Bullish_Reversal_Risk * 100)::NUMERIC, 2)::TEXT || '%'
        ELSE ROUND((d.D_M5_Bearish_Reversal_Risk * 100)::NUMERIC, 2)::TEXT || '%'
    END AS F_Reversal,
    -- Step 2E: Logging all Factors
    ROUND((d.F_DM1_Factor * 100)::NUMERIC, 2)::TEXT || '%' AS F_DM1_Factor,   
    ROUND((d.F_Commitment * 100)::NUMERIC, 2)::TEXT || '%' AS F_Commitment,   
    ROUND((d.F_Sustainability * 100)::NUMERIC, 2)::TEXT || '%' AS F_Sustainability
FROM Daily_Composite_Score d
ON CONFLICT (trading_date, asset_id) 
DO UPDATE SET
    DCS = EXCLUDED.DCS, DCS_Classification = EXCLUDED.DCS_Classification, 
    F_Reversal = EXCLUDED.F_Reversal, F_DM1_Factor = EXCLUDED.F_DM1_Factor,
    F_Commitment = EXCLUDED.F_Commitment, F_Sustainability = EXCLUDED.F_Sustainability;