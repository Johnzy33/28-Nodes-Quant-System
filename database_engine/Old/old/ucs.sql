
WITH latest_data AS (
    -- 1. Determine the latest completed trading date in the system
    SELECT MAX(trading_date) AS current_trading_date FROM us2000_session_context
),
target_pcs_signal AS (
    -- 2. Pull the PCS base score and necessary FK for the latest trading date.
    --    CRITICAL FIX: Use DISTINCT ON to ensure only ONE record per unique UCS Primary Key 
    --    is passed, preventing the "affect row a second time" error.
    SELECT DISTINCT ON (uc.asset_id, uc.trading_date, uc.ps1_name, uc.cs_name)
        upc.asset_id,
        uc.trading_date,
        uc.ps1_name,
        uc.cs_name,
        upc.pcs_score AS pcs_base_score,
        upc.cs_ps1_fk, -- The Foreign Key to the context (PS1->CS)
        -- Infer Trade Direction from the daily_outcome_7 
        CASE
            WHEN upc.daily_outcome_7 IN ('Bullish', 'Bullish_Reversal', 'Failed_Bearish') THEN 'BULLISH'
            WHEN upc.daily_outcome_7 IN ('Bearish', 'Bearish_Reversal', 'Failed_Bullish') THEN 'BEARISH'
            ELSE 'NEUTRAL'
        END AS trade_direction
    FROM predictive_confidence_score upc
    -- Join to Context to restrict to the latest date and get context fields
    JOIN asset_session_context uc 
        ON upc.cs_ps1_fk = uc.cs_ps1_fk
    CROSS JOIN latest_data ld
    WHERE uc.trading_date = ld.current_trading_date
      AND upc.daily_outcome_7 NOT IN ('Pure_Indecision', 'Other') -- Filter out non-directional signals
    -- This ORDER BY ensures consistent selection if duplicate PCS records existed for a key
    ORDER BY uc.asset_id, uc.trading_date, uc.ps1_name, uc.cs_name, upc.pcs_score DESC 
),
full_ucs_components AS (
    -- 3. Perform the joins to TCS, CVI, and FHR using the unique Foreign Keys
    SELECT
        p.trading_date,
        p.asset_id,
        p.ps1_name,
        p.cs_name,
        p.trade_direction,
        p.pcs_base_score,
        -- TCS: Use PS2_PS1_FK (structural stability)
        COALESCE(tcs.tcs_score, 60.0) AS tcs_score,
        -- CVI: Use CS_PS1_FK (consolidation risk)
        COALESCE(CASE
            WHEN uc.ps1_7_state LIKE 'Consolidation%' THEN ucv.cvi_score
            ELSE 60.0 -- Neutral if PS1 was a directional session
        END, 60.0) AS cvi_score,
        -- FHR: Use CS_PS1_FK (reversal veto)
        COALESCE(ufr.fhr_score, 0.0) AS fhr_veto_score
    FROM target_pcs_signal p
    -- Retrieve full context record to get the PS1 7-state bias for CVI condition
    JOIN us2000_session_context uc 
        ON p.cs_ps1_fk = uc.cs_ps1_fk
    -- --- TCS JOIN (The PS2 Context) ---
    LEFT JOIN us2000_trend_stability_score tcs
        ON tcs.ps2_ps1_fk = uc.ps2_ps1_fk -- DIRECT FK JOIN
        AND tcs.trend_direction = p.trade_direction 
    -- --- CVI JOIN (Consolidation Risk) ---
    LEFT JOIN us2000_consolidation_volatility_index ucv
        ON ucv.cs_ps1_fk = p.cs_ps1_fk -- DIRECT FK JOIN
    -- --- FHR JOIN (Reversal Veto) ---
    LEFT JOIN us2000_fhr_reversal_score ufr
        ON ufr.cs_ps1_fk = p.cs_ps1_fk -- DIRECT FK JOIN
        AND ufr.pattern_type = (
            -- Placeholder logic for FHR pattern type match (based on direction)
            CASE p.trade_direction
                WHEN 'BULLISH' THEN 'PDL_Break_to_Bullish'
                WHEN 'BEARISH' THEN 'PDH_Break_to_Bearish'
                ELSE NULL
            END
        )
)
-- 4. Final UCS Calculation and UPSERT
INSERT INTO us2000_ucs_score (
    asset_id, trading_date, ps1_name, cs_name, trade_direction, pcs_base_score,
    tcs_score, cvi_score, fhr_veto_score, unified_confidence_score
)
SELECT
    asset_id, trading_date, ps1_name, cs_name, trade_direction, pcs_base_score,
    tcs_score, cvi_score, fhr_veto_score,
    -- CALCULATE FINAL UCS
    pcs_base_score
    * (tcs_score / 60.0)
    * (cvi_score / 60.0)
    -- FHR VETO LOGIC
    * (CASE
        WHEN fhr_veto_score >= 60.0 THEN 0.20 
        ELSE 1.0
    END) AS unified_confidence_score
FROM full_ucs_components
-- *** UPSERT LOGIC TO AVOID PRIMARY KEY VIOLATIONS ***
ON CONFLICT (asset_id, trading_date, ps1_name, cs_name) 
DO UPDATE SET
    trade_direction = EXCLUDED.trade_direction,
    pcs_base_score = EXCLUDED.pcs_base_score,
    tcs_score = EXCLUDED.tcs_score,
    cvi_score = EXCLUDED.cvi_score,
    fhr_veto_score = EXCLUDED.fhr_veto_score,
    unified_confidence_score = EXCLUDED.unified_confidence_score;


