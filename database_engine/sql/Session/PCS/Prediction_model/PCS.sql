-----------------
-- The model
-------------------

DROP TABLE IF EXISTS predictive_confidence_score CASCADE;

-- Metric 6: Predictive Confidence Score (PCS) Table - Production Ready Name
CREATE TABLE IF NOT EXISTS predictive_confidence_score (
    asset_id TEXT NOT NULL,
    ps_name TEXT NOT NULL,
    ps_bias_3 TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias_3 TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    -- NEW: The three-tiered day type classification
    day_type_tier TEXT NOT NULL,
    -- NEW: Column to identify the lookback period: '6M', '1Y', or 'ALL'
    lookback_period TEXT NOT NULL,
    p_day_type_conditional DOUBLE PRECISION NOT NULL, -- Conditional (Metric 5-A)
    p_day_type_base DOUBLE PRECISION NOT NULL,       -- Base Rate (Metric 10)
    pcs_score DOUBLE PRECISION NOT NULL,             -- PCS = Conditional / Base
    insight_label TEXT NOT NULL,                     -- High-Confidence, Contradictory, etc. 
    -- PRIMARY KEY must now include day_type_tier and lookback_period
    PRIMARY KEY (asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, day_type_tier, lookback_period)
);
TRUNCATE predictive_confidence_score;

-- CTE to calculate PCS for all Lookback Periods and Tiers
WITH pcs_calculation AS (
    SELECT
        t1.asset_id,
        t1.ps_name,
        t1.ps_bias_3,
        t1.cs_name,
        t1.cs_bias_3,
        t1.daily_outcome_7,
        t1.day_type_tier,      -- NEW: Tier from Metric 5-A
        t1.lookback_period,    -- NEW: Lookback Period from Metric 5-A
        t1.p_day_type AS p_day_type_conditional, -- Conditional Probability (C) from Metric 5-A
        b.p_day_type_base,                       -- Base Rate (A) from Metric 10
        -- PCS Calculation: P(Day Type | Pattern) / P(Day Type)
        ROUND(((t1.p_day_type / NULLIF(b.p_day_type_base, 0))::NUMERIC), 2) AS pcs_score,
        -- Insight Label Logic
        CASE
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) >= 2.0 THEN 'Extreme Confidence Signal'
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) >= 1.5 THEN 'High-Confidence Signal'
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) < 0.75 THEN 'Contradictory Signal/Failure Risk'
            ELSE 'Confirmatory/Expect Direction'
        END AS insight_label
    -- Join Conditional Rate (t1) to Base Rate (b)
    FROM day_type_1st_order t1 -- Production-Ready Metric 5-A results
    JOIN day_type_base_rates b -- Production-Ready Metric 10 results
        -- The JOIN must now include the lookback_period
        ON t1.asset_id = b.asset_id 
        AND t1.daily_outcome_7 = b.daily_outcome_7
        AND t1.lookback_period = b.lookback_period -- KEY JOIN on Lookback Period!
)
-- INSERT INTO Table 6 (PCS)
INSERT INTO predictive_confidence_score (
    asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, day_type_tier, lookback_period,
    p_day_type_conditional, p_day_type_base, pcs_score, insight_label
)
SELECT * FROM pcs_calculation;

---------------------b------------
-- live
--------------------------------

WITH latest_context AS (
    -- 1. Get the most recent trading date and the current pattern/tier.
    -- This uses the logic from your live session capture, assuming the latest
    -- row in your session context table holds the data for the current prediction day.
    SELECT
        asset_id,
        trading_date,
        ps1_name AS ps_name,
        ps1_bias_3_state AS ps_bias_3,
        cs_name,
        cs_bias_3_state AS cs_bias_3,
        daily_outcome_7,
        -- Assuming the current day's tier is calculated and available, 
        -- or derived using the 'ALL' lookback base rate, as previously discussed.
        day_type_tier
    FROM asset_session_context -- Your Live Snapshot Table
    JOIN predictive_confidence_score uc
    ORDER BY
        trading_date DESC
    LIMIT 1
)
-- 2. Join the current context to the final PCS table to retrieve all three lookback scores.
SELECT
    p.asset_id,
    p.trading_date,
    p.ps_name,
    p.cs_name,
    p.daily_outcome_7,
    p.day_type_tier,
    -- Select the three key result columns
    p.lookback_period,
    p.confidence_score,
    p.insight_label
