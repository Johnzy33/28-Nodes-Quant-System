-----------------------
---- Load live vie of the PCS System
-----------------------------------

-- View Name: latest_pcs_scores_v
-- Purpose: Retrieves the three PCS scores (6M, 1Y, ALL) for the SINGLE most recent,
--          fully calculated pattern/outcome key in the system.

CREATE OR REPLACE VIEW latest_pcs_scores_v AS
WITH latest_context AS (
    -- 1. Identify the unique key of the most recent completed pattern/outcome.
    --    We use DISTINCT ON (asset_id) to get the latest trading_date's full key.
    SELECT DISTINCT ON (asset_id)
        asset_id,
        trading_date,
        ps1_name AS ps_name,
        ps1_bias_3_state AS ps_bias_3,
        cs_name,
        cs_bias_3_state AS cs_bias_3,
        daily_outcome_7,
        -- We must join to the base rate table to dynamically get the TIER for the latest date.
        -- Assumes the TIER is derived from the 'ALL' lookback base rate for consistency.
        CASE
            WHEN br.p_day_type_base >= 70.00 THEN 'Tier 1'
            WHEN br.p_day_type_base >= 50.00 THEN 'Tier 2'
            ELSE 'Tier 3'
        END AS day_type_tier
    FROM 
        new_session_context c -- Your Live Snapshot Table
    LEFT JOIN day_type_base_rates br
        ON c.asset_id = br.asset_id
        AND c.daily_outcome_7 = br.daily_outcome_7
        AND br.lookback_period = 'ALL' -- Use the robust 'ALL' history base rate to define the tier
    ORDER BY 
        asset_id, trading_date DESC
)
-- 2. Join that unique latest key against the full PCS history table
SELECT
    p.asset_id,
    c.trading_date AS prediction_date,
    p.ps_name,
    p.ps_bias_3,
    p.cs_name,
    p.cs_bias_3,
    p.daily_outcome_7,
    p.day_type_tier,
    -- Scores and Labels
    p.lookback_period,
    p.pcs_score,
    p.insight_label,
    p.p_day_type_conditional,
    p.p_day_type_base,
    -- Mark the 1Y score as the Primary Reference Score for easy querying
    CASE WHEN p.lookback_period = '1Y' THEN TRUE ELSE FALSE END AS is_reference_score
FROM
    predictive_confidence_score p
JOIN
    latest_context c
    ON p.asset_id = c.asset_id
    AND p.ps_name = c.ps_name
    AND p.ps_bias_3 = c.ps_bias_3
    AND p.cs_name = c.cs_name
    AND p.cs_bias_3 = c.cs_bias_3
    AND p.daily_outcome_7 = c.daily_outcome_7
    AND p.day_type_tier = c.day_type_tier
ORDER BY
    -- Order by date and then by priority (1Y first, then 6M, then ALL)
    c.trading_date DESC,
    CASE p.lookback_period
        WHEN '1Y' THEN 1
        WHEN '6M' THEN 2
        WHEN 'ALL' THEN 3
        ELSE 4
    END;