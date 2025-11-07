---------------------------------
-- pcs full
--------------------------------

-- ------------------------------------------------
-- 1. SETUP: Create Tables if they don't exist
-- ------------------------------------------------

-- Metric 10: Unconditional Day Type Base Rates
DROP TABLE day_type_base_rates;
CREATE TABLE IF NOT EXISTS day_type_base_rates (
    asset_id TEXT NOT NULL, daily_outcome_7 TEXT NOT NULL, lookback_period TEXT NOT NULL, 
    outcome_count BIGINT NOT NULL, p_day_type_base DOUBLE PRECISION NOT NULL,  
    PRIMARY KEY (asset_id, daily_outcome_7, lookback_period) 
);

-- Metric 5-A: Conditional Probability (P(Success | Pattern, Tier))
DROP TABLE day_type_1st_order;
CREATE TABLE IF NOT EXISTS day_type_1st_order (
    asset_id TEXT NOT NULL, ps_name TEXT NOT NULL, ps_bias_3 TEXT NOT NULL, cs_name TEXT NOT NULL, cs_bias_3 TEXT NOT NULL, 
    daily_outcome_7 TEXT NOT NULL, day_type_tier TEXT NOT NULL, lookback_period TEXT NOT NULL, 
    total_attempts BIGINT NOT NULL, success_count BIGINT NOT NULL, p_day_type DOUBLE PRECISION NOT NULL,  
    PRIMARY KEY (asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, day_type_tier, lookback_period) 
);

-- Metric 6: Predictive Confidence Score (PCS)
DROP TABLE predictive_confidence_score;

CREATE TABLE IF NOT EXISTS predictive_confidence_score (
    asset_id TEXT NOT NULL, ps_name TEXT NOT NULL, ps_bias_3 TEXT NOT NULL, cs_name TEXT NOT NULL, cs_bias_3 TEXT NOT NULL, 
    daily_outcome_7 TEXT NOT NULL, day_type_tier TEXT NOT NULL, lookback_period TEXT NOT NULL,
    p_day_type_conditional DOUBLE PRECISION NOT NULL, p_day_type_base DOUBLE PRECISION NOT NULL, 
    pcs_score DOUBLE PRECISION NOT NULL, insight_label TEXT NOT NULL,
    PRIMARY KEY (asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, day_type_tier, lookback_period)
);

-- ------------------------------------------------
-- 2. Stored Procedure: Unified PCS Pipeline
-- ------------------------------------------------
call refresh_pcs_metrics();

CREATE OR REPLACE PROCEDURE refresh_pcs_metrics()
LANGUAGE plpgsql
AS $$
DECLARE
    lookback_query TEXT := '
        SELECT ''6M'' AS lookback_period, CURRENT_DATE - INTERVAL ''6 months'' AS start_date
        UNION ALL
        SELECT ''1Y'' AS lookback_period, CURRENT_DATE - INTERVAL ''1 year'' AS start_date 
        UNION ALL
        SELECT ''ALL'' AS lookback_period, ''1900-01-01''::DATE AS start_date';
