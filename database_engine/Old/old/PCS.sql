DROP TABLE IF EXISTS us2000_day_type_base_rates CASCADE;
-- Metric 10: Unconditional Day Type Base Rates (Used as P(Day Type Base Rate) in PCS)
CREATE TABLE IF NOT EXISTS us2000_day_type_base_rates (
    asset_id TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    outcome_count BIGINT NOT NULL,
    p_day_type_base DOUBLE PRECISION NOT NULL,  -- The unconditional probability
    PRIMARY KEY (asset_id, daily_outcome_7)
);
TRUNCATE us2000_day_type_base_rates;

-- CTE to calculate and insert the base rates
WITH daily_outcome_7_state AS (
    -- Defines the 7-State Outcome (Bullish/Bearish or one of the 5 consolidation subtypes)
    SELECT
        trading_date,
        asset_id,
        CASE
            WHEN day_type IN ('Bullish', 'Bearish') THEN day_type
            WHEN day_type = 'Consolidation' THEN consolidation_subtype
            ELSE 'Other'
        END AS daily_outcome_7
    FROM asset_daily_views
    WHERE day_type IS NOT NULL AND asset_id = 'assets:US2000'
),
total_days AS (
    SELECT COUNT(*) AS total_count FROM asset_daily_views WHERE asset_id = 'assets:US2000'
),
base_rate_calculation AS (
    SELECT
        asset_id,
        daily_outcome_7,
        COUNT(*)::NUMERIC AS outcome_count,
        ROUND((COUNT(*)::NUMERIC / (SELECT total_count FROM total_days)) * 100, 2) AS p_day_type_base
    FROM daily_outcome_7_state
    GROUP BY 1, 2
)
-- INSERT INTO Base Rate Table
INSERT INTO us2000_day_type_base_rates (
    asset_id, daily_outcome_7, outcome_count, p_day_type_base
)
SELECT * FROM base_rate_calculation;

---------------------
--- PCS
---------------------
DROP TABLE IF EXISTS us2000_predictive_confidence_score CASCADE;
-- Metric 6: Predictive Confidence Score (PCS) Table
CREATE TABLE IF NOT EXISTS us2000_predictive_confidence_score (
    asset_id TEXT NOT NULL,
    ps_name TEXT NOT NULL,
    ps_bias_3 TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias_3 TEXT NOT NULL,
    daily_outcome_7 TEXT NOT NULL,
    p_day_type_conditional DOUBLE PRECISION NOT NULL, -- P(Day Type | Pattern) - Metric 5 result
    p_day_type_base DOUBLE PRECISION NOT NULL,       -- P(Day Type) - Base Rate (from new table)
    pcs_score DOUBLE PRECISION NOT NULL,             -- PCS = Conditional / Base
    insight_label TEXT NOT NULL,                     -- High-Confidence, Contradictory, etc.
    PRIMARY KEY (asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7)
);
TRUNCATE us2000_predictive_confidence_score;
-- CTE to calculate PCS
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
        -- FIX: Cast the division result to NUMERIC before rounding
        ROUND(((t1.p_day_type / NULLIF(b.p_day_type_base, 0))::NUMERIC), 2) AS pcs_score,
        CASE
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) >= 2.0 THEN 'Extreme Confidence Signal' -- Raised threshold
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) >= 1.5 THEN 'High-Confidence Signal'
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) < 0.75 THEN 'Contradictory Signal/Failure Risk'
            ELSE 'Confirmatory/Expect Direction'
        END AS insight_label
    FROM us2000_day_type_1st_order t1 -- Results from Metric 5-A
    JOIN us2000_day_type_base_rates b
        ON t1.asset_id = b.asset_id AND t1.daily_outcome_7 = b.daily_outcome_7
)
-- INSERT INTO Table 6
INSERT INTO us2000_predictive_confidence_score (
    asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7,
    p_day_type_conditional, p_day_type_base, pcs_score, insight_label
)
SELECT * FROM pcs_calculation;