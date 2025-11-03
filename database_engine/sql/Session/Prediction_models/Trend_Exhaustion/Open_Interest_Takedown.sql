----------------------------------
-- Open Interest Takedown Analysis
----------------------------------

DROP TABLE IF EXISTS asset_open_interest_takedown CASCADE;
CREATE TABLE IF NOT EXISTS asset_open_interest_takedown (
    asset_id TEXT NOT NULL,
    ps1_name TEXT NOT NULL,
    ps1_bias_7 TEXT NOT NULL,                       -- The bias of the current trend session (PS1)
    cs_name TEXT NOT NULL,                         -- The session where the potential OIT is observed (CS)
    trend_direction TEXT NOT NULL, 
    cs_ps1_fk BIGINT NOT NULL,                      -- 'Bullish' or 'Bearish' (of the PS1 bias)

    total_patterns BIGINT NOT NULL,                -- Denominator: Total occurrences of the PS1->CS pattern
    
    oit_takedown_count BIGINT NOT NULL,            -- Numerator: Count of times the OIT condition was met
    oit_score DOUBLE PRECISION NOT NULL,           -- OIT_Takedown_Count / Total_Patterns
    
    avg_pullback_pct DOUBLE PRECISION NOT NULL,    -- Average magnitude of the counter-move (as a % of CS range)
    
    oit_label TEXT NOT NULL,                       -- 'Exhausted' (High OIT) or 'Committed' (Low OIT)
    
    PRIMARY KEY (asset_id, ps1_name, ps1_bias_7, cs_name, trend_direction)
);
TRUNCATE asset_open_interest_takedown;



WITH cs_directional_moves AS (
    -- 1. Identify the directional move and range of the CS session
    SELECT
        uc.asset_id,
        uc.cs_ps1_fk,
        uc.ps1_name, -- Add ps1_name here
        uc.ps1_bias_7_state AS ps1_bias_7,
        uc.cs_name,
        sv.high,
        sv.low,
        sv.open,
        sv.close,
        (sv.high - sv.low) AS cs_range,
        -- Determine the primary directional move of the CS
        CASE
            WHEN sv.close > sv.open THEN 'BULLISH'
            WHEN sv.close < sv.open THEN 'BEARISH'
            ELSE 'NEUTRAL'
        END AS cs_direction
        
    FROM asset_session_context uc
    JOIN asset_session_views sv 
        ON uc.asset_id = sv.asset_id AND uc.trading_date = sv.trading_date AND uc.cs_name = sv.session_name
    WHERE sv.high IS NOT NULL AND sv.low IS NOT NULL AND (sv.high - sv.low) > 0.001 -- Filter out flat sessions
),
oit_events AS (
    -- 2. Apply the OIT Condition (Pullback > 50% of CS Range)
    SELECT
        c.asset_id,
        c.cs_ps1_fk,
        c.ps1_name, -- Add ps1_name here
        c.ps1_bias_7,
        c.cs_name,
        c.cs_direction AS trend_direction,
        c.cs_range,
        -- Calculate the counter-directional pullback size
        CASE
            -- BULLISH CS: Pullback = High - Close
            WHEN c.cs_direction = 'BULLISH' THEN (c.high - c.close)
            -- BEARISH CS: Pullback = Close - Low
            WHEN c.cs_direction = 'BEARISH' THEN (c.close - c.low)
            ELSE 0.0
        END AS pullback_size,
        -- OIT Condition: Is Pullback > 50% of the Total CS Range?
        CASE
            WHEN (c.cs_direction IN ('BULLISH', 'BEARISH')) 
                 AND ( (c.cs_direction = 'BULLISH' AND (c.high - c.close) > (c.cs_range * 0.50))
                       OR (c.cs_direction = 'BEARISH' AND (c.close - c.low) > (c.cs_range * 0.50))
                     ) THEN 1
            ELSE 0
        END AS is_oit_takedown
    FROM cs_directional_moves c
    WHERE c.cs_direction IN ('BULLISH', 'BEARISH')
),
aggregated_oit AS (
    -- 3. Aggregate results by the PS1->CS pattern
    SELECT
        o.asset_id,
        o.ps1_name, -- Add ps1_name here
        o.ps1_bias_7,
        o.cs_name,
        o.trend_direction,
        
        COUNT(*) AS total_patterns,
        SUM(o.is_oit_takedown) AS oit_takedown_count,
        AVG(o.pullback_size / o.cs_range) * 100 AS avg_pullback_pct -- Average exhaustion magnitude
        
    FROM oit_events o
    GROUP BY 1, 2, 3, 4, 5 -- Add ps1_name to GROUP BY
)
-- 4. Final Insertion into M14
INSERT INTO asset_open_interest_takedown (
    asset_id, cs_ps1_fk, ps1_name, ps1_bias_7, cs_name, trend_direction, total_patterns, 
    oit_takedown_count, oit_score, avg_pullback_pct, oit_label
)
SELECT
    a.asset_id,
    -- cs_ps1_fk is missing in aggregation, so we need a placeholder.
    -- However, the PRIMARY KEY includes ps1_name, so this will only work if cs_ps1_fk
    -- is not part of the unique key and you don't need its aggregated value.
    -1 AS cs_ps1_fk, 
    a.ps1_name,
    a.ps1_bias_7,
    a.cs_name,
    a.trend_direction,
    
    a.total_patterns,
    a.oit_takedown_count,
    (a.oit_takedown_count::DOUBLE PRECISION / NULLIF(a.total_patterns, 0)) AS oit_score,
    a.avg_pullback_pct,
    
    CASE
        WHEN (a.oit_takedown_count::DOUBLE PRECISION / NULLIF(a.total_patterns, 0)) >= 0.60 
            THEN 'Exhausted (High Internal Risk)'
        ELSE 'Committed'
    END AS oit_label
FROM aggregated_oit a
WHERE a.total_patterns > 0;