BEGIN
    -- ------------------------------------------
    -- A. START: Metric 10 - Day Type Base Rates 
    -- ------------------------------------------
    RAISE NOTICE 'Starting Metric 10: Calculating Base Rates...';
    TRUNCATE day_type_base_rates;

    EXECUTE format('
    WITH lookback_intervals AS (%s),
    daily_outcome_7_state AS (
        SELECT trading_date, asset_id,
            CASE WHEN day_type IN (''Bullish'', ''Bearish'') THEN day_type
                 WHEN day_type = ''Consolidation'' THEN consolidation_subtype
                 ELSE ''Other'' END AS daily_outcome_7
        FROM asset_daily_views
        WHERE day_type IS NOT NULL 
    ),
    dated_outcomes AS (
        SELECT dos.asset_id, dos.daily_outcome_7, li.lookback_period, dos.trading_date
        FROM daily_outcome_7_state dos
        CROSS JOIN lookback_intervals li
        WHERE dos.trading_date >= li.start_date
    ),
    total_days_by_lookback AS (
        SELECT asset_id, lookback_period, COUNT(DISTINCT trading_date)::NUMERIC AS total_count
        FROM dated_outcomes GROUP BY 1, 2
    )
    INSERT INTO day_type_base_rates (asset_id, daily_outcome_7, lookback_period, outcome_count, p_day_type_base)
    SELECT
        d.asset_id, d.daily_outcome_7, d.lookback_period, COUNT(*)::NUMERIC AS outcome_count,
        ROUND((COUNT(*)::NUMERIC / NULLIF(td.total_count, 0)) * 100, 2) AS p_day_type_base
    FROM dated_outcomes d
    JOIN total_days_by_lookback td ON d.asset_id = td.asset_id AND d.lookback_period = td.lookback_period
    GROUP BY 1, 2, 3, td.total_count;'
    , lookback_query);
    
    -- ------------------------------------------
    -- B. MIDDLE: Metric 5-A - Conditional Probability & Tiering
    -- ------------------------------------------
    RAISE NOTICE 'Starting Metric 5-A: Calculating Conditional Rates and Tiers...';
    TRUNCATE day_type_1st_order;

    EXECUTE format('
    WITH lookback_intervals AS (%s),
    unified_bias_source AS (
        SELECT trading_date, asset_id, session_name, start_ts, session_type AS bias_3_state,
            CASE WHEN session_type IN (''Bullish'', ''Bearish'') THEN session_type
                 WHEN session_type = ''Consolidation'' THEN consolidation_subtype ELSE ''Other'' END AS bias_7_state
        FROM asset_session_views
    ),
    daily_outcome_7_state AS (
        SELECT trading_date, asset_id,
            CASE WHEN day_type IN (''Bullish'', ''Bearish'') THEN day_type
                 WHEN day_type = ''Consolidation'' THEN consolidation_subtype ELSE ''Other'' END AS daily_outcome_7
        FROM asset_daily_views
        WHERE day_type IS NOT NULL 
    ),
    sequential_pairs_3b AS (
        SELECT ps.trading_date, ps.asset_id, ps.session_name AS ps_name, ps.bias_3_state AS ps_bias_3,
               cs.session_name AS cs_name, cs.bias_3_state AS cs_bias_3
        FROM unified_bias_source ps
        JOIN unified_bias_source cs
            ON ps.asset_id = cs.asset_id AND ps.start_ts < cs.start_ts
            AND ( (ps.session_name = ''AS'' AND cs.session_name = ''LN'' AND ps.trading_date = cs.trading_date) OR
                  (ps.session_name = ''LN'' AND cs.session_name = ''NYAM'' AND ps.trading_date = cs.trading_date) OR
                  (ps.session_name = ''NYAM'' AND cs.session_name = ''NYL'' AND ps.trading_date = cs.trading_date) OR
                  (ps.session_name = ''NYL'' AND cs.session_name = ''NYPM'' AND ps.trading_date = cs.trading_date) OR
                  (ps.session_name = ''NYPM'' AND cs.session_name = ''AS'' AND ps.trading_date = ps.trading_date + INTERVAL ''1 day'') )
    ),
    TieredDatedPatterns AS (
        SELECT
            p.asset_id, p.trading_date, p.ps_name, p.ps_bias_3, p.cs_name, p.cs_bias_3, d.daily_outcome_7, 
            li.lookback_period,
            CASE
                WHEN BR.p_day_type_base >= 70.00 THEN ''Tier 1''  
                WHEN BR.p_day_type_base >= 50.00 THEN ''Tier 2''  
                ELSE ''Tier 3''                                  
            END AS day_type_tier
        FROM sequential_pairs_3b p
        JOIN daily_outcome_7_state d ON p.trading_date = d.trading_date AND p.asset_id = d.asset_id
        CROSS JOIN lookback_intervals li
        LEFT JOIN day_type_base_rates BR 
            ON p.asset_id = BR.asset_id AND d.daily_outcome_7 = BR.daily_outcome_7 
            AND li.lookback_period = BR.lookback_period
        WHERE p.trading_date >= li.start_date
    ),
    day_type_1st_order_successes AS (
        SELECT asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, day_type_tier, lookback_period,
            COUNT(*) AS success_count
        FROM TieredDatedPatterns GROUP BY 1, 2, 3, 4, 5, 6, 7, 8
    ),
    day_type_1st_order_D AS (
        SELECT asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, day_type_tier, lookback_period,
            COUNT(*) AS total_attempts
        FROM TieredDatedPatterns GROUP BY 1, 2, 3, 4, 5, 6, 7
    )
    INSERT INTO day_type_1st_order (asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, day_type_tier, lookback_period, success_count, total_attempts, p_day_type)
    SELECT
        s.asset_id, s.ps_name, s.ps_bias_3, s.cs_name, s.cs_bias_3, s.daily_outcome_7, s.day_type_tier, s.lookback_period, 
        s.success_count, d.total_attempts,
        ROUND((s.success_count::NUMERIC / NULLIF(d.total_attempts, 0)) * 100, 2) AS p_day_type
    FROM day_type_1st_order_successes s
    JOIN day_type_1st_order_D d
        ON s.asset_id = d.asset_id AND s.ps_name = d.ps_name AND s.ps_bias_3 = d.ps_bias_3 
        AND s.cs_name = d.cs_name AND s.cs_bias_3 = d.cs_bias_3
        AND s.day_type_tier = d.day_type_tier AND s.lookback_period = d.lookback_period
    WHERE d.total_attempts > 0;
    ', lookback_query);
    
    -- ------------------------------------------
    -- C. END: Metric 6 - Predictive Confidence Score (PCS)
    -- ------------------------------------------
    RAISE NOTICE 'Starting Metric 6: Calculating PCS Scores...';
    TRUNCATE predictive_confidence_score;

    INSERT INTO predictive_confidence_score (
        asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, day_type_tier, lookback_period,
        p_day_type_conditional, p_day_type_base, pcs_score, insight_label
    )
    SELECT
        t1.asset_id, t1.ps_name, t1.ps_bias_3, t1.cs_name, t1.cs_bias_3, t1.daily_outcome_7, t1.day_type_tier, t1.lookback_period,
        t1.p_day_type AS p_day_type_conditional, 
        b.p_day_type_base,                       -- Base Rate (A) from Metric 10
        -- PCS Calculation: P(Day Type | Pattern) / P(Day Type)
        ROUND(((t1.p_day_type / NULLIF(b.p_day_type_base, 0))::NUMERIC), 2) AS pcs_score,
        -- Insight Label Logic (FIXED TYPO HERE)
        CASE
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) >= 2.0 THEN 'Extreme Confidence Signal'
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) >= 1.5 THEN 'High-Confidence Signal'
            WHEN (t1.p_day_type / NULLIF(b.p_day_type_base, 0)) < 0.75 THEN 'Contradictory Signal/Failure Risk'
            ELSE 'Confirmatory/Expect Direction'
        END AS insight_label
    FROM day_type_1st_order t1 
    JOIN day_type_base_rates b 
        ON t1.asset_id = b.asset_id AND t1.daily_outcome_7 = b.daily_outcome_7
        AND t1.lookback_period = b.lookback_period;
        
    RAISE NOTICE 'PCS Pipeline complete.';

END;
$$;