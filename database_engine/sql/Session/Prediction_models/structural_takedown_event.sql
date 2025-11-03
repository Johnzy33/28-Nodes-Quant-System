-- Structural Break Event Logger

-- Generalized Structural Takedowns (Context for UCS metrics)
DROP TABLE IF EXISTS asset_structural_takedowns CASCADE;
CREATE TABLE IF NOT EXISTS asset_structural_takedowns (
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    session_name TEXT NOT NULL,
    bias_7_state TEXT NOT NULL,
    is_pdh_break BOOLEAN NOT NULL,
    is_pdl_break BOOLEAN NOT NULL,
    PRIMARY KEY (asset_id, trading_date, session_name)
);
TRUNCATE asset_structural_takedowns;

-- Generalized Takedown Events (Detailed log of all breaks)
DROP TABLE IF EXISTS asset_takedown_events CASCADE;
CREATE TABLE IF NOT EXISTS asset_takedown_events (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    cs_session TEXT NOT NULL,
    ps1_session TEXT NOT NULL,
    taken_level TEXT NOT NULL,
    taken_price DOUBLE PRECISION NOT NULL,
    cs_price DOUBLE PRECISION NOT NULL,
    cumulative_check TEXT, -- Stores 'N-Day-High/Low' or 'Untaken-High/Low'
    PRIMARY KEY (trading_date, asset_id, cs_session, ps1_session, taken_level)
);
TRUNCATE asset_takedown_events;

-- NEW: Table for Session Highs/Lows that survived the rest of their trading day
DROP TABLE IF EXISTS asset_survived_session_levels CASCADE;
CREATE TABLE IF NOT EXISTS asset_survived_session_levels (
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    session_name TEXT NOT NULL,
    session_high DOUBLE PRECISION NOT NULL,
    session_low DOUBLE PRECISION NOT NULL,
    high_survived BOOLEAN NOT NULL,
    low_survived BOOLEAN NOT NULL,
    PRIMARY KEY (asset_id, trading_date, session_name)
);
TRUNCATE asset_survived_session_levels;

