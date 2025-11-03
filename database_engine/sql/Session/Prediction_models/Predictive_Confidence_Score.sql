----------------------------------
-- The Base Table for PCS
---------------------------------

DROP TABLE IF EXISTS asset_day_type_base_rates CASCADE;
-- Metric 10: Unconditional Day Type Base Rates
CREATE TABLE IF NOT EXISTS asset_day_type_base_rates (
    asset_id TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    outcome_count BIGINT NOT NULL,
    p_day_type_base DOUBLE PRECISION NOT NULL,  -- The unconditional probability P(Day Type)
    PRIMARY KEY (asset_id, daily_outcome_7)
);
TRUNCATE asset_day_type_base_rates;

WITH daily_outcome_7_state AS (
    -- Defines the 7-State Outcome
    SELECT
        trading_date,
        asset_id,
        CASE
            WHEN day_type IN ('Bullish', 'Bearish') THEN day_type
            WHEN day_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other'
        END AS daily_outcome_7
    FROM asset_daily_views -- <<< GENERALIZED VIEW
    WHERE day_type IS NOT NULL
),
total_days_per_asset AS (
    -- Calculate the total number of valid days for the denominator (D)
    SELECT 
        asset_id, 
        COUNT(*) AS total_count 
    FROM asset_daily_views 
    WHERE day_type IS NOT NULL
    GROUP BY 1
),
base_rate_calculation AS (
    SELECT
        d7.asset_id,
        d7.daily_outcome_7,
        COUNT(*)::NUMERIC AS outcome_count,
        -- Calculate P(Day Type Base) = N / D (Per Asset)
        ROUND((COUNT(*)::NUMERIC / t.total_count) * 100, 2) AS p_day_type_base
    FROM daily_outcome_7_state d7
    JOIN total_days_per_asset t
        ON d7.asset_id = t.asset_id
    GROUP BY 1, 2, t.total_count
)
-- INSERT INTO Base Rate Table
INSERT INTO asset_day_type_base_rates (
    asset_id, daily_outcome_7, outcome_count, p_day_type_base
)
SELECT * FROM base_rate_calculation;

---------------------------------------------
-- The Predictive Confidence of a Day outcome
---------------------------------------------

DROP TABLE IF EXISTS asset_predictive_confidence_score CASCADE;
-- Metric 6: Predictive Confidence Score (PCS) Table
CREATE TABLE IF NOT EXISTS asset_predictive_confidence_score (
    asset_id TEXT NOT NULL,
    ps_name TEXT NOT NULL,
    ps_bias_3 TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias_3 TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    p_day_type_conditional DOUBLE PRECISION NOT NULL, -- P(Day Type | Pattern)
    p_day_type_base DOUBLE PRECISION NOT NULL,       -- P(Day Type)
    pcs_score DOUBLE PRECISION NOT NULL,             -- PCS = Conditional / Base
    insight_label TEXT NOT NULL,
    PRIMARY KEY (asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7)
);
TRUNCATE asset_predictive_confidence_score;

WITH pcs_calculation AS (
    SELECT
        t1.asset_id,
        t1.ps_name,
        t1.ps_bias_3,
        t1.cs_name,
        t1.cs_bias_3,
        t1.daily_outcome_7,
        t1.p_day_type AS p_day_type_conditional, -- Conditional Probability (C) from Metric 5-A
        b.p_day_type_base,                      -- Base Rate (A) from Metric 10
        -- PCS = Conditional / Base (Bayes Factor / Lift)
        ROUND(((t1.p_day_type / NULLIF(b.p_day_type_base, 0))), 2) AS pcs_score,
        CASE
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) >= 2.0 THEN 'Extreme Confidence Signal'
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) >= 1.5 THEN 'High-Confidence Signal'
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) < 0.75 THEN 'Contradictory Signal/Failure Risk'
            ELSE 'Confirmatory/Expect Direction'
        END AS insight_label
    FROM asset_day_type_1st_order t1 -- <<< GENERALIZED Metric 5-A results
    JOIN asset_day_type_base_rates b -- <<< GENERALIZED Metric 10 results
        ON t1.asset_id = b.asset_id 
        AND t1.daily_outcome_7 = b.daily_outcome_7
)
-- INSERT INTO Table 6
INSERT INTO asset_predictive_confidence_score (
    asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7,
    p_day_type_conditional, p_day_type_base, pcs_score, insight_label
)
SELECT * FROM pcs_calculation;