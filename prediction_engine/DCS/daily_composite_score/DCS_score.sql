--------------------------------------------
------- Daily DCS Score
-----------------------------------------

DROP TABLE IF EXISTS daily_composite_score CASCADE;

CREATE TABLE IF NOT EXISTS daily_composite_score (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    -- Final Score and Classification
    DCS NUMERIC(10, 4),                 -- Store as a number (0.0 to 1.0)
    DCS_Classification TEXT,            -- Final categorical output
    -- Factors (Stored as raw numbers for analysis)
    F_Reversal NUMERIC(10, 4),          
    F_DM1_Factor NUMERIC(10, 4),        
    F_Commitment NUMERIC(10, 4),        
    F_Sustainability NUMERIC(10, 4),
    
    PRIMARY KEY (trading_date, asset_id)
);
SELECT create_hypertable('daily_composite_score', 'trading_date', if_not_exists => TRUE);
CALL refresh_daily_composite_score();

CREATE OR REPLACE PROCEDURE refresh_daily_composite_score()
LANGUAGE sql
AS $$
INSERT INTO daily_composite_score (
    trading_date, asset_id, DCS, DCS_Classification, 
    F_Reversal, F_DM1_Factor, F_Commitment, F_Sustainability
)
WITH Metrics_Snapshot_Lookup AS (
    SELECT
        D.trading_date, 
        D.asset_id,
        S.Prior_Weekly_Type, 
        S.DBS_ST_Score, S.DBS_LT_Score, S.DBS_YTD_Score,
        S.D_M5_Bullish_Reversal_Risk, S.D_M5_Bearish_Reversal_Risk,
        S.D_M3_Continuation_Prob, S.D_M4_Bullish_FT_Prob, S.D_M4_Bearish_FT_Prob
    FROM daily_views D
    -- Join to the corrected metrics snapshot table
    INNER JOIN daily_metrics_snapshot S 
        ON S.asset_id = D.asset_id 
        AND S.trading_date = D.trading_date
),
Daily_Composite_Score AS (
    -- This CTE performs all the final logic: DBS selection, factor calculation, and final score.
    SELECT
        D.trading_date, 
        D.asset_id,
        -- Step 1: Select the DBS_Strongest_Signal (D.M2)
        (CASE
            WHEN GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) > 0.25 
            THEN 
                (CASE
                    WHEN ABS(DBS_ST_Score) = GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) THEN DBS_ST_Score
                    WHEN ABS(DBS_LT_Score) = GREATEST(ABS(DBS_ST_Score), ABS(DBS_LT_Score), ABS(DBS_YTD_Score)) THEN DBS_LT_Score
                    ELSE DBS_YTD_Score
                END)
            ELSE 0.0 
        END) AS DBS_Strongest_Signal,
        -- Step 2: Calculate all Factors (NUMERIC outputs)
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
        END AS F_Sustainability
    FROM Metrics_Snapshot_Lookup D
)
-- FINAL INSERT
SELECT
    d.trading_date, d.asset_id,
    -- Final Score (Stored as a NUMERIC)
    ROUND((d.DBS_Strongest_Signal * d.F_Reversal_Factor * d.F_DM1_Factor)::NUMERIC, 4) AS DCS,
    -- Final Classification
    CASE
        WHEN d.DBS_Strongest_Signal * d.F_Reversal_Factor * d.F_DM1_Factor >= 0.50 THEN 'High Conviction Long'
        WHEN d.DBS_Strongest_Signal * d.F_Reversal_Factor * d.F_DM1_Factor > 0.25 THEN 'Medium Conviction Long'
        WHEN d.DBS_Strongest_Signal * d.F_Reversal_Factor * d.F_DM1_Factor < -0.50 THEN 'High Conviction Short'
        WHEN d.DBS_Strongest_Signal * d.F_Reversal_Factor * d.F_DM1_Factor < -0.25 THEN 'Medium Conviction Short'
        ELSE 'Neutral/Low Conviction'
    END AS DCS_Classification,
    -- Factor Logging (Stored as NUMERIC)
    ROUND(d.F_Reversal_Factor::NUMERIC, 4) AS F_Reversal,
    ROUND(d.F_DM1_Factor::NUMERIC, 4) AS F_DM1_Factor,   
    ROUND(d.F_Commitment::NUMERIC, 4) AS F_Commitment,   
    ROUND(d.F_Sustainability::NUMERIC, 4) AS F_Sustainability
FROM Daily_Composite_Score d
ON CONFLICT (trading_date, asset_id) 
DO UPDATE SET
    DCS = EXCLUDED.DCS, DCS_Classification = EXCLUDED.DCS_Classification, 
    F_Reversal = EXCLUDED.F_Reversal, F_DM1_Factor = EXCLUDED.F_DM1_Factor,
    F_Commitment = EXCLUDED.F_Commitment, F_Sustainability = EXCLUDED.F_Sustainability;
$$;