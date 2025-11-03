

CREATE TABLE IF NOT EXISTS asset_daily_volatility (
    trading_date DATE NOT NULL,
    asset_id TEXT NOT NULL,
    volatility_scale_factor NUMERIC(10, 4) NOT NULL, 
    PRIMARY KEY (trading_date, asset_id)
);

INSERT INTO asset_daily_volatility (
    trading_date, asset_id, volatility_scale_factor
)
WITH True_Range AS (
    -- 1. Calculate the True Range (TR) for each day
    SELECT
        trading_date,
        asset_id,
        "close" AS current_close,
        LAG("close", 1) OVER (PARTITION BY asset_id ORDER BY trading_date) AS prior_close,
        "high" AS current_high,
        "low" AS current_low,
        -- True Range (TR) calculation
        GREATEST(
            ("high" - "low"),
            ABS("high" - LAG("close", 1) OVER (PARTITION BY asset_id ORDER BY trading_date)),
            ABS("low" - LAG("close", 1) OVER (PARTITION BY asset_id ORDER BY trading_date))
        ) AS TR
    FROM asset_daily_views
),
ATR_Calculation AS (
    -- 2. Calculate the 10-Day Average True Range (ATR)
    SELECT
        trading_date,
        asset_id,
        TR,
        -- 10-Day Exponentially Smoothed Moving Average of TR (Approximation of standard ATR)
        AVG(TR) OVER (
            PARTITION BY asset_id 
            ORDER BY trading_date 
            ROWS BETWEEN 9 PRECEDING AND CURRENT ROW
        ) AS ATR_10_Day
    FROM True_Range
    WHERE TR IS NOT NULL
),
Volatility_Scaling AS (
    -- 3. Calculate the 30-Day Rolling Average ATR and the final Volatility Scale Factor (V_R_F)
    SELECT
        trading_date,
        asset_id,
        ATR_10_Day,
        -- 30-Day Rolling Mean of the 10-Day ATR
        AVG(ATR_10_Day) OVER (
            PARTITION BY asset_id 
            ORDER BY trading_date 
            ROWS BETWEEN 29 PRECEDING AND CURRENT ROW
        ) AS ATR_30_Day_Avg
        
    FROM ATR_Calculation
),
V_R_F_Final AS (
    -- 4. Apply the scaling and min/max limits
    SELECT
        trading_date,
        asset_id,
        ATR_10_Day,
        ATR_30_Day_Avg,
        -- Calculate the raw scale factor (Ratio of historical volatility to current volatility)
        (ATR_30_Day_Avg / NULLIF(ATR_10_Day, 0)) AS Raw_Scale_Factor
    FROM Volatility_Scaling
    WHERE ATR_30_Day_Avg IS NOT NULL AND ATR_10_Day IS NOT NULL
)
SELECT
    trading_date,
    asset_id,
    -- Apply the final MIN/MAX bounds (0.5 to 1.5) and round
    ROUND(
        LEAST(1.5, GREATEST(0.5, Raw_Scale_Factor))::NUMERIC, 
        4
    ) AS volatility_scale_factor
FROM V_R_F_Final
ON CONFLICT (trading_date, asset_id) 
DO UPDATE SET
    volatility_scale_factor = EXCLUDED.volatility_scale_factor;