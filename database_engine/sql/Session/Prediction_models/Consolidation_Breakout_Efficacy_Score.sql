-----------------
-- CBES Model: Consolidation Breakout Efficacy Score
-----------------

DROP TABLE IF EXISTS asset_cbes_score CASCADE;
CREATE TABLE IF NOT EXISTS asset_cbes_score (
    asset_id TEXT NOT NULL,
    cs_ps1_fk BIGINT NOT NULL,
    ps1_name TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    trade_direction TEXT NOT NULL,                  -- The predicted direction of the breakout (Bullish/Bearish)
    
    cvi_score DOUBLE PRECISION NOT NULL,            -- M9 component: P(Takedown)
    p_follow_through DOUBLE PRECISION NOT NULL,     -- M4 component: P(Follow Through)
    p_directional_day DOUBLE PRECISION NOT NULL,    -- M5 component: P(Bullish/Bearish Day Type)
    
    cbes_score DOUBLE PRECISION NOT NULL,           -- Final Aggregated Score
    efficacy_label TEXT NOT NULL,                   -- 'High Efficacy', 'Low Efficacy'
    
    PRIMARY KEY (cs_ps1_fk, trade_direction)
);
TRUNCATE asset_cbes_score;

WITH directional_patterns AS (
    -- 1. Identify all PS1->CS patterns where PS1 was a consolidation state AND 
    --    the predicted outcome is directional (Bullish/Bearish)
    SELECT
        uc.asset_id,
        uc.cs_ps1_fk,
        uc.ps1_name,
        uc.cs_name,
        uc.ps1_bias_7_state AS ps1_bias_7,
        -- Infer the intended direction from the CS bias (M2 result is implicit here)
        CASE
            WHEN uc.cs_name IN ('Bullish', 'Bullish_Reversal', 'Failed_Bearish') THEN 'BULLISH'
            WHEN uc.cs_name IN ('Bearish', 'Bearish_Reversal', 'Failed_Bullish') THEN 'BEARISH'
            ELSE 'NEUTRAL' -- Filtered out later
        END AS trade_direction
        
    FROM asset_session_context uc
    WHERE uc.ps1_bias_7_state IN ('Failed_Bullish', 'Failed_Bearish', 'Pure_Indecision', 
                                  'Bullish_Consolidation', 'Bearish_Consolidation')
      AND uc.cs_name NOT IN ('Pure_Indecision', 'Other') -- Must lead to a directional CS
),
cbes_components AS (
    -- 2. Join the three required component scores (M9, M4, M5)
    SELECT
        dp.*,
        -- M9: CVI Score (P(Takedown)) - Need to filter CVI by the current PS1 bias
        COALESCE(m9.cvi_score, 0.0) AS cvi_score,
        -- M4: Follow-Through Probability (P(FT)) - Validity Factor
        -- Assuming Takedown Matrix stores P(FT) for the Breaker Session (CS)
        COALESCE(m4.follow_through_prob, 0.0) AS p_follow_through, 
        -- M5: P(Directional Day) - Validation Factor
        COALESCE(
            CASE 
                -- If predicting BULLISH, use P(Day Type = Bullish OR Bullish_Reversal OR Failed_Bearish)
                WHEN dp.trade_direction = 'BULLISH' THEN m5.p_directional_outcome_sum
                -- If predicting BEARISH, use P(Day Type = Bearish OR Bearish_Reversal OR Failed_Bullish)
                WHEN dp.trade_direction = 'BEARISH' THEN m5.p_directional_outcome_sum
                ELSE 0.0
            END, 0.0
        ) AS p_directional_day
        
    FROM directional_patterns dp
    -- M9 Join (CVI)
    LEFT JOIN asset_consolidation_volatility_index m9 
        ON dp.asset_id = m9.asset_id AND dp.cs_ps1_fk = m9.cs_ps1_fk
    -- M4 Join (Follow-Through)
    -- NOTE: FT is calculated specific to PDH or PDL break, so we simplify by taking the MAX FT for the CS session for this aggregate score.
    -- (A precise calculation would require knowing which level the CVI break targetted.)
    LEFT JOIN (
        SELECT asset_id, breaker_session, MAX(follow_through_prob) AS follow_through_prob
        FROM asset_takedown_outcome_matrix 
        WHERE outcome = 'Follow_Through'
        GROUP BY 1, 2
    ) m4 ON dp.asset_id = m4.asset_id AND dp.cs_name = m4.breaker_session
    -- M5 Join (Directional Day)
    -- NOTE: Need to pre-calculate the sum of Bullish/Bearish directional outcomes from M5 for the pattern
    LEFT JOIN (
        SELECT asset_id, cs_ps1_fk, 
            SUM(CASE WHEN day_type IN ('Bullish', 'Bullish_Reversal', 'Failed_Bearish', 'Bullish_Consolidation') THEN p_conditional ELSE 0.0 END) AS p_bullish_sum,
            SUM(CASE WHEN day_type IN ('Bearish', 'Bearish_Reversal', 'Failed_Bullish', 'Bearish_Consolidation') THEN p_conditional ELSE 0.0 END) AS p_bearish_sum
        FROM asset_day_type_1st_order
        GROUP BY 1, 2
    ) m5_agg ON dp.cs_ps1_fk = m5_agg.cs_ps1_fk
    -- Final calculation of p_directional_day in the main SELECT using the aggregated sums is simpler
    -- NOTE: I will simplify the M5 join and calculation in the final SELECT to avoid complex nested joins in the CTE
)
-- 3. Final Calculation and Insertion
INSERT INTO asset_cbes_score (
    asset_id, cs_ps1_fk, ps1_name, cs_name, trade_direction, 
    cvi_score, p_follow_through, p_directional_day, cbes_score, efficacy_label
)
SELECT
    c.asset_id,
    c.cs_ps1_fk,
    c.ps1_name,
    c.cs_name,
    c.trade_direction,
    
    c.cvi_score,
    c.p_follow_through,
    -- Recalculate P_Directional_Day using the M5 aggregation for precision
    COALESCE(
        CASE
            WHEN c.trade_direction = 'BULLISH' THEN m5.p_bullish_sum
            WHEN c.trade_direction = 'BEARISH' THEN m5.p_bearish_sum
            ELSE 0.0
        END, 0.0
    ) AS p_directional_day,
    -- CBES = CVI * P(FT) * P(Directional Day)
    (c.cvi_score / 100.0) * c.p_follow_through * (COALESCE(
        CASE
            WHEN c.trade_direction = 'BULLISH' THEN m5.p_bullish_sum
            WHEN c.trade_direction = 'BEARISH' THEN m5.p_bearish_sum
            ELSE 0.0
        END, 0.0
    )) AS cbes_score,
    -- Assign label (Thresholds are arbitrary but typically 0.60 for high probability)
    CASE
        WHEN (c.cvi_score / 100.0) * c.p_follow_through * (COALESCE(
                CASE
                    WHEN c.trade_direction = 'BULLISH' THEN m5.p_bullish_sum
                    WHEN c.trade_direction = 'BEARISH' THEN m5.p_bearish_sum
                    ELSE 0.0
                END, 0.0
            )) >= 0.60 THEN 'High Efficacy (Sustained Breakout Expected)'
        ELSE 'Low Efficacy (Breakout Failure Risk)'
    END AS efficacy_label
FROM cbes_components c
LEFT JOIN (
    SELECT cs_ps1_fk, 
        SUM(CASE WHEN day_type IN ('Bullish', 'Bullish_Reversal', 'Failed_Bearish', 'Bullish_Consolidation') THEN p_conditional ELSE 0.0 END) AS p_bullish_sum,
        SUM(CASE WHEN day_type IN ('Bearish', 'Bearish_Reversal', 'Failed_Bullish', 'Bearish_Consolidation') THEN p_conditional ELSE 0.0 END) AS p_bearish_sum
    FROM asset_day_type_1st_order
    GROUP BY 1
) m5 ON c.cs_ps1_fk = m5.cs_ps1_fk
WHERE c.trade_direction IN ('BULLISH', 'BEARISH');