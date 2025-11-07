-- -----------------------------------------------------------
-- VIEW: latest_session_view (formerly latest_asset_pattern_v)
-- Provides the most recent completed 2nd-order pattern keys for each asset.
-- -----------------------------------------------------------

CALL re
CREATE OR REPLACE VIEW latest_session_view AS
WITH RankedContext AS (
    SELECT
        asset_id,
        -- Keys for 2nd-order (Bias 7) and 1st-order (Bias 3) lookups
        ps2_name,
        ps2_bias_7,
        ps1_name,
        ps1_bias_7,
        ps1_bias_3, -- FIXED: This was missing and caused the error
        cs_name,
        cs_bias_7,
        cs_bias_3,  -- FIXED: This was also needed for 1st order lookups
        
        trading_date,
        session_end_ts,
        -- Ranking by the most recent session_end_ts per asset
        ROW_NUMBER() OVER (
            PARTITION BY asset_id 
            ORDER BY session_end_ts DESC
        ) as rn
    FROM
        session_context
    -- Only consider patterns that are fully formed (2nd order pattern requires PS2)
    WHERE ps2_name IS NOT NULL
)
SELECT
    asset_id,
    -- 2nd Order Pattern Keys
    ps2_name,
    ps2_bias_7,
    ps1_name,
    ps1_bias_7,
    cs_name,
    cs_bias_7,
    trading_date,
    session_end_ts,
    -- 1st Order Pattern Keys (renamed for clarity in the Rust app)
    ps1_name AS latest_ps_name_1st,
    ps1_bias_3 AS latest_ps_bias_3_1st, -- FIXED: Now available
    cs_name AS latest_cs_name_1st,
    cs_bias_3 AS latest_cs_bias_3_1st    -- FIXED: Now available
FROM
    RankedContext
WHERE
    rn = 1; -- Select only the latest record for each asset