------------------------------
-- Unified Confidence Score (UCS) Calculation
------------------------------

DROP TABLE IF EXISTS asset_unified_confidence_score CASCADE;
CREATE TABLE IF NOT EXISTS asset_unified_confidence_score (
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    ps1_name TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    trade_direction TEXT NOT NULL,                  -- BULLISH, BEARISH, or NEUTRAL
    ps1_7_state TEXT NOT NULL,                      -- PS1's 7-state bias
    predicted_cs_bias TEXT NOT NULL,                -- The specific 7-state outcome predicted by PCS
    pcs_base_score DOUBLE PRECISION NOT NULL,       -- Base score (Metric 6)
    tcs_score DOUBLE PRECISION NOT NULL,            -- Trend Stability (Metric 8)
    cvi_score DOUBLE PRECISION NOT NULL,            -- Consolidation Risk (Metric 9)
    fhr_veto_score DOUBLE PRECISION NOT NULL,       -- Reversal Veto (Metric 7)
    unified_confidence_score DOUBLE PRECISION NOT NULL, -- Final UCS calculation
    final_signal_label TEXT NOT NULL,               -- Final decision-making label
    PRIMARY KEY (asset_id, trading_date, ps1_name, cs_name)
);
TRUNCATE asset_unified_confidence_score;