-- =================================================================================
-- PART A: FOUNDATION AND CUMULATIVE CHECKS
-- =================================================================================
WITH daily_comparison AS (
    SELECT
        trading_date, asset_id,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdh,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdl,
        LAG(high, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd2h,
        LAG(low, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd2l,
        LAG(high, 3) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd3h,
        LAG(low, 3) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd3l
    FROM asset_daily_views
),
cumulative_checks AS (
    SELECT
        trading_date, asset_id, pdh, pdl,
        CASE WHEN (pdh > pd2h AND pd2h > pd3h) THEN '3-Day-High' WHEN (pdh > pd2h) THEN '2-Day-High' ELSE NULL END AS cumulative_high_check,
        CASE WHEN (pdl < pd2l AND pd2l < pd3l) THEN '3-Day-Low' WHEN (pdl < pd2l) THEN '2-Day-Low' ELSE NULL END AS cumulative_low_check
    FROM daily_comparison
),
session_sequence AS (
    -- Order Sessions Chronologically and pull 7-Bias
    SELECT
        trading_date, asset_id, session_name, high, low, start_ts,
        CASE WHEN session_type IN ('Bullish', 'Bearish') THEN session_type WHEN session_type = 'Consolidation' THEN consolidation_subtype ELSE 'Other' END AS bias_7_state,
        ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS session_rank
    FROM asset_session_views
),
-- =================================================================================
-- PART B: INTRA-DAY SEQUENTIAL TAKEDOWNS (PS1 -> CS on same day)
-- =================================================================================
intra_day_comparison AS (
    SELECT
        cs.trading_date, cs.asset_id, cs.session_name AS cs_session, cs.high AS cs_high, cs.low AS cs_low,
        LAG(cs.session_name, 1) OVER w AS ps1_session,
        LAG(cs.high, 1) OVER w AS ps1_high,
        LAG(cs.low, 1) OVER w AS ps1_low
    FROM session_sequence cs
    WINDOW w AS (PARTITION BY cs.trading_date, cs.asset_id ORDER BY cs.session_rank)
    WHERE cs.session_rank > 1
),
intra_day_events AS (
    -- High Takedown event
    SELECT trading_date, asset_id, cs_session, ps1_session, 'High' AS taken_level, ps1_high AS taken_price, cs_high AS cs_price, NULL::TEXT AS cumulative_check
    FROM intra_day_comparison WHERE ps1_session IS NOT NULL AND cs_high > ps1_high
    UNION ALL 
    -- Low Takedown event
    SELECT trading_date, asset_id, cs_session, ps1_session, 'Low' AS taken_level, ps1_low AS taken_price, cs_low AS cs_price, NULL::TEXT AS cumulative_check
    FROM intra_day_comparison WHERE ps1_session IS NOT NULL AND cs_low < ps1_low
),
-- =================================================================================
-- PART C: PREVIOUS DAY LEVEL TAKEDOWNS (PDH/PDL)
-- =================================================================================
daily_level_takedowns AS (
    SELECT
        s.trading_date, s.asset_id, s.session_name AS cs_session, s.start_ts, s.bias_7_state,
        'PrevDay-Daily' AS ps1_session,
        CASE WHEN s.high > cc.pdh THEN 'High' ELSE NULL END AS high_level,
        CASE WHEN s.high > cc.pdh THEN cc.pdh ELSE NULL END AS high_taken_price,
        CASE WHEN s.high > cc.pdh THEN s.high ELSE NULL END AS high_cs_price,
        cc.cumulative_high_check AS high_cumulative_check,
        CASE WHEN s.low < cc.pdl THEN 'Low' ELSE NULL END AS low_level,
        CASE WHEN s.low < cc.pdl THEN cc.pdl ELSE NULL END AS low_taken_price,
        CASE WHEN s.low < cc.pdl THEN s.low ELSE NULL END AS low_cs_price,
        cc.cumulative_low_check AS low_cumulative_check,
        ROW_NUMBER() OVER (PARTITION BY s.trading_date, s.asset_id, CASE WHEN s.high > cc.pdh THEN 'High' WHEN s.low < cc.pdl THEN 'Low' ELSE NULL END ORDER BY s.start_ts ASC) AS rank_num
    FROM session_sequence s
    JOIN cumulative_checks cc ON cc.trading_date = s.trading_date AND cc.asset_id = s.asset_id
    WHERE (s.high > cc.pdh AND cc.pdh IS NOT NULL) OR (s.low < cc.pdl AND cc.pdl IS NOT NULL)
),
daily_level_events AS (
    -- PDH Takedowns (High)
    SELECT trading_date, asset_id, cs_session, ps1_session, high_level AS taken_level, high_taken_price AS taken_price, high_cs_price AS cs_price, high_cumulative_check AS cumulative_check, rank_num
    FROM daily_level_takedowns WHERE high_level IS NOT NULL
    UNION ALL
    -- PDL Takedowns (Low)
    SELECT trading_date, asset_id, cs_session, ps1_session, low_level AS taken_level, low_taken_price AS taken_price, low_cs_price AS cs_price, low_cumulative_check AS cumulative_check, rank_num
    FROM daily_level_takedowns WHERE low_level IS NOT NULL
),
-- =================================================================================
-- PART D: UNTAKEN SESSION LEVEL TAKEDOWNS (Cross-Day)
-- =================================================================================
session_max_min AS (
    -- Calculates max/min of subsequent sessions on the SAME day for survival check
    SELECT
        s1.trading_date, s1.asset_id, s1.session_name, s1.high AS session_high, s1.low AS session_low,
        MAX(s2.high) AS max_subsequent_high,
        MIN(s2.low) AS min_subsequent_low
    FROM session_sequence s1
    LEFT JOIN session_sequence s2 ON s1.asset_id = s2.asset_id AND s1.trading_date = s2.trading_date AND s2.session_rank > s1.session_rank
    GROUP BY 1, 2, 3, 4, 5
),
survived_levels AS (
    -- Logs the levels that survived the rest of their trading day
    SELECT
        asset_id, trading_date, session_name, session_high, session_low,
        (max_subsequent_high IS NULL OR max_subsequent_high <= session_high) AS high_survived,
        (min_subsequent_low IS NULL OR min_subsequent_low >= session_low) AS low_survived
    FROM session_max_min
),
cross_day_takedown_events AS (
    -- Joins current sessions against the survivors from prior days
    SELECT
        cs.trading_date, cs.asset_id, cs.session_name AS cs_session, ss.session_name AS ps1_session,
        CASE WHEN cs.high > ss.session_high THEN 'High' ELSE NULL END AS high_taken_level,
        CASE WHEN cs.high > ss.session_high THEN ss.session_high ELSE NULL END AS high_taken_price,
        CASE WHEN cs.high > ss.session_high THEN cs.high ELSE NULL END AS high_cs_price,
        CASE WHEN cs.low < ss.session_low THEN 'Low' ELSE NULL END AS low_taken_level,
        CASE WHEN cs.low < ss.session_low THEN ss.session_low ELSE NULL END AS low_taken_price,
        CASE WHEN cs.low < ss.session_low THEN cs.low ELSE NULL END AS low_cs_price
    FROM asset_session_views cs
    JOIN survived_levels ss ON cs.asset_id = ss.asset_id
    WHERE cs.trading_date > ss.trading_date
      AND (
          (ss.high_survived = TRUE AND cs.high > ss.session_high) OR
          (ss.low_survived = TRUE AND cs.low < ss.session_low)
      )
),
cross_day_events AS (
    -- Unpivot and tag Cross-Day Takedowns
    SELECT trading_date, asset_id, cs_session, ps1_session, high_taken_level AS taken_level, high_taken_price AS taken_price, high_cs_price AS cs_price, 'Untaken-High' AS cumulative_check
    FROM cross_day_takedown_events WHERE high_taken_level IS NOT NULL
    UNION ALL
    SELECT trading_date, asset_id, cs_session, ps1_session, low_taken_level AS taken_level, low_taken_price AS taken_price, low_cs_price AS cs_price, 'Untaken-Low' AS cumulative_check
    FROM cross_day_takedown_events WHERE low_taken_level IS NOT NULL
)
-- =================================================================================
-- PART E: FINAL INSERTS
-- =================================================================================
-- 1. INSERT INTO asset_survived_session_levels
INSERT INTO asset_survived_session_levels (
    asset_id, trading_date, session_name, session_high, session_low, high_survived, low_survived
)
SELECT * FROM survived_levels
-- 2. INSERT INTO asset_structural_takedowns (First PDH/PDL Break Only)
INSERT INTO asset_structural_takedowns (
    asset_id, trading_date, session_name, bias_7_state, is_pdh_break, is_pdl_break
)
SELECT DISTINCT ON (trading_date, asset_id, cs_session)
    s.asset_id, s.trading_date, s.cs_session, s.bias_7_state,
    MAX(CASE WHEN s.taken_level = 'High' AND s.rank_num = 1 THEN TRUE ELSE FALSE END) AS is_pdh_break,
    MAX(CASE WHEN s.taken_level = 'Low' AND s.rank_num = 1 THEN TRUE ELSE FALSE END) AS is_pdl_break
FROM daily_level_takedowns s
GROUP BY 1, 2, 3, 4
INSERT INTO asset_takedown_events (
    trading_date, asset_id, cs_session, ps1_session, taken_level, 
    taken_price, cs_price, cumulative_check
)
-- Type 1: Intra-Day Sequential Events
SELECT * FROM intra_day_events
UNION ALL
-- Type 2: Previous Day Daily Level Events (First Breaker Only)
SELECT trading_date, asset_id, cs_session, ps1_session, taken_level, taken_price, cs_price, cumulative_check
FROM daily_level_events WHERE rank_num = 1
UNION ALL
-- Type 3: Cross-Day Untaken Session Level Events
SELECT * FROM cross_day_events;







-------------------------------------

WITH daily_comparison AS (
    SELECT
        trading_date, asset_id,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdh,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdl,
        LAG(high, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd2h,
        LAG(low, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd2l,
        LAG(high, 3) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd3h,
        LAG(low, 3) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd3l
    FROM asset_daily_views
),
cumulative_checks AS (
    SELECT
        trading_date, asset_id, pdh, pdl,
        CASE WHEN (pdh > pd2h AND pd2h > pd3h) THEN '3-Day-High' WHEN (pdh > pd2h) THEN '2-Day-High' ELSE NULL END AS cumulative_high_check,
        CASE WHEN (pdl < pd2l AND pd2l < pd3l) THEN '3-Day-Low' WHEN (pdl < pd2l) THEN '2-Day-Low' ELSE NULL END AS cumulative_low_check
    FROM daily_comparison
),
session_sequence AS (
    -- Order Sessions Chronologically and pull 7-Bias
    SELECT
        trading_date, asset_id, session_name, high, low, start_ts,
        CASE WHEN session_type IN ('Bullish', 'Bearish') THEN session_type WHEN session_type = 'Consolidation' THEN consolidation_subtype ELSE 'Other' END AS bias_7_state,
        ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS session_rank
    FROM asset_session_views
),
-- =================================================================================
-- PART B: INTRA-DAY SEQUENTIAL TAKEDOWNS (PS1 -> CS on same day)
-- =================================================================================
intra_day_comparison AS (
    SELECT
        cs.trading_date, cs.asset_id, cs.session_name AS cs_session, cs.high AS cs_high, cs.low AS cs_low,
        LAG(cs.session_name, 1) OVER w AS ps1_session,
        LAG(cs.high, 1) OVER w AS ps1_high,
        LAG(cs.low, 1) OVER w AS ps1_low
    FROM session_sequence cs
    WHERE cs.session_rank > 1
    WINDOW w AS (PARTITION BY cs.trading_date, cs.asset_id ORDER BY cs.session_rank)
),
intra_day_events AS (
    -- High Takedown event
    SELECT trading_date, asset_id, cs_session, ps1_session, 'High' AS taken_level, ps1_high AS taken_price, cs_high AS cs_price, NULL::TEXT AS cumulative_check
    FROM intra_day_comparison WHERE ps1_session IS NOT NULL AND cs_high > ps1_high
    UNION ALL 
    -- Low Takedown event
    SELECT trading_date, asset_id, cs_session, ps1_session, 'Low' AS taken_level, ps1_low AS taken_price, cs_low AS cs_price, NULL::TEXT AS cumulative_check
    FROM intra_day_comparison WHERE ps1_session IS NOT NULL AND cs_low < ps1_low
),
-- =================================================================================
-- PART C: PREVIOUS DAY LEVEL TAKEDOWNS (PDH/PDL)
-- =================================================================================
daily_level_takedowns AS (
    SELECT
        s.trading_date, s.asset_id, s.session_name AS cs_session, s.start_ts, s.bias_7_state,
        'PrevDay-Daily' AS ps1_session,
        CASE WHEN s.high > cc.pdh THEN 'High' ELSE NULL END AS high_level,
        CASE WHEN s.high > cc.pdh THEN cc.pdh ELSE NULL END AS high_taken_price,
        CASE WHEN s.high > cc.pdh THEN s.high ELSE NULL END AS high_cs_price,
        cc.cumulative_high_check AS high_cumulative_check,
        CASE WHEN s.low < cc.pdl THEN 'Low' ELSE NULL END AS low_level,
        CASE WHEN s.low < cc.pdl THEN cc.pdl ELSE NULL END AS low_taken_price,
        CASE WHEN s.low < cc.pdl THEN s.low ELSE NULL END AS low_cs_price,
        cc.cumulative_low_check AS low_cumulative_check,
        ROW_NUMBER() OVER (PARTITION BY s.trading_date, s.asset_id, CASE WHEN s.high > cc.pdh THEN 'High' WHEN s.low < cc.pdl THEN 'Low' ELSE NULL END ORDER BY s.start_ts ASC) AS rank_num
    FROM session_sequence s
    JOIN cumulative_checks cc ON cc.trading_date = s.trading_date AND cc.asset_id = s.asset_id
    WHERE (s.high > cc.pdh AND cc.pdh IS NOT NULL) OR (s.low < cc.pdl AND cc.pdl IS NOT NULL)
),
daily_level_events AS (
    -- PDH Takedowns (High)
    SELECT trading_date, asset_id, cs_session, ps1_session, high_level AS taken_level, high_taken_price AS taken_price, high_cs_price AS cs_price, high_cumulative_check AS cumulative_check, rank_num
    FROM daily_level_takedowns WHERE high_level IS NOT NULL
    UNION ALL
    -- PDL Takedowns (Low)
    SELECT trading_date, asset_id, cs_session, ps1_session, low_level AS taken_level, low_taken_price AS taken_price, low_cs_price AS cs_price, low_cumulative_check AS cumulative_check, rank_num
    FROM daily_level_takedowns WHERE low_level IS NOT NULL
),
-- =================================================================================
-- PART D: UNTAKEN SESSION LEVEL TAKEDOWNS (Cross-Day)
-- =================================================================================
session_max_min AS (
    -- Calculates max/min of subsequent sessions on the SAME day for survival check
    SELECT
        s1.trading_date, s1.asset_id, s1.session_name, s1.high AS session_high, s1.low AS session_low,
        MAX(s2.high) AS max_subsequent_high,
        MIN(s2.low) AS min_subsequent_low
    FROM session_sequence s1
    LEFT JOIN session_sequence s2 ON s1.asset_id = s2.asset_id AND s1.trading_date = s2.trading_date AND s2.session_rank > s1.session_rank
    GROUP BY 1, 2, 3, 4, 5
),
survived_levels AS (
    -- Logs the levels that survived the rest of their trading day
    SELECT
        asset_id, trading_date, session_name, session_high, session_low,
        (max_subsequent_high IS NULL OR max_subsequent_high <= session_high) AS high_survived,
        (min_subsequent_low IS NULL OR min_subsequent_low >= session_low) AS low_survived
    FROM session_max_min
),
cross_day_takedown_events AS (
    -- Joins current sessions against the survivors from prior days
    SELECT
        cs.trading_date, cs.asset_id, cs.session_name AS cs_session, ss.session_name AS ps1_session,
        CASE WHEN cs.high > ss.session_high THEN 'High' ELSE NULL END AS high_taken_level,
        CASE WHEN cs.high > ss.session_high THEN ss.session_high ELSE NULL END AS high_taken_price,
        CASE WHEN cs.high > ss.session_high THEN cs.high ELSE NULL END AS high_cs_price,
        CASE WHEN cs.low < ss.session_low THEN 'Low' ELSE NULL END AS low_taken_level,
        CASE WHEN cs.low < ss.session_low THEN ss.session_low ELSE NULL END AS low_taken_price,
        CASE WHEN cs.low < ss.session_low THEN cs.low ELSE NULL END AS low_cs_price
    FROM asset_session_views cs
    JOIN survived_levels ss ON cs.asset_id = ss.asset_id
    WHERE cs.trading_date > ss.trading_date
      AND (
          (ss.high_survived = TRUE AND cs.high > ss.session_high) OR
          (ss.low_survived = TRUE AND cs.low < ss.session_low)
      )
),
cross_day_events AS (
    -- Unpivot and tag Cross-Day Takedowns
    SELECT trading_date, asset_id, cs_session, ps1_session, high_taken_level AS taken_level, high_taken_price AS taken_price, high_cs_price AS cs_price, 'Untaken-High' AS cumulative_check
    FROM cross_day_takedown_events WHERE high_taken_level IS NOT NULL
    UNION ALL
    SELECT trading_date, asset_id, cs_session, ps1_session, low_taken_level AS taken_level, low_taken_price AS taken_price, low_cs_price AS cs_price, 'Untaken-Low' AS cumulative_check
    FROM cross_day_takedown_events WHERE low_taken_level IS NOT NULL
),
-- =================================================================================
-- PART E: FINAL INSERTS
-- =================================================================================
first_daily_level_takedowns AS (
    SELECT
        trading_date,
        asset_id,
        cs_session AS session_name,
        MAX(bias_7_state) AS bias_7_state,
        MAX(CASE WHEN taken_level = 'High' THEN 1 ELSE 0 END) AS is_pdh_break,
        MAX(CASE WHEN taken_level = 'Low' THEN 1 ELSE 0 END) AS is_pdl_break
    FROM daily_level_events
    WHERE rank_num = 1
    GROUP BY trading_date, asset_id, cs_session
)
-- 1. INSERT INTO asset_survived_session_levels
INSERT INTO asset_survived_session_levels (
    asset_id, trading_date, session_name, session_high, session_low, high_survived, low_survived
)
SELECT * FROM survived_levels;
-- 2. INSERT INTO asset_structural_takedowns (First PDH/PDL Break Only)
INSERT INTO asset_structural_takedowns (
    asset_id, trading_date, session_name, bias_7_state, is_pdh_break, is_pdl_break
)
SELECT
    asset_id,
    trading_date,
    session_name,
    bias_7_state,
    is_pdh_break,
    is_pdl_break
FROM first_daily_level_takedowns;

----------------------------Divided

WITH session_sequence AS (
    -- Order Sessions Chronologically and pull 7-Bias
    SELECT
        trading_date, asset_id, session_name, high, low, start_ts,
        CASE WHEN session_type IN ('Bullish', 'Bearish') THEN session_type WHEN session_type = 'Consolidation' THEN consolidation_subtype ELSE 'Other' END AS bias_7_state,
        ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS session_rank
    FROM asset_session_views
),
session_max_min AS (
    -- Calculates max/min of subsequent sessions on the SAME day for survival check
    SELECT
        s1.trading_date, s1.asset_id, s1.session_name, s1.high AS session_high, s1.low AS session_low,
        MAX(s2.high) AS max_subsequent_high,
        MIN(s2.low) AS min_subsequent_low
    FROM session_sequence s1
    LEFT JOIN session_sequence s2 ON s1.asset_id = s2.asset_id AND s1.trading_date = s2.trading_date AND s2.session_rank > s1.session_rank
    GROUP BY 1, 2, 3, 4, 5
),
survived_levels AS (
    -- Logs the levels that survived the rest of their trading day
    SELECT
        asset_id, trading_date, session_name, session_high, session_low,
        (max_subsequent_high IS NULL OR max_subsequent_high <= session_high) AS high_survived,
        (min_subsequent_low IS NULL OR min_subsequent_low >= session_low) AS low_survived
    FROM session_max_min
)
-- INSERT INTO asset_survived_session_levels
INSERT INTO asset_survived_session_levels (
    asset_id, trading_date, session_name, session_high, session_low, high_survived, low_survived
)
SELECT * FROM survived_levels;

----------------------------------------
-- Take Down Event's
---------------------------------

WITH daily_comparison AS (
    SELECT
        trading_date, asset_id,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdh,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdl,
        LAG(high, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd2h,
        LAG(low, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd2l,
        LAG(high, 3) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd3h,
        LAG(low, 3) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd3l
    FROM asset_daily_views
),
cumulative_checks AS (
    SELECT
        trading_date, asset_id, pdh, pdl,
        CASE WHEN (pdh > pd2h AND pd2h > pd3h) THEN '3-Day-High' WHEN (pdh > pd2h) THEN '2-Day-High' ELSE NULL END AS cumulative_high_check,
        CASE WHEN (pdl < pd2l AND pd2l < pd3l) THEN '3-Day-Low' WHEN (pdl < pd2l) THEN '2-Day-Low' ELSE NULL END AS cumulative_low_check
    FROM daily_comparison
),
session_sequence AS (
    -- Order Sessions Chronologically and pull 7-Bias
    SELECT
        trading_date, asset_id, session_name, high, low, start_ts,
        CASE WHEN session_type IN ('Bullish', 'Bearish') THEN session_type WHEN session_type = 'Consolidation' THEN consolidation_subtype ELSE 'Other' END AS bias_7_state,
        ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS session_rank
    FROM asset_session_views
),
daily_level_takedowns AS (
    SELECT
        s.trading_date, s.asset_id, s.session_name AS cs_session, s.start_ts, s.bias_7_state,
        'PrevDay-Daily' AS ps1_session,
        CASE WHEN s.high > cc.pdh THEN 'High' ELSE NULL END AS high_level,
        CASE WHEN s.high > cc.pdh THEN cc.pdh ELSE NULL END AS high_taken_price,
        CASE WHEN s.high > cc.pdh THEN s.high ELSE NULL END AS high_cs_price,
        cc.cumulative_high_check AS high_cumulative_check,
        CASE WHEN s.low < cc.pdl THEN 'Low' ELSE NULL END AS low_level,
        CASE WHEN s.low < cc.pdl THEN cc.pdl ELSE NULL END AS low_taken_price,
        CASE WHEN s.low < cc.pdl THEN s.low ELSE NULL END AS low_cs_price,
        cc.cumulative_low_check AS low_cumulative_check,
        ROW_NUMBER() OVER (PARTITION BY s.trading_date, s.asset_id, CASE WHEN s.high > cc.pdh THEN 'High' WHEN s.low < cc.pdl THEN 'Low' ELSE NULL END ORDER BY s.start_ts ASC) AS rank_num
    FROM session_sequence s
    JOIN cumulative_checks cc ON cc.trading_date = s.trading_date AND cc.asset_id = s.asset_id
    WHERE (s.high > cc.pdh AND cc.pdh IS NOT NULL) OR (s.low < cc.pdl AND cc.pdl IS NOT NULL)
),
daily_level_events AS (
    -- PDH Takedowns (High)
    SELECT trading_date, asset_id, cs_session, ps1_session, high_level AS taken_level, high_taken_price AS taken_price, high_cs_price AS cs_price, high_cumulative_check AS cumulative_check, rank_num, bias_7_state
    FROM daily_level_takedowns WHERE high_level IS NOT NULL
    UNION ALL
    -- PDL Takedowns (Low)
    SELECT trading_date, asset_id, cs_session, ps1_session, low_level AS taken_level, low_taken_price AS taken_price, low_cs_price AS cs_price, low_cumulative_check AS cumulative_check, rank_num, bias_7_state
    FROM daily_level_takedowns WHERE low_level IS NOT NULL
),
first_daily_level_takedowns AS (
    SELECT
        trading_date,
        asset_id,
        cs_session AS session_name,
        MAX(bias_7_state) AS bias_7_state,
        BOOL_OR(taken_level = 'High') AS is_pdh_break,
        BOOL_OR(taken_level = 'Low') AS is_pdl_break
    FROM daily_level_events
    WHERE rank_num = 1
    GROUP BY trading_date, asset_id, cs_session
)
-- INSERT INTO asset_structural_takedowns (First PDH/PDL Break Only)
INSERT INTO asset_structural_takedowns (
    asset_id, trading_date, session_name, bias_7_state, is_pdh_break, is_pdl_break
)
SELECT
    asset_id,
    trading_date,
    session_name,
    bias_7_state,
    is_pdh_break,
    is_pdl_break
FROM first_daily_level_takedowns;
-----------------------------------

DROP TABLE IF EXISTS asset_takedown_events CASCADE;
CREATE TABLE IF NOT EXISTS asset_takedown_events (
    id BIGSERIAL PRIMARY KEY, -- New surrogate key
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    cs_session TEXT NOT NULL,
    ps1_session TEXT,
    taken_level TEXT NOT NULL,
    taken_price DOUBLE PRECISION,
    cs_price DOUBLE PRECISION,
    cumulative_check TEXT
);

TRUNCATE asset_takedown_events;



WITH daily_comparison AS (
    SELECT
        trading_date, asset_id,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdh,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdl,
        LAG(high, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd2h,
        LAG(low, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd2l,
        LAG(high, 3) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd3h,
        LAG(low, 3) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd3l
    FROM asset_daily_views
),
cumulative_checks AS (
    SELECT
        trading_date, asset_id, pdh, pdl,
        CASE WHEN (pdh > pd2h AND pd2h > pd3h) THEN '3-Day-High' WHEN (pdh > pd2h) THEN '2-Day-High' ELSE NULL END AS cumulative_high_check,
        CASE WHEN (pdl < pd2l AND pd2l < pd3l) THEN '3-Day-Low' WHEN (pdl < pd2l) THEN '2-Day-Low' ELSE NULL END AS cumulative_low_check
    FROM daily_comparison
),
session_sequence AS (
    -- Order Sessions Chronologically and pull 7-Bias
    SELECT
        trading_date, asset_id, session_name, high, low, start_ts,
        CASE WHEN session_type IN ('Bullish', 'Bearish') THEN session_type WHEN session_type = 'Consolidation' THEN consolidation_subtype ELSE 'Other' END AS bias_7_state,
        ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS session_rank
    FROM asset_session_views
),
intra_day_comparison AS (
    SELECT
        cs.trading_date, cs.asset_id, cs.session_name AS cs_session, cs.high AS cs_high, cs.low AS cs_low,
        LAG(cs.session_name, 1) OVER w AS ps1_session,
        LAG(cs.high, 1) OVER w AS ps1_high,
        LAG(cs.low, 1) OVER w AS ps1_low
    FROM session_sequence cs
    WHERE cs.session_rank > 1
    WINDOW w AS (PARTITION BY cs.trading_date, cs.asset_id ORDER BY cs.session_rank)
),
intra_day_events_pre AS (
    SELECT
        trading_date, asset_id, cs_session, ps1_session, 'High' AS taken_level, ps1_high AS taken_price, cs_high AS cs_price, NULL::TEXT AS cumulative_check
    FROM intra_day_comparison WHERE ps1_session IS NOT NULL AND cs_high > ps1_high
    UNION ALL
    SELECT trading_date, asset_id, cs_session, ps1_session, 'Low' AS taken_level, ps1_low AS taken_price, cs_low AS cs_price, NULL::TEXT AS cumulative_check
    FROM intra_day_comparison WHERE ps1_session IS NOT NULL AND cs_low < ps1_low
),
intra_day_events AS (
    SELECT *,
           ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id, cs_session, ps1_session, taken_level ORDER BY cs_price) as row_id
    FROM intra_day_events_pre
),
daily_level_takedowns AS (
    SELECT
        s.trading_date, s.asset_id, s.session_name AS cs_session, s.start_ts, s.bias_7_state,
        'PrevDay-Daily' AS ps1_session,
        CASE WHEN s.high > cc.pdh THEN 'High' ELSE NULL END AS high_level,
        CASE WHEN s.high > cc.pdh THEN cc.pdh ELSE NULL END AS high_taken_price,
        CASE WHEN s.high > cc.pdh THEN s.high ELSE NULL END AS high_cs_price,
        cc.cumulative_high_check AS high_cumulative_check,
        CASE WHEN s.low < cc.pdl THEN 'Low' ELSE NULL END AS low_level,
        CASE WHEN s.low < cc.pdl THEN cc.pdl ELSE NULL END AS low_taken_price,
        CASE WHEN s.low < cc.pdl THEN s.low ELSE NULL END AS low_cs_price,
        cc.cumulative_low_check AS low_cumulative_check,
        ROW_NUMBER() OVER (PARTITION BY s.trading_date, s.asset_id, CASE WHEN s.high > cc.pdh THEN 'High' WHEN s.low < cc.pdl THEN 'Low' ELSE NULL END ORDER BY s.start_ts ASC) AS rank_num
    FROM session_sequence s
    JOIN cumulative_checks cc ON cc.trading_date = s.trading_date AND cc.asset_id = s.asset_id
    WHERE (s.high > cc.pdh AND cc.pdh IS NOT NULL) OR (s.low < cc.pdl AND cc.pdl IS NOT NULL)
),
daily_level_events AS (
    SELECT trading_date, asset_id, cs_session, ps1_session, high_level AS taken_level, high_taken_price AS taken_price, high_cs_price AS cs_price, high_cumulative_check AS cumulative_check, ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id, cs_session, ps1_session, high_level ORDER BY high_cs_price) AS row_id
    FROM daily_level_takedowns WHERE high_level IS NOT NULL
    UNION ALL
    SELECT trading_date, asset_id, cs_session, ps1_session, low_level AS taken_level, low_taken_price AS taken_price, low_cs_price AS cs_price, low_cumulative_check AS cumulative_check, ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id, cs_session, ps1_session, low_level ORDER BY low_cs_price) AS row_id
    FROM daily_level_takedowns WHERE low_level IS NOT NULL
),
session_max_min AS (
    SELECT
        s1.trading_date, s1.asset_id, s1.session_name, s1.high AS session_high, s1.low AS session_low,
        MAX(s2.high) AS max_subsequent_high,
        MIN(s2.low) AS min_subsequent_low
    FROM session_sequence s1
    LEFT JOIN session_sequence s2 ON s1.asset_id = s2.asset_id AND s1.trading_date = s2.trading_date AND s2.session_rank > s1.session_rank
    GROUP BY 1, 2, 3, 4, 5
),
survived_levels AS (
    SELECT
        asset_id, trading_date, session_name, session_high, session_low,
        (max_subsequent_high IS NULL OR max_subsequent_high <= session_high) AS high_survived,
        (min_subsequent_low IS NULL OR min_subsequent_low >= session_low) AS low_survived
    FROM session_max_min
),
cross_day_takedown_events_pre AS (
    SELECT
        cs.trading_date, cs.asset_id, cs.session_name AS cs_session, ss.session_name AS ps1_session,
        CASE WHEN cs.high > ss.session_high THEN 'High' ELSE NULL END AS high_taken_level,
        CASE WHEN cs.high > ss.session_high THEN ss.session_high ELSE NULL END AS high_taken_price,
        CASE WHEN cs.high > ss.session_high THEN cs.high ELSE NULL END AS high_cs_price,
        CASE WHEN cs.low < ss.session_low THEN 'Low' ELSE NULL END AS low_taken_level,
        CASE WHEN cs.low < ss.session_low THEN ss.session_low ELSE NULL END AS low_taken_price,
        CASE WHEN cs.low < ss.session_low THEN cs.low ELSE NULL END AS low_cs_price
    FROM asset_session_views cs
    JOIN survived_levels ss ON cs.asset_id = ss.asset_id
    WHERE cs.trading_date > ss.trading_date
      AND (
          (ss.high_survived = TRUE AND cs.high > ss.session_high) OR
          (ss.low_survived = TRUE AND cs.low < ss.session_low)
      )
),
cross_day_events AS (
    SELECT trading_date, asset_id, cs_session, ps1_session, high_taken_level AS taken_level, high_taken_price AS taken_price, high_cs_price AS cs_price, 'Untaken-High' AS cumulative_check, ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id, cs_session, ps1_session, high_taken_level ORDER BY high_cs_price) AS row_id
    FROM cross_day_takedown_events_pre WHERE high_taken_level IS NOT NULL
    UNION ALL
    SELECT trading_date, asset_id, cs_session, ps1_session, low_taken_level AS taken_level, low_taken_price AS taken_price, low_cs_price AS cs_price, 'Untaken-Low' AS cumulative_check, ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id, cs_session, ps1_session, low_taken_level ORDER BY low_cs_price) AS row_id
    FROM cross_day_takedown_events_pre WHERE low_taken_level IS NOT NULL
),
all_takedown_events AS (
    SELECT trading_date, asset_id, cs_session, ps1_session, taken_level, taken_price, cs_price, cumulative_check
    FROM intra_day_events
    UNION ALL
    SELECT trading_date, asset_id, cs_session, ps1_session, taken_level, taken_price, cs_price, cumulative_check
    FROM daily_level_events
    UNION ALL
    SELECT trading_date, asset_id, cs_session, ps1_session, taken_level, taken_price, cs_price, cumulative_check
    FROM cross_day_events
)
INSERT INTO asset_takedown_events (
    trading_date, asset_id, cs_session, ps1_session, taken_level, taken_price, cs_price, cumulative_check
)
SELECT
    trading_date,
    asset_id,
    cs_session,
    ps1_session,
    taken_level,
    taken_price,
    cs_price,
    cumulative_check
FROM all_takedown_events;

----------------------------------------------------------
-- Drop existing tables (if any) to avoid conflicts
DROP TABLE IF EXISTS asset_structural_takedowns CASCADE;
CREATE TABLE IF NOT EXISTS asset_structural_takedowns (
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    session_name TEXT NOT NULL,
    bias_7_state TEXT NOT NULL,
    is_pdh_break BOOLEAN NOT NULL,
    is_pdl_break BOOLEAN NOT NULL,
    PRIMARY KEY (asset_id, trading_date, session_name)
);
TRUNCATE asset_structural_takedowns;

DROP TABLE IF EXISTS asset_takedown_events CASCADE;
CREATE TABLE IF NOT EXISTS asset_takedown_events (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    cs_session TEXT NOT NULL,
    ps1_session TEXT NOT NULL,
    taken_level TEXT NOT NULL,
    taken_price DOUBLE PRECISION NOT NULL,
    cs_price DOUBLE PRECISION NOT NULL,
    cumulative_check TEXT,
    PRIMARY KEY (trading_date, asset_id, cs_session, ps1_session, taken_level)
);
TRUNCATE asset_takedown_events;

DROP TABLE IF EXISTS asset_survived_session_levels CASCADE;
CREATE TABLE IF NOT EXISTS asset_survived_session_levels (
    asset_id TEXT NOT NULL,
    trading_date DATE NOT NULL,
    session_name TEXT NOT NULL,
    session_high DOUBLE PRECISION NOT NULL,
    session_low DOUBLE PRECISION NOT NULL,
    high_survived BOOLEAN NOT NULL,
    low_survived BOOLEAN NOT NULL,
    PRIMARY KEY (asset_id, trading_date, session_name)
);
TRUNCATE asset_survived_session_levels;

-- PART A: FOUNDATION AND CUMULATIVE CHECKS
WITH daily_comparison AS (
    SELECT
        trading_date, asset_id,
        LAG(high, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdh,
        LAG(low, 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pdl,
        LAG(high, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd2h,
        LAG(low, 2) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd2l,
        LAG(high, 3) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd3h,
        LAG(low, 3) OVER (PARTITION BY asset_id ORDER BY trading_date) AS pd3l
    FROM asset_daily_views
),
cumulative_checks AS (
    SELECT
        trading_date, asset_id, pdh, pdl,
        CASE WHEN (pdh > pd2h AND pd2h > pd3h) THEN '3-Day-High' 
             WHEN (pdh > pd2h) THEN '2-Day-High' 
             ELSE NULL 
        END AS cumulative_high_check,
        CASE WHEN (pdl < pd2l AND pd2l < pd3l) THEN '3-Day-Low' 
             WHEN (pdl < pd2l) THEN '2-Day-Low' 
             ELSE NULL 
        END AS cumulative_low_check
    FROM daily_comparison
),
session_sequence AS (
    SELECT
        trading_date, asset_id, session_name, high, low, start_ts,
        CASE 
            WHEN session_type IN ('Bullish', 'Bearish') THEN session_type 
            WHEN session_type = 'Consolidation' THEN consolidation_subtype 
            ELSE 'Other' 
        END AS bias_7_state,
        ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS session_rank
    FROM asset_session_views
),
intra_day_comparison AS (
    SELECT
        cs.trading_date, cs.asset_id, cs.session_name AS cs_session, cs.high AS cs_high, cs.low AS cs_low,
        LAG(cs.session_name, 1) OVER w AS ps1_session,
        LAG(cs.high, 1) OVER w AS ps1_high,
        LAG(cs.low, 1) OVER w AS ps1_low
    FROM session_sequence cs
    WINDOW w AS (PARTITION BY cs.trading_date, cs.asset_id ORDER BY cs.session_rank)
    WHERE cs.session_rank > 1
),
intra_day_events AS (
    -- High Takedown event
    SELECT trading_date, asset_id, cs_session, ps1_session, 'High' AS taken_level, ps1_high AS taken_price, cs_high AS cs_price, NULL::TEXT AS cumulative_check
    FROM intra_day_comparison WHERE ps1_session IS NOT NULL AND cs_high > ps1_high
    UNION ALL 
    -- Low Takedown event
    SELECT trading_date, asset_id, cs_session, ps1_session, 'Low' AS taken_level, ps1_low AS taken_price, cs_low AS cs_price, NULL::TEXT AS cumulative_check
    FROM intra_day_comparison WHERE ps1_session IS NOT NULL AND cs_low < ps1_low
),
daily_level_takedowns AS (
    SELECT
        s.trading_date, s.asset_id, s.session_name AS cs_session, s.start_ts, s.bias_7_state,
        'PrevDay-Daily' AS ps1_session,
        CASE WHEN s.high > cc.pdh THEN 'High' ELSE NULL END AS high_level,
        CASE WHEN s.high > cc.pdh THEN cc.pdh ELSE NULL END AS high_taken_price,
        CASE WHEN s.high > cc.pdh THEN s.high ELSE NULL END AS high_cs_price,
        cc.cumulative_high_check AS high_cumulative_check,
        CASE WHEN s.low < cc.pdl THEN 'Low' ELSE NULL END AS low_level,
        CASE WHEN s.low < cc.pdl THEN cc.pdl ELSE NULL END AS low_taken_price,
        CASE WHEN s.low < cc.pdl THEN s.low ELSE NULL END AS low_cs_price,
        cc.cumulative_low_check AS low_cumulative_check,
        ROW_NUMBER() OVER (PARTITION BY s.trading_date, s.asset_id, 
            CASE WHEN s.high > cc.pdh THEN 'High' 
                 WHEN s.low < cc.pdl THEN 'Low' ELSE NULL END 
            ORDER BY s.start_ts ASC) AS rank_num
    FROM session_sequence s
    JOIN cumulative_checks cc ON cc.trading_date = s.trading_date AND cc.asset_id = s.asset_id
    WHERE (s.high > cc.pdh AND cc.pdh IS NOT NULL) OR (s.low < cc.pdl AND cc.pdl IS NOT NULL)
),
daily_level_events AS (
    -- PDH Takedowns (High)
    SELECT trading_date, asset_id, cs_session, ps1_session, high_level AS taken_level, high_taken_price AS taken_price, high_cs_price AS cs_price, high_cumulative_check AS cumulative_check, rank_num
    FROM daily_level_takedowns WHERE high_level IS NOT NULL
    UNION ALL
    -- PDL Takedowns (Low)
    SELECT trading_date, asset_id, cs_session, ps1_session, low_level AS taken_level, low_taken_price AS taken_price, low_cs_price AS cs_price, low_cumulative_check AS cumulative_check, rank_num
    FROM daily_level_takedowns WHERE low_level IS NOT NULL
),
session_max_min AS (
    -- Calculates max/min of subsequent sessions on the SAME day for survival check
    SELECT
        s1.trading_date, s1.asset_id, s1.session_name, s1.high AS session_high, s1.low AS session_low,
        MAX(s2.high) AS max_subsequent_high,
        MIN(s2.low) AS min_subsequent_low
    FROM session_sequence s1
    LEFT JOIN session_sequence s2 ON s1.asset_id = s2.asset_id AND s1.trading_date = s2.trading_date AND s2.session_rank > s1.session_rank
    GROUP BY s1.trading_date, s1.asset_id, s1.session_name, s1.high, s1.low
),
survived_levels AS (
    -- Logs the levels that survived the rest of their trading day
    SELECT
        asset_id, trading_date, session_name, session_high, session_low,
        (max_subsequent_high IS NULL OR max_subsequent_high <= session_high) AS high_survived,
        (min_subsequent_low IS NULL OR min_subsequent_low >= session_low) AS low_survived
    FROM session_max_min
),
cross_day_takedown_events AS (
    -- Joins current sessions against survivors from prior days
    SELECT
        cs.trading_date, cs.asset_id, cs.session_name AS cs_session, ss.session_name AS ps1_session,
        CASE WHEN cs.high > ss.session_high THEN 'High' ELSE NULL END AS high_taken_level,
        CASE WHEN cs.high > ss.session_high THEN ss.session_high ELSE NULL END AS high_taken_price,
        CASE WHEN cs.high > ss.session_high THEN cs.high ELSE NULL END AS high_cs_price,
        CASE WHEN cs.low < ss.session_low THEN 'Low' ELSE NULL END AS low_taken_level,
        CASE WHEN cs.low < ss.session_low THEN ss.session_low ELSE NULL END AS low_taken_price,
        CASE WHEN cs.low < ss.session_low THEN cs.low ELSE NULL END AS low_cs_price
    FROM asset_session_views cs
    JOIN survived_levels ss ON cs.asset_id = ss.asset_id
    WHERE cs.trading_date > ss.trading_date
      AND (
          (ss.high_survived = TRUE AND cs.high > ss.session_high) OR
          (ss.low_survived = TRUE AND cs.low < ss.session_low)
      )
),
cross_day_events AS (
    -- Unpivot and tag Cross-Day Takedowns
    SELECT trading_date, asset_id, cs_session, ps1_session, high_taken_level AS taken_level, high_taken_price AS taken_price, high_cs_price AS cs_price, 'Untaken-High' AS cumulative_check
    FROM cross_day_takedown_events WHERE high_taken_level IS NOT NULL
    UNION ALL
    SELECT trading_date, asset_id, cs_session, ps1_session, low_taken_level AS taken_level, low_taken_price AS taken_price, low_cs_price AS cs_price, 'Untaken-Low' AS cumulative_check
    FROM cross_day_takedown_events WHERE low_taken_level IS NOT NULL
)
-- =================================================================================
-- PART E: FINAL INSERTS
-- =================================================================================
-- 1. INSERT INTO asset_survived_session_levels
INSERT INTO asset_survived_session_levels (
    asset_id, trading_date, session_name, session_high, session_low, high_survived, low_survived
)
SELECT * FROM survived_levels;
-- 2. INSERT INTO asset_structural_takedowns (First PDH/PDL Break Only)
INSERT INTO asset_structural_takedowns (
    asset_id, trading_date, session_name, bias_7_state, is_pdh_break, is_pdl_break
)
SELECT DISTINCT ON (trading_date, asset_id, cs_session)
    s.asset_id, s.trading_date, s.cs_session, s.bias_7_state,
    MAX(CASE WHEN s.taken_level = 'High' AND s.rank_num = 1 THEN TRUE ELSE FALSE END) AS is_pdh_break,
    MAX(CASE WHEN s.taken_level = 'Low' AND s.rank_num = 1 THEN TRUE ELSE FALSE END) AS is_pdl_break
FROM daily_level_takedowns s
GROUP BY s.asset_id, s.trading_date, s.cs_session, s.bias_7_state;

-- 3. INSERT INTO asset_takedown_events
INSERT INTO asset_takedown_events (
    trading_date, asset_id, cs_session, ps1_session, taken_level, 
    taken_price, cs_price, cumulative_check
)
SELECT * FROM intra_day_events
UNION ALL
SELECT trading_date, asset_id, cs_session, ps1_session, taken_level, taken_price, cs_price, cumulative_check
FROM daily_level_events WHERE rank_num = 1
UNION ALL
SELECT * FROM cross_day_events;


