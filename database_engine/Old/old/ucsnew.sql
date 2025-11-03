-- UCS SQL: Calculates the final confidence score for any PS1 -> CS transition using the provided tables.

WITH target_pcs_signal AS (
    -- 1. Get the base PCS signal for today's predictions
    SELECT
        t.asset_id,
        t.trading_date,
        t.ps2_name,                   -- Session two steps back (e.g., NYPM if CS is AS)
        t.ps1_name,                   -- Session one step back (The immediate Preceding Session)
        t.cs_name,                    -- Current Session (The target session)
        t.trade_direction,            -- 'BULLISH' or 'BEARISH'
        t.pcs_base_score
    FROM us2000_predictive_confidence_score t
    WHERE t.trading_date = CURRENT_DATE
),
current_session_biases AS (
    -- 2. Fetch the current 7-state and 3-state biases for all sessions today
    SELECT
        trading_date,
        session_name,
        bias_7_state,
        bias_3_state
    FROM us2000_session_bias
    WHERE trading_date = CURRENT_DATE
),
current_trade_context AS (
    -- 3. Combine the target PCS signal with the 'live' 7-state and 3-state biases for PS1 and PS2
    SELECT
        t.*,
        ps1_bias.bias_7_state AS ps1_7_state,
        ps1_bias.bias_3_state AS ps1_3_state,
        ps2_bias.bias_7_state AS ps2_7_state
    FROM target_pcs_signal t
    -- Join 1: Get PS1's current biases
    JOIN current_session_biases ps1_bias
        ON t.ps1_name = ps1_bias.session_name
        AND t.trading_date = ps1_bias.trading_date
    -- Join 2: Get PS2's current 7-state bias
    JOIN current_session_biases ps2_bias
        ON t.ps2_name = ps2_bias.session_name
        AND t.trading_date = ps2_bias.trading_date
),
-- CTE 1: Contextual TCS (Trend Continuation Score)
-- Joins on the full historical sequence: Direction + PS2 Bias + PS1 Bias
tcs_contextual_score AS (
    SELECT
        s.asset_id,
        s.trading_date,
        t.tcs_score
    FROM current_trade_context s
    LEFT JOIN us2000_trend_continuation_score t
        ON t.asset_id = s.asset_id
        AND t.trend_direction = s.trade_direction
        AND t.ps2_bias_7 = s.ps2_7_state    -- PS2's actual 7-state today
        AND t.ps1_bias_7 = s.ps1_7_state    -- PS1's actual 7-state today
),
-- CTE 2: Contextual CVI (Consolidation Volatility Index)
-- Conditional Join: Only relevant if PS1's 3-state bias is 'Consolidation'
cvi_contextual_score AS (
    SELECT
        s.asset_id,
        s.trading_date,
        -- Conditional: Only use the CVI if PS1 was a consolidation (3-state check)
        CASE
            WHEN s.ps1_3_state = 'Consolidation' THEN c.cvi_score
            ELSE NULL
        END AS cvi_score
    FROM current_trade_context s
    LEFT JOIN us2000_consolidation_volatility_index c
        ON c.asset_id = s.asset_id
        AND c.ps_name = s.ps1_name  -- Match PS1's name
        AND c.cs_name = s.cs_name   -- Match CS's name
        AND c.ps_bias_7 = s.ps1_7_state -- Must match the specific PS1 7-state subtype today
),
-- CTE 3: FHR (Failing Historical Reversal) - The PS1 Trap Filter (VETO)
-- Checks if PS1 triggered a takedown event and if that event/bias has high reversal risk.
fhr_trap_filter AS (
    SELECT
        s.asset_id,
        s.trading_date,
        f.fhr_score AS fhr_veto_score
    FROM current_trade_context s
    -- Check for a Takedown Event in PS1 today
    LEFT JOIN us2000_takedown_events t
        ON s.trading_date = t.trading_date
        AND t.breaker_session = s.ps1_name -- The takedown must have happened in the PS1 session
    -- Join to the FHR table for the score, but ONLY if a takedown occurred
    LEFT JOIN us2000_failing_historical_reversal f
        ON f.asset_id = s.asset_id
        AND f.cs_name = s.ps1_name     -- FHR 'CS' is the breaker session 'PS1'
        AND f.cs_bias_7 = s.ps1_7_state -- FHR 'CS_BIAS_7' is the PS1 7-state bias today
        -- Match the takedown event type (PDH or PDL break)
        AND f.pattern_type = CASE WHEN t.prior_level = 'High' THEN 'PDH_Break_to_Bearish' ELSE 'PDL_Break_to_Bullish' END
    WHERE t.breaker_session IS NOT NULL -- Exclude records where no takedown event occurred
)
-- CTE 4: Final UCS Calculation and Aggregation
    SELECT
        s.trading_date,
        s.asset_id,
        s.ps1_name AS preceding_session,
        s.cs_name AS target_session,
        s.trade_direction,
        s.pcs_base_score,
        -- COALESCE ensures a neutral score of 60.0 (1.0 multiplier) if the contextual join fails or is irrelevant.
        COALESCE(t.tcs_score, 60.0) AS tcs_score,
        COALESCE(c.cvi_score, 60.0) AS cvi_score,
        -- FHR defaults to 0.0 if no trap was sprung today
        COALESCE(f.fhr_veto_score, 0.0) AS fhr_veto_score,
        -- CALCULATE FINAL UCS
        -- Multipliers are derived from the score: (Score / 60.0) where 60.0 is the neutral point (1.0 multiplier)
        s.pcs_base_score
        * (COALESCE(t.tcs_score, 60.0) / 60.0) -- TCS: Stability Multiplier
        * (COALESCE(c.cvi_score, 60.0) / 60.0) -- CVI: Volatility Multiplier
        -- FHR VETO LOGIC: If the FHR score is high (>= 60.0, high reversal risk), apply a heavy penalty (multiplier of 0.2).
        * (CASE
            WHEN COALESCE(f.fhr_veto_score, 0.0) >= 60.0 THEN 0.20 -- High Veto Risk (Trap Sprung)
            ELSE 1.0
        END) AS unified_confidence_score
FROM current_trade_context s
LEFT JOIN tcs_contextual_score t USING (asset_id, trading_date)
LEFT JOIN cvi_contextual_score c USING (asset_id, trading_date)
LEFT JOIN fhr_trap_filter f USING (asset_id, trading_date);
