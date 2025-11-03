DROP TABLE IF EXISTS asset_takedown_outcome_matrix CASCADE;
CREATE TABLE IF NOT EXISTS asset_takedown_outcome_matrix (
    asset_id TEXT NOT NULL,
    cs_session TEXT NOT NULL,                     -- Formerly breaker_session
    ps1_session TEXT NOT NULL,                    -- Formerly prior_session
    prior_level TEXT NOT NULL,
    total_takedowns BIGINT NOT NULL,
    ft_count BIGINT NOT NULL,
    fr_count BIGINT NOT NULL,
    n_count BIGINT NOT NULL,
    p_follow_through DOUBLE PRECISION NOT NULL,
    p_reversal DOUBLE PRECISION NOT NULL,
    p_neutral DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (asset_id, cs_session, ps1_session, prior_level)
);
TRUNCATE asset_takedown_outcome_matrix;

-- =================================================================================
-- SUB-TRACK 2: PROBABILITY MATRIX CALCULATION (GENERALIZED & RENAMED)
-- =================================================================================
WITH numbered_sessions AS (
    -- 1. Order Sessions to easily find the Breaker Session's rank and subsequent sessions
    SELECT
        trading_date, asset_id, session_name, high, low, start_ts,
        ROW_NUMBER() OVER (PARTITION BY trading_date, asset_id ORDER BY start_ts) AS session_rank
    FROM asset_session_views
),
event_details AS (
    -- 2. Combine Takedown Events (TDE) with Breaker Session (CS) data
    SELECT
        tde.trading_date,
        tde.asset_id,
        tde.breaker_session AS cs_session,                        -- Current Session (The Breaker)
        tde.prior_session AS ps1_session,                       -- Prior Session (Level Source)
        tde.taken_level AS prior_level,
        tde.cs_price AS breaker_price,
        ns.session_rank AS cs_rank,
        ns.high AS cs_high,
        ns.low AS cs_low
    FROM asset_takedown_events tde
    JOIN numbered_sessions ns
        ON tde.trading_date = ns.trading_date
        AND tde.asset_id = ns.asset_id
        AND tde.cs_session = ns.session_name
),
subsequent_range AS (
    -- 3. Find the maximum High and minimum Low for all sessions AFTER the Breaker Session (CS)
    SELECT
        ed.*,
        -- Subsequent High: Max high of all sessions with a higher rank (i.e., later in the day)
        MAX(ns.high) OVER (
            PARTITION BY ns.trading_date, ns.asset_id
            ORDER BY ns.session_rank
            ROWS BETWEEN 1 FOLLOWING AND UNBOUNDED FOLLOWING
        ) AS subsequent_high,        
        -- Subsequent Low: Min low of all sessions with a higher rank (i.e., later in the day)
        MIN(ns.low) OVER (
            PARTITION BY ns.trading_date, ns.asset_id
            ORDER BY ns.session_rank
            ROWS BETWEEN 1 FOLLOWING AND UNBOUNDED FOLLOWING
        ) AS subsequent_low
    FROM event_details ed
    JOIN numbered_sessions ns
        ON ed.trading_date = ns.trading_date
        AND ed.asset_id = ns.asset_id
        AND ed.cs_rank = ns.session_rank -- Join back to the CS row to use the window function properly
),
classified_events AS (
    -- 4. Classify each event outcome (FT, FR, or N)
    SELECT
        trading_date, asset_id, cs_session, ps1_session, prior_level,        
        CASE
            -- Case 1: Takedown was UP (High Level Broken)
            WHEN prior_level IN ('High', 'Untaken-High') THEN
                CASE
                    WHEN subsequent_high > breaker_price THEN 'FT'
                    WHEN subsequent_low < cs_low THEN 'FR'
                    ELSE 'N'
                END
            -- Case 2: Takedown was DOWN (Low Level Broken)
            WHEN prior_level IN ('Low', 'Untaken-Low') THEN
                CASE
                    WHEN subsequent_low < breaker_price THEN 'FT'
                    WHEN subsequent_high > cs_high THEN 'FR'
                    ELSE 'N'
                END            
            ELSE 'N'
        END AS outcome        
    FROM subsequent_range
),
aggregated_counts AS (
    -- 5. Aggregate the counts by asset and pattern (CS_Session, PS1_Session, Prior_Level)
    SELECT
        asset_id, cs_session, ps1_session, prior_level,
        COUNT(*) AS total_takedowns,
        SUM(CASE WHEN outcome = 'FT' THEN 1 ELSE 0 END) AS ft_count,
        SUM(CASE WHEN outcome = 'FR' THEN 1 ELSE 0 END) AS fr_count,
        SUM(CASE WHEN outcome = 'N' THEN 1 ELSE 0 END) AS n_count
    FROM classified_events
    GROUP BY 1, 2, 3, 4
)
-- 6. FINAL INSERT: Calculate Probabilities and store the matrix
INSERT INTO asset_takedown_outcome_matrix (
    asset_id, cs_session, ps1_session, prior_level, 
    total_takedowns, ft_count, fr_count, n_count, 
    p_follow_through, p_reversal, p_neutral
)
SELECT
    asset_id, cs_session, ps1_session, prior_level,
    total_takedowns,
    ft_count,
    fr_count,
    n_count,
    ROUND((ft_count::NUMERIC / total_takedowns) * 100, 2) AS p_follow_through,
    ROUND((fr_count::NUMERIC / total_takedowns) * 100, 2) AS p_reversal,
    ROUND((n_count::NUMERIC / total_takedowns) * 100, 2) AS p_neutral
FROM aggregated_counts
WHERE total_takedowns > 0;