------------------------

WITH latest_data AS (
    -- 1. Determine the latest completed trading date
    SELECT MAX(trading_date) AS current_trading_date FROM us2000_session_context
),
base_context AS (
    -- 2. Pull base context and biases for the latest date, ensuring uniqueness
    SELECT DISTINCT ON (uc.asset_id, uc.trading_date, uc.ps1_name, uc.cs_name)
        uc.asset_id,
        uc.trading_date,
        uc.ps1_name,
        uc.cs_name,
        uc.ps1_7_state, -- <<< PS BIAS ADDED HERE (from Context)
        upc.pcs_score AS pcs_base_score,
        upc.cs_ps1_fk, 
        upc.daily_outcome_7 AS predicted_cs_bias, -- Predicted CS Bias
        -- Infer Trade Direction from the daily_outcome_7 
        CASE
            WHEN upc.daily_outcome_7 IN ('Bullish', 'Bullish_Reversal', 'Failed_Bearish') THEN 'BULLISH'
            WHEN upc.daily_outcome_7 IN ('Bearish', 'Bearish_Reversal', 'Failed_Bullish') THEN 'BEARISH'
            ELSE 'NEUTRAL'
        END AS trade_direction
    FROM asset_session_context uc 
    -- Join to PCS to get the base score and predicted CS bias
    JOIN us2000_predictive_confidence_score upc 
        ON upc.cs_ps1_fk = uc.cs_ps1_fk
    CROSS JOIN latest_data ld
    WHERE uc.trading_date = ld.current_trading_date
      AND upc.daily_outcome_7 NOT IN ('Pure_Indecision', 'Other')
    ORDER BY uc.asset_id, uc.trading_date, uc.ps1_name, uc.cs_name, upc.pcs_score DESC 
),
full_ucs_components AS (
    -- 3. Perform the joins to TCS, CVI, and FHR using the unique Foreign Keys
    SELECT
        bc.*, -- Select all fields from the base context CTE    
        -- TCS: Use PS2_PS1_FK (structural stability)
        COALESCE(tcs.tcs_score, 60.0) AS tcs_score, 
        -- CVI: Use CS_PS1_FK (consolidation risk)
        COALESCE(CASE
            WHEN bc.ps1_7_state LIKE 'Consolidation%' THEN ucv.cvi_score
            ELSE 60.0 
        END, 60.0) AS cvi_score,
        -- FHR: Use CS_PS1_FK (reversal veto)
        COALESCE(ufr.fhr_score, 0.0) AS fhr_veto_score
    FROM base_context bc
    -- NOTE: We join back to the context table to get PS2_PS1_FK for TCS
    JOIN us2000_session_context uc 
        ON bc.cs_ps1_fk = uc.cs_ps1_fk
    -- --- TCS JOIN (The PS2 Context) ---
    LEFT JOIN us2000_trend_stability_score tcs
        ON tcs.ps2_ps1_fk = uc.ps2_ps1_fk
        AND tcs.trend_direction = bc.trade_direction 
    -- --- CVI JOIN (Consolidation Risk) ---
    LEFT JOIN us2000_consolidation_volatility_index ucv
        ON ucv.cs_ps1_fk = bc.cs_ps1_fk 
    -- --- FHR JOIN (Reversal Veto) ---
    LEFT JOIN us2000_fhr_reversal_score ufr
        ON ufr.cs_ps1_fk = bc.cs_ps1_fk
        AND ufr.pattern_type = (
            CASE bc.trade_direction
                WHEN 'BULLISH' THEN 'PDL_Break_to_Bullish'
                WHEN 'BEARISH' THEN 'PDH_Break_to_Bearish'
                ELSE NULL
            END
        )
),
final_calculation AS (
    -- 4. Calculate UCS 
    SELECT
        *,         
        -- CALCULATE FINAL UCS
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
INSERT INTO us2000_ucs_score (
    asset_id, trading_date, ps1_name, cs_name, trade_direction, pcs_base_score,
    tcs_score, cvi_score, fhr_veto_score, unified_confidence_score, 
    predicted_cs_bias, ps1_7_state, final_signal_label -- <<< ALL NEW FIELDS INCLUDED
)
SELECT
    asset_id, trading_date, ps1_name, cs_name, trade_direction, pcs_base_score,
    tcs_score, cvi_score, fhr_veto_score, unified_confidence_score, 
    predicted_cs_bias, ps1_7_state, final_signal_label
FROM final_signal_output
ON CONFLICT (asset_id, trading_date, ps1_name, cs_name) 
DO UPDATE SET
    trade_direction = EXCLUDED.trade_direction,
    pcs_base_score = EXCLUDED.pcs_base_score,
    tcs_score = EXCLUDED.tcs_score,
    cvi_score = EXCLUDED.cvi_score,
    fhr_veto_score = EXCLUDED.fhr_veto_score,
    unified_confidence_score = EXCLUDED.unified_confidence_score,
    predicted_cs_bias = EXCLUDED.predicted_cs_bias,
    ps1_7_state = EXCLUDED.ps1_7_state, -- <<< PS BIAS UPDATE
    final_signal_label = EXCLUDED.final_signal_label;
```eof

This query is now complete and includes the **`ps1_7_state`** (PS Bias), the **`predicted_cs_bias`**, and the **`final_signal_label`** in the `us2000_ucs_score` table, making it a fully self-contained signal record.

Would you like to write the final query to **extract the live, actionable trade signals** from this comprehensive table?