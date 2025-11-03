-----------------------------------------------
-- 1 Candle Pattern Function
-----------------------------------------------

CREATE OR REPLACE FUNCTION get_candle_pattern(o double precision, h double precision, l double precision, c double precision)
RETURNS TEXT
AS $$
DECLARE
    body_size DOUBLE PRECISION;
    total_range DOUBLE PRECISION;
    body_ratio DOUBLE PRECISION;
    shadow_ratio DOUBLE PRECISION;
BEGIN
    body_size := ABS(c - o);
    total_range := h - l;

    -- Avoid division by zero
    IF total_range = 0 OR total_range < 0.0001 THEN
        RETURN 'Flat';
    END IF;

    body_ratio := body_size / total_range;

    -- 1. Doji Check
    IF body_ratio < 0.05 THEN
        RETURN 'Doji';
    END IF;

    -- 2. Marubozu Check 
    IF body_ratio > 0.9 THEN
        RETURN CASE WHEN c > o THEN 'Bullish Marubozu' ELSE 'Bearish Marubozu' END;
    END IF;

    -- 3. Hammer/Hanging Man Check 
    IF body_ratio < 0.3 THEN 
        IF c > o THEN -- Bullish Candle
            -- Hammer: Long lower shadow
            shadow_ratio := (o - l) / body_size;
            IF shadow_ratio >= 2.0 THEN RETURN 'Hammer'; END IF;
        ELSE -- Bearish Candle
            -- Hanging Man: Long upper shadow
            shadow_ratio := (h - o) / body_size;
            IF shadow_ratio >= 2.0 THEN RETURN 'Hanging Man'; END IF;
        END IF;
    END IF;
    
    -- 4. Standard Candles
    RETURN CASE WHEN c > o THEN 'Bullish' ELSE 'Bearish' END;
END
$$ LANGUAGE plpgsql IMMUTABLE STRICT;

-----------------------------------------------
-- 2. SESSION FUNCTION
-----------------------------------------------

CREATE OR REPLACE FUNCTION custom_session_group(ts TIMESTAMPTZ)
RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $$
    -- Convert the TIMESTAMPTZ to the hour it represents in the New York Time Zone.
    SELECT CASE 
        -- LN: London Session (2:00 to 7:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 2 AND 7 THEN 'LN'    
        
        -- NYAM: New York AM (8:00 to 11:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 8 AND 11 THEN 'NYAM' 
        
        -- NYL: New York Lunch (12:00 to 13:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 12 AND 13 THEN 'NYL'  
        
        -- NYPM: New York PM (14:00 to 17:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 14 AND 17 THEN 'NYPM' 
        
        -- AS: Asia Session (Wraps midnight: 6 PM to 1 AM NY Time)
        -- Part 1: End of Day (18:00 to 23:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 18 AND 23 THEN 'AS'     
        
        -- Part 2: Start of Day (00:00 to 01:59 NY Time)
        WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 0 AND 1 THEN 'AS'     
        
        ELSE 'Unknown'
    END;
$$;

SELECT 
    COUNT(ps.trading_date) AS total_attempts_D
FROM us2000_session_views ps -- Prior Session (NYPM)
JOIN us2000_session_views cs -- Current/Breaker Session (AS)
    ON ps.asset_id = cs.asset_id
    -- Match the day transition: NYPM followed by next day AS
    AND ps.session_name = 'NYPM' 
    AND cs.session_name = 'AS' 
    AND ps.trading_date < cs.trading_date -- AS is the next day after NYPM
WHERE 
    ps.session_type = 'Bullish' -- PS Bias condition (NYPM Bullish)
    AND cs.session_type = 'Bullish'; -- BS Bias condition (AS Bullish)

SELECT 
    COUNT(ps.trading_date) AS total_attempts_D_CORRECTED
FROM us2000_session_views ps -- Prior Session (NYPM)
JOIN us2000_session_views cs -- Current/Breaker Session (AS)
    ON ps.asset_id = cs.asset_id
    AND ps.session_name = 'NYPM' 
    AND cs.session_name = 'AS' 
    -- FIX: Ensure AS is EXACTLY one day after NYPM
    AND cs.trading_date = ps.trading_date + INTERVAL '1 day' 
WHERE 
    ps.session_type = 'Bullish' -- PS Bias condition (NYPM Bullish)
    AND cs.session_type = 'Bullish'; -- BS Bias condition (AS Bullish)