
--------------------
-- PCS 1st - order
----------------

SELECT
    asset_id,
    lookback_period,
    ps_name AS PS1_Session,
    ps_bias_3 AS PS1_Bias_3State,
    cs_name AS CS_Session,
    cs_bias_3 AS CS_Bias_3State,
    daily_outcome_7 AS Daily_Outcome_7State,
    p_day_type_conditional AS Pattern_Success_Rate,
    p_day_type_base AS Outcome_Base_Rate,
    pcs_score AS PCS_Score,
    insight_label AS PCS_Insight
FROM
    predictive_confidence_score
WHERE
    asset_id = 'assets:UK100:FundedNext' -- 🎯 SET YOUR ASSET HERE
   -- AND lookback_period = 'ALL'          -- 🎯 SET LOOKBACK ('6M', '1Y', 'ALL')
    AND lookback_period = '6M'
    AND ps_name = 'LN'                   -- 🎯 SET PREVIOUS SESSION (PS1)
    AND cs_name = 'NYAM'                 -- 🎯 SET CURRENT SESSION (CS)
    AND ps_bias_3 = 'Bullish'
    AND cs_bias_3 = 'Bullish'
ORDER BY
    pcs_score DESC;

----------------------
-- PCS 2nd Order
---------------------

SELECT
    asset_id,
    lookback_period,
    ps2_name AS PS2_Session,
    ps2_bias_7 AS PS2_Bias_7State,
    ps1_name AS PS1_Session,
    ps1_bias_7 AS PS1_Bias_7State,
    cs_name AS CS_Session,
    cs_bias_7 AS cs_bias_7,
    daily_outcome_7 AS Daily_Outcome_7State,
    p_day_type_conditional AS Pattern_Success_Rate,
    p_day_type_base AS Outcome_Base_Rate,
    pcs_score AS PCS_Score,
    insight_label AS PCS_Insight
FROM
    predictive_confidence_score_2nd_order
WHERE
    asset_id = 'assets:UK100:FundedNext'  -- 🎯 SET YOUR ASSET HERE
    AND lookback_period = 'ALL'            -- 🎯 SET LOOKBACK ('6M', '1Y', 'ALL')
    AND ps1_name = 'LN'                   -- 🎯 SET 1ST PREVIOUS SESSION (PS1)
    AND ps2_name = 'AS'                   -- 🎯 SET 2ND PREVIOUS SESSION (PS2)
    AND ps1_bias_7 = 'Bullish' 
    AND ps2_bias_7 = 'Bullish'
ORDER BY
    pcs_score DESC;


----------------------------------
-- TCS
--------------------------------

SELECT
    asset_id,
    lookback_period,
    ps1_name AS PS1_Session,
    ps1_bias AS PS1_Bias,
    cs_name AS CS_Session,
    cs_bias AS CS_Bias,
    tcs_score AS TCS_Score
FROM
    tcs_1st_order
WHERE
    asset_id = 'assets:UK100:FundedNext' -- 🎯 SET YOUR ASSET HERE
    AND lookback_period = 'ALL'          -- 🎯 SET LOOKBACK ('6M', '1Y', 'ALL')
    AND ps1_name = 'LN'                  -- 🎯 SET PREVIOUS SESSION (PS1)
    AND ps1_bias = 'Bullish'             -- 🎯 SET PS1 BIAS (7-state name)
ORDER BY
    tcs_score DESC;

-----------------------------
--- TCS 2nd_order
----------------------------

SELECT
    asset_id,
    lookback_period,
    ps2_name AS PS2_Session,
    ps2_bias AS PS2_Bias,
    ps1_name AS PS1_Session,
    ps1_bias AS PS1_Bias,
    cs_name AS CS_Session,
    cs_bias AS CS_Bias,
    tcs_score AS TCS_Score
FROM
    tcs_2nd_order
WHERE
    asset_id = 'assets:UK100:FundedNext'  -- 🎯 SET YOUR ASSET HERE
    AND lookback_period = '1Y'            -- 🎯 SET LOOKBACK ('6M', '1Y', 'ALL')
    AND ps1_name = 'LN'                   -- 🎯 SET 1ST PREVIOUS SESSION (PS1)
    AND ps2_name = 'AS'                   -- 🎯 SET 2ND PREVIOUS SESSION (PS2)
    AND ps1_bias = 'Bullish'              -- 🎯 SET PS1 BIAS
ORDER BY
    tcs_score DESC;