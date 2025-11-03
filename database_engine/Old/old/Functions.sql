-- Define the composite type to hold both the primary and secondary classification
DROP TYPE IF EXISTS market_classification CASCADE;
CREATE TYPE market_classification AS (
    session_type TEXT,
    consolidation_subtype TEXT
);
-------------
-- ...existing code...
CREATE OR REPLACE FUNCTION get_market_type(
    open_val double precision, 
    high_val double precision, 
    low_val double precision, 
    close_val double precision
)
RETURNS market_classification LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    session_range DOUBLE PRECISION;
    body_size DOUBLE PRECISION;
    upper_wick DOUBLE PRECISION;
    lower_wick DOUBLE PRECISION;
    
    body_range_ratio DOUBLE PRECISION;
    upper_wick_ratio DOUBLE PRECISION;
    lower_wick_ratio DOUBLE PRECISION;
    
    -- SYSTEM THRESHOLDS
    CONSOLIDATION_BODY_RATIO CONSTANT DOUBLE PRECISION := 0.20;
    OPPOSING_WICK_PRESSURE CONSTANT DOUBLE PRECISION := 0.40;
    REVERSAL_WICK_DOMINANCE CONSTANT DOUBLE PRECISION := 2.0;
    BALANCED_WICK_RATIO CONSTANT DOUBLE PRECISION := 0.50;
BEGIN
    session_range := high_val - low_val;
    
    -- 1. Handle Edge Cases
    IF session_range <= 0.00000001 THEN
        RETURN ROW('Consolidation', 'Pure_Indecision')::market_classification;
    END IF;

    -- Calculate metrics
    body_size := abs(close_val - open_val);
    upper_wick := high_val - greatest(open_val, close_val);
    lower_wick := least(open_val, close_val) - low_val;
    
    body_range_ratio := CASE WHEN session_range = 0 THEN 0 ELSE body_size / session_range END;
    upper_wick_ratio := CASE WHEN session_range = 0 THEN 0 ELSE upper_wick / session_range END;
    lower_wick_ratio := CASE WHEN session_range = 0 THEN 0 ELSE lower_wick / session_range END;

    -- =======================================================
    -- 2. TREND CLASSIFICATION (BULLISH/BEARISH)
    -- =======================================================
    IF close_val > open_val THEN
        -- Potential Bullish: apply veto checks
        IF upper_wick_ratio < OPPOSING_WICK_PRESSURE AND body_range_ratio >= CONSOLIDATION_BODY_RATIO THEN
            RETURN ROW('Bullish', NULL)::market_classification;
        END IF;
        -- else fall through to consolidation logic
    ELSIF close_val < open_val THEN
        -- Potential Bearish: apply veto checks
        IF lower_wick_ratio < OPPOSING_WICK_PRESSURE AND body_range_ratio >= CONSOLIDATION_BODY_RATIO THEN
            RETURN ROW('Bearish', NULL)::market_classification;
        END IF;
        -- else fall through to consolidation logic
    END IF;

    -- =======================================================
    -- 3. CONSOLIDATION CLASSIFICATION
    -- =======================================================
    IF body_range_ratio < CONSOLIDATION_BODY_RATIO THEN
        -- Reversal patterns
        IF lower_wick >= REVERSAL_WICK_DOMINANCE * GREATEST(body_size, 1e-12) THEN
            RETURN ROW('Consolidation', 'Bullish_Reversal')::market_classification;
        ELSIF upper_wick >= REVERSAL_WICK_DOMINANCE * GREATEST(body_size, 1e-12) THEN
            RETURN ROW('Consolidation', 'Bearish_Reversal')::market_classification;
        END IF;

        -- Pure indecision or fallback
        IF LEAST(upper_wick, lower_wick) >= BALANCED_WICK_RATIO * GREATEST(upper_wick, lower_wick) THEN
            RETURN ROW('Consolidation', 'Pure_Indecision')::market_classification;
        END IF;

        RETURN ROW('Consolidation', 'Pure_Indecision')::market_classification;
    END IF;

    -- Failed trend checks (if we reached here because vetoes fired)
    IF upper_wick_ratio >= OPPOSING_WICK_PRESSURE THEN
        RETURN ROW('Consolidation', 'Failed_Bullish')::market_classification;
    ELSIF lower_wick_ratio >= OPPOSING_WICK_PRESSURE THEN
        RETURN ROW('Consolidation', 'Failed_Bearish')::market_classification;
    END IF;

    -- Final fallback
    RETURN ROW('Consolidation', 'Other')::market_classification;
END;
$$;
-- ...existing code...

CREATE OR REPLACE FUNCTION get_trading_date(ts TIMESTAMPTZ)
RETURNS DATE LANGUAGE SQL IMMUTABLE AS $$
    SELECT ((ts AT TIME ZONE 'America/New_York') + INTERVAL '6 hours')::date;
$$;