-- =================================================================================
-- METRIC 10: UNIFIED CONFIDENCE SCORE (UCS) - GENERALIZED
-- =================================================================================
WITH latest_data AS (
    -- 1. Determine the latest completed trading date (generalized source)
    SELECT MAX(trading_date) AS current_trading_date FROM asset_session_context
),
base_context AS (
    -- 2. Pull base context and biases for the latest date
    SELECT DISTINCT ON (uc.asset_id, uc.trading_date, uc.ps1_name, uc.cs_name)
        uc.asset_id,
        uc.trading_date,
        uc.cs_ps1_fk,
        uc.ps1_name,
        uc.cs_name,
        uc.ps1_bias_7_state AS ps1_7_state, -- PS1 7-state bias from context table
        upc.pcs_score AS pcs_base_score,
        upc.daily_outcome_7 AS predicted_cs_bias,
        -- Infer Trade Direction from the daily_outcome_7 predicted by PCS
        CASE
            WHEN upc.daily_outcome_7 IN ('Bullish', 'Bullish_Reversal', 'Failed_Bearish', 'Bullish_Consolidation') THEN 'BULLISH'
            WHEN upc.daily_outcome_7 IN ('Bearish', 'Bearish_Reversal', 'Failed_Bullish', 'Bearish_Consolidation') THEN 'BEARISH'
            ELSE 'NEUTRAL'
        END AS trade_direction
        
    FROM asset_session_context uc -- <<< GENERALIZED CONTEXT
    JOIN asset_predictive_confidence_score upc -- <<< GENERALIZED PCS
        ON upc.cs_ps1_fk = uc.cs_ps1_fk
        AND upc.asset_id = uc.asset_id
    CROSS JOIN latest_data ld
    WHERE uc.trading_date = ld.current_trading_date
      AND upc.daily_outcome_7 NOT IN ('Pure_Indecision', 'Other') -- Filter out pure noise predictions
    ORDER BY uc.asset_id, uc.trading_date, uc.ps1_name, uc.cs_name, upc.pcs_score DESC 
),
full_ucs_components AS (
    -- 3. Perform the joins to TCS, CVI, and FHR using the Foreign Keys
    SELECT
        bc.*, -- Base context fields
        -- TCS: Uses PS2_PS1_FK (Trend Stability)
        COALESCE(tcs.tcs_score, 60.0) AS tcs_score, 
        -- CVI: Uses CS_PS1_FK (Consolidation Risk Veto)
        -- Only apply CVI if PS1 was actually a consolidation type, otherwise use neutral 60.0
        COALESCE(ucv.cvi_score, 60.0) AS cvi_score,
        -- FHR: Uses CS_PS1_FK (Reversal Veto)
        COALESCE(ufr.fhr_score, 0.0) AS fhr_veto_score
    FROM base_context bc
    -- NOTE: Join to context to get PS2_PS1_FK needed for TCS
    JOIN asset_session_context uc 
        ON bc.cs_ps1_fk = uc.cs_ps1_fk
        AND bc.asset_id = uc.asset_id
        AND bc.trading_date = uc.trading_date
    -- --- TCS JOIN (Metric 8) ---
    LEFT JOIN asset_trend_stability_score tcs -- <<< GENERALIZED TCS
        ON tcs.ps2_ps1_fk = uc.ps2_ps1_fk
        AND tcs.trend_direction = bc.trade_direction 
    -- --- CVI JOIN (Metric 9) ---
    LEFT JOIN asset_consolidation_volatility_index ucv -- <<< GENERALIZED CVI
        ON ucv.cs_ps1_fk = bc.cs_ps1_fk
        AND ucv.asset_id = bc.asset_id
    -- --- FHR JOIN (Metric 7) ---
    LEFT JOIN asset_fhr_reversal_score ufr -- <<< GENERALIZED FHR
        ON ufr.cs_ps1_fk = bc.cs_ps1_fk
        AND ufr.asset_id = bc.asset_id
        AND ufr.pattern_type = (
            CASE bc.trade_direction
                WHEN 'BULLISH' THEN 'PDL_Break_to_Bullish' -- Bullish trade is reversed by a Bullish failure (PDL break)
                WHEN 'BEARISH' THEN 'PDH_Break_to_Bearish' -- Bearish trade is reversed by a Bearish failure (PDH break)
                ELSE NULL
            END
        )
),
final_calculation AS (
    -- 4. Calculate UCS 
    SELECT
        *,         
        -- UCS = PCS * (TCS / 60) * (CVI / 60) * VETO_FACTOR
        pcs_base_score
        * (tcs_score / 60.0)
        * (cvi_score / 60.0)
        * (CASE WHEN fhr_veto_score >= 60.0 THEN 0.20 ELSE 1.0 END) AS unified_confidence_score
    FROM full_ucs_components
),
final_signal_output AS (
    -- 5. Generate the final label
    SELECT
        *,
        -- GENERATE FINAL SIGNAL LABEL
        CASE
            WHEN unified_confidence_score >= 90.0 THEN 'Extreme ' || trade_direction || ' (Aggressive Entry)'
            WHEN unified_confidence_score >= 80.0 THEN 'Strong ' || trade_direction || ' (High Confidence)'
            WHEN unified_confidence_score >= 60.0 THEN 'Moderate ' || trade_direction || ' (Standard Entry)'
            ELSE 'Weak ' || trade_direction || ' (Avoid/Monitor)'
        END AS final_signal_label
    FROM final_calculation
)
-- 6. Final UPSERT: Insert/Update the complete UCS record
INSERT INTO asset_unified_confidence_score (
    asset_id, trading_date, ps1_name, cs_name, trade_direction, ps1_7_state, predicted_cs_bias,
    pcs_base_score, tcs_score, cvi_score, fhr_veto_score, unified_confidence_score, final_signal_label
)
SELECT
    asset_id, trading_date, ps1_name, cs_name, trade_direction, ps1_7_state, predicted_cs_bias,
    pcs_base_score, tcs_score, cvi_score, fhr_veto_score, unified_confidence_score, final_signal_label
FROM final_signal_output
ON CONFLICT (asset_id, trading_date, ps1_name, cs_name) 
DO UPDATE SET
    trade_direction = EXCLUDED.trade_direction,
    ps1_7_state = EXCLUDED.ps1_7_state,
    predicted_cs_bias = EXCLUDED.predicted_cs_bias,
    pcs_base_score = EXCLUDED.pcs_base_score,
    tcs_score = EXCLUDED.tcs_score,
    cvi_score = EXCLUDED.cvi_score,
    fhr_veto_score = EXCLUDED.fhr_veto_score,
    unified_confidence_score = EXCLUDED.unified_confidence_score,
    final_signal_label = EXCLUDED.final_signal_label;