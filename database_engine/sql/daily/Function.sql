--------------
-- Function: get_close_type
-----


CREATE OR REPLACE FUNCTION get_close_type(
    open_p DOUBLE PRECISION, 
    high_p DOUBLE PRECISION, 
    low_p DOUBLE PRECISION, 
    close_p DOUBLE PRECISION
)
RETURNS TEXT 
AS $$
DECLARE
    day_range DOUBLE PRECISION := high_p - low_p;
    close_location_ratio DOUBLE PRECISION;
BEGIN
    -- Handle zero range to prevent division by zero (e.g., inside day, perfect consolidation)
    IF day_range = 0 OR day_range IS NULL THEN
        RETURN 'Neutral_Close';
    END IF;

    -- Calculate the ratio of the close price relative to the total day range (0.0 at Low, 1.0 at High)
    close_location_ratio := (close_p - low_p) / day_range;

    -- Bullish Close: Close is in the top 40% of the range (0.60 to 1.0)
    IF close_location_ratio >= 0.60 THEN
        RETURN 'Bullish_Close';
        
    -- Bearish Close: Close is in the bottom 40% of the range (0.0 to 0.40)
    ELSIF close_location_ratio <= 0.40 THEN
        RETURN 'Bearish_Close';
        
    -- Neutral Close: Close is in the middle 20% (0.40 to 0.60)
    ELSE
        RETURN 'Neutral_Close';
    END IF;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

---------------------------------
--- calulate_nyam_bias Function
---------------------------------

CREATE OR REPLACE FUNCTION calculate_nyam_bias(
    target_date DATE, 
    asset_id_text TEXT
)
RETURNS TEXT 
AS $$
DECLARE
    nyam_open DOUBLE PRECISION;
    nyam_high DOUBLE PRECISION;
    nyam_low DOUBLE PRECISION;
    nyam_close DOUBLE PRECISION;
    session_range DOUBLE PRECISION;
    close_location_ratio DOUBLE PRECISION;
    -- Define the session name used in custom_session_group()
    NYAM_SESSION_NAME CONSTANT TEXT := 'NYAM'; 
BEGIN
    -- 1. Look up the NYAM session's OHLC data (Placeholder for actual JOIN/QUERY)
    -- This segment needs to be replaced with your actual query against 'asset_session_views' or 'asset_market_data'
    SELECT
        open, high, low, close
    INTO
        nyam_open, nyam_high, nyam_low, nyam_close
    FROM
        asset_session_views -- Assuming this view exists
    WHERE
        trading_date = target_date AND asset_id = asset_id_text AND session_name = NYAM_SESSION_NAME;

    -- 2. Handle missing data
    IF nyam_open IS NULL THEN
        RETURN 'Neutral';
    END IF;

    -- 3. Calculate Bias based on Conviction (same 40% threshold)
    session_range := nyam_high - nyam_low;

    IF session_range = 0 THEN
        RETURN 'Neutral';
    END IF;
    
    close_location_ratio := (nyam_close - nyam_low) / session_range;
    
    -- Bullish Bias: Close is in the top 40%
    IF close_location_ratio >= 0.60 THEN
        RETURN 'Bullish';
    
    -- Bearish Bias: Close is in the bottom 40%
    ELSIF close_location_ratio <= 0.40 THEN
        RETURN 'Bearish';
    
    -- Neutral Bias: Close is in the middle 20%
    ELSE
        RETURN 'Neutral';
    END IF;
END;
$$ LANGUAGE plpgsql;