FROM
    predictive_confidence_score p
JOIN
    latest_context c
    ON p.asset_id = c.asset_id
    -- Match the identified pattern and its characteristics (PS, CS, Outcomes)
    AND p.ps_name = c.ps_name
    AND p.ps_bias_3 = c.ps_bias_3
    AND p.cs_name = c.cs_name
    AND p.cs_bias_3 = c.cs_bias_3
    AND p.daily_outcome_7 = c.daily_outcome_7
    -- Match the dynamically determined current day's tier
    AND p.day_type_tier = c.day_type_tier
ORDER BY
    -- Order by lookback period so 6M/1Y/ALL are clearly presented
    CASE p.lookback_period
        WHEN '6M' THEN 1
        WHEN '1Y' THEN 2
        WHEN 'ALL' THEN 3
        ELSE 4
    END;


-----------------

DROP TABLE IF EXISTS predictive_confidence_score CASCADE;
CREATE TABLE IF NOT EXISTS predictive_confidence_score (
    ps_cs_pattern_fk BIGINT NOT NULL,
    asset_id TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    day_type_tier TEXT NOT NULL,
    lookback_period TEXT NOT NULL,
    p_day_type_conditional DOUBLE PRECISION NOT NULL,
    p_day_type_base DOUBLE PRECISION NOT NULL,
    pcs_score DOUBLE PRECISION NOT NULL,
    insight_label TEXT NOT NULL,
    PRIMARY KEY (ps_cs_pattern_fk, daily_outcome_7, day_type_tier, lookback_period)
);
TRUNCATE predictive_confidence_score;
WITH pcs_calculation AS (
    SELECT
        t1.ps_cs_pattern_fk, t1.asset_id, t1.daily_outcome_7, t1.day_type_tier, t1.lookback_period,
        t1.p_day_type AS p_day_type_conditional, 
        b.p_day_type_base,                       
        ROUND(((t1.p_day_type / NULLIF(b.p_day_type_base, 0))::NUMERIC), 2) AS pcs_score,
        CASE
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) >= 2.0 THEN 'Extreme Confidence Signal'
            WHEN (t1.p_day_type / NULLIF(b.p_type_base, 0)) >= 1.5 THEN 'High-Confidence Signal'
            WHEN (t1.p_type / NULLIF(b.p_type_base, 0)) < 0.75 THEN 'Contradictory Signal/Failure Risk'
            ELSE 'Confirmatory/Expect Direction'
        END AS insight_label
    FROM day_type_1st_order t1 
    JOIN day_type_base_rates b
        ON t1.asset_id = b.asset_id 
        AND t1.daily_outcome_7 = b.daily_outcome_7
        AND t1.lookback_period = b.lookback_period 
)
INSERT INTO predictive_confidence_score (
    ps_cs_pattern_fk, asset_id, daily_outcome_7, day_type_tier, lookback_period,
    p_day_type_conditional, p_day_type_base, pcs_score, insight_label
)
SELECT * FROM pcs_calculation
ON CONFLICT (ps_cs_pattern_fk, daily_outcome_7, day_type_tier, lookback_period)
DO UPDATE SET
    pcs_score = EXCLUDED.pcs_score,
    p_day_type_conditional = EXCLUDED.p_day_type_conditional,
    p_day_type_base = EXCLUDED.p_day_type_base,
    insight_label = EXCLUDED.insight_label;