

-- 1. Day Type Base Rate Table (Denominator for TCS)
CREATE TABLE IF NOT EXISTS cs_base_rates (
    asset_id TEXT NOT NULL,
    lookback_period TEXT NOT NULL,
    cs_bias TEXT NOT NULL, -- e.g., 'Bullish', 'Bearish', 'Consolidation Down', etc.
    total_count BIGINT NOT NULL,
    p_base DOUBLE PRECISION NOT NULL, -- P(CS Bias | Base)
    PRIMARY KEY (asset_id, lookback_period, cs_bias)
);

-- 2. Generalized Transition Matrices (Numerator Input)
-- (These are the tables we defined previously)
CREATE TABLE IF NOT EXISTS transition_1st_order (
    asset_id TEXT NOT NULL,
    lookback_period TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps1_bias TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_transition DOUBLE PRECISION NOT NULL, -- P(CS Bias | PS Sequence)
    PRIMARY KEY (asset_id, lookback_period, ps1_name, ps1_bias, cs_name, cs_bias)
);
CREATE TABLE IF NOT EXISTS transition_2nd_order (
    asset_id TEXT NOT NULL,
    lookback_period TEXT NOT NULL,
    ps2_name TEXT NOT NULL,
    ps2_bias TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps1_bias TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias TEXT NOT NULL,
    total_attempts BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    p_transition DOUBLE PRECISION NOT NULL, -- P(CS Bias | PS Sequence)
    PRIMARY KEY (asset_id, lookback_period, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias)
);

DROP TABLE IF EXISTS tcs_1st_order;
DROP TABLE IF EXISTS tcs_2nd_order;

-- 3. Final Transition Confidence Score (TCS) Tables
CREATE TABLE IF NOT EXISTS tcs_1st_order (
    asset_id TEXT NOT NULL,
    lookback_period TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps1_bias TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias TEXT NOT NULL,
    p_transition_conditional DOUBLE PRECISION NOT NULL, 
    p_cs_base DOUBLE PRECISION NOT NULL,              
    tcs_score DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (asset_id, lookback_period, ps1_name, ps1_bias, cs_name, cs_bias)
);

CREATE TABLE IF NOT EXISTS tcs_2nd_order (
    asset_id TEXT NOT NULL,
    lookback_period TEXT NOT NULL,
    ps2_name TEXT NOT NULL,
    ps2_bias TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps1_bias TEXT NOT NULL,
    cs_name TEXT NOT NULL,
    cs_bias TEXT NOT NULL,
    p_transition_conditional DOUBLE PRECISION NOT NULL, 
    p_cs_base DOUBLE PRECISION NOT NULL,              
    tcs_score DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (asset_id, lookback_period, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias)
);

CALL refresh_TCS_METRICS();

CREATE OR REPLACE PROCEDURE REFRESH_TCS_METRICS()
LANGUAGE plpgsql
AS $$
DECLARE
    -- Use the same lookback structure as your PCS procedure
    lookback_query TEXT := '
        SELECT ''6M'' AS lookback_period, CURRENT_DATE - INTERVAL ''6 months'' AS start_date
        UNION ALL
        SELECT ''1Y'' AS lookback_period, CURRENT_DATE - INTERVAL ''1 year'' AS start_date
        UNION ALL
        SELECT ''ALL'' AS lookback_period, ''1900-01-01''::DATE AS start_date';
BEGIN

    RAISE NOTICE 'Starting TCS Pipeline (Standard Formula)...';

    -- 0. Clean up existing data (Truncate all tables)
    TRUNCATE cs_base_rates;
    TRUNCATE transition_1st_order;
    TRUNCATE transition_2nd_order;
    TRUNCATE tcs_1st_order;
    TRUNCATE tcs_2nd_order;


    -- =================================================================================
    -- A. STEP 1: CALCULATE CS BASE RATES (P(CS Bias | Base)) - NO CHANGE
    -- =================================================================================
    RAISE NOTICE 'A. Calculating Conditional Session (CS) Base Rates...';

    EXECUTE format('
    WITH lookback_intervals AS (%s),
    -- Source the CS Bias (7-state) from session_context
    CS_Bias_Source AS (
        SELECT asset_id, trading_date, cs_bias_7 AS cs_bias
        FROM session_context
        WHERE cs_bias_7 IS NOT NULL AND cs_bias_7 != ''Other''
    ),
    dated_outcomes AS (
        SELECT s.asset_id, s.cs_bias, li.lookback_period, s.trading_date
        FROM CS_Bias_Source s
        CROSS JOIN lookback_intervals li
        WHERE s.trading_date >= li.start_date
    ),
    total_sessions_by_lookback AS (
        SELECT asset_id, lookback_period, COUNT(trading_date)::NUMERIC AS total_count
        FROM dated_outcomes GROUP BY 1, 2
    )
    INSERT INTO cs_base_rates (asset_id, lookback_period, cs_bias, total_count, p_base)
    SELECT
        d.asset_id, d.lookback_period, d.cs_bias, COUNT(*)::NUMERIC AS outcome_count,
        ROUND(((COUNT(*)::NUMERIC / NULLIF(td.total_count, 0)) * 100)::NUMERIC, 2) AS p_base
    FROM dated_outcomes d
    JOIN total_sessions_by_lookback td ON d.asset_id = td.asset_id AND d.lookback_period = td.lookback_period
    GROUP BY 1, 2, 3, td.total_count;'
    , lookback_query);


    -- =================================================================================
    -- B. STEP 2: CALCULATE TRANSITION MATRICES (P(CS Bias | PS Sequence)) - NO CHANGE
    -- =================================================================================
    RAISE NOTICE 'B. Calculating 1st and 2nd Order Transition Matrices...';

    -- B.1: 1st Order Transition (7-State) - Calculate and Insert
    EXECUTE format('
    WITH lookback_intervals AS (%s),
    FullSessionContext AS (
        SELECT 
            asset_id, trading_date, cs_name, cs_bias_7, 
            ps1_name, ps1_bias_7
        FROM session_context
    ),
    transition_1st_order_window AS (
        SELECT
            p.asset_id, p.ps1_name, p.ps1_bias_7 AS ps1_bias, 
            p.cs_name, p.cs_bias_7 AS cs_bias, 
            li.lookback_period, p.trading_date
        FROM FullSessionContext p
        CROSS JOIN lookback_intervals li
        WHERE p.trading_date >= li.start_date
        AND p.ps1_name IS NOT NULL
    ),
    transition_1st_order_D AS (
        SELECT asset_id, lookback_period, ps1_name, ps1_bias, COUNT(*)::NUMERIC AS total_attempts
        FROM transition_1st_order_window GROUP BY 1, 2, 3, 4
    )
    INSERT INTO transition_1st_order (
        asset_id, lookback_period, ps1_name, ps1_bias, cs_name, cs_bias, total_attempts, success_count, p_transition
    )
    SELECT
        t1.asset_id, t1.lookback_period, t1.ps1_name, t1.ps1_bias, t1.cs_name, t1.cs_bias,
        td.total_attempts::BIGINT, COUNT(*)::BIGINT AS success_count,
        ROUND(((COUNT(*)::NUMERIC / NULLIF(td.total_attempts, 0)) * 100)::NUMERIC, 2) AS p_transition
    FROM transition_1st_order_window t1
    JOIN transition_1st_order_D td
        ON t1.asset_id = td.asset_id AND t1.lookback_period = td.lookback_period
        AND t1.ps1_name = td.ps1_name AND t1.ps1_bias = td.ps1_bias
    GROUP BY 1, 2, 3, 4, 5, 6, td.total_attempts;
    ', lookback_query);


    -- B.2: 2nd Order Transition (7-State) - Calculate and Insert
    EXECUTE format('
    WITH lookback_intervals AS (%s),
    FullSessionContext AS (
        SELECT 
            asset_id, trading_date, 
            ps2_name, ps2_bias_7, 
            ps1_name, ps1_bias_7, 
            cs_name, cs_bias_7
        FROM session_context
    ),
    transition_2nd_order_window AS (
        SELECT
            p.asset_id, p.ps2_name, p.ps2_bias_7, p.ps1_name, p.ps1_bias_7, p.cs_name, p.cs_bias_7, 
            li.lookback_period, p.trading_date
        FROM FullSessionContext p
        CROSS JOIN lookback_intervals li
        WHERE p.trading_date >= li.start_date
        AND p.ps2_name IS NOT NULL
    ),
    transition_2nd_order_D AS (
        SELECT asset_id, lookback_period, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, COUNT(*)::NUMERIC AS total_attempts
        FROM transition_2nd_order_window GROUP BY 1, 2, 3, 4, 5, 6
    )
    INSERT INTO transition_2nd_order (
        asset_id, lookback_period, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, total_attempts, success_count, p_transition
    )
    SELECT
        t2.asset_id, t2.lookback_period, t2.ps2_name, t2.ps2_bias_7, t2.ps1_name, t2.ps1_bias_7, t2.cs_name, t2.cs_bias_7,
        td.total_attempts::BIGINT, COUNT(*)::BIGINT AS success_count,
        ROUND(((COUNT(*)::NUMERIC / NULLIF(td.total_attempts, 0)) * 100)::NUMERIC, 2) AS p_transition
    FROM transition_2nd_order_window t2
    JOIN transition_2nd_order_D td
        ON t2.asset_id = td.asset_id AND t2.lookback_period = td.lookback_period
        AND t2.ps2_name = td.ps2_name AND t2.ps2_bias_7 = td.ps2_bias_7
        AND t2.ps1_name = td.ps1_name AND t2.ps1_bias_7 = td.ps1_bias_7
    GROUP BY 1, 2, 3, 4, 5, 6, 7, 8, td.total_attempts;
    ', lookback_query);


    -- =================================================================================
    -- C. STEP 3: CALCULATE FINAL TCS SCORES (CORRECTED STANDARD FORMULA)
    -- =================================================================================
    RAISE NOTICE 'C. Calculating Final TCS Scores...';

    -- C.1: Calculate and Insert TCS 1st Order (FIXED INSERT)
    INSERT INTO tcs_1st_order (
        asset_id, lookback_period, ps1_name, ps1_bias, cs_name, cs_bias, 
        p_transition_conditional, p_cs_base, tcs_score
    )
    SELECT
        t.asset_id, t.lookback_period, t.ps1_name, t.ps1_bias, t.cs_name, t.cs_bias,
        t.p_transition,
        b.p_base,
        -- ✅ CORRECTED TCS Calculation: P(CS | PS Sequence) / P(CS | Base) * 100
        ROUND(((t.p_transition / NULLIF(b.p_base, 0)) * 100)::NUMERIC, 2) AS tcs_score
    FROM transition_1st_order t
    JOIN cs_base_rates b
        ON t.asset_id = b.asset_id
        AND t.lookback_period = b.lookback_period
        AND t.cs_bias = b.cs_bias;

    -- C.2: Calculate and Insert TCS 2nd Order (FIXED INSERT)
    INSERT INTO tcs_2nd_order (
        asset_id, lookback_period, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, 
        p_transition_conditional, p_cs_base, tcs_score
    )
    SELECT
        t.asset_id, t.lookback_period, t.ps2_name, t.ps2_bias, t.ps1_name, t.ps1_bias, t.cs_name, t.cs_bias,
        t.p_transition,
        b.p_base,
        -- ✅ CORRECTED TCS Calculation: P(CS | PS Sequence) / P(CS | Base) * 100
        ROUND(((t.p_transition / NULLIF(b.p_base, 0)) * 100)::NUMERIC, 2) AS tcs_score
    FROM transition_2nd_order t
    JOIN cs_base_rates b
        ON t.asset_id = b.asset_id
        AND t.lookback_period = b.lookback_period
        AND t.cs_bias = b.cs_bias;

    RAISE NOTICE 'TCS Pipeline complete. Scores are now scaled around 100.';

END;
$$;