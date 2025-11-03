
WITH Model_Signals AS (
    -- 1. Standard Signal Processing (Unchanged)
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score
    FROM asset_daily_composite_score
    WHERE dcs_classification IN ('Medium Conviction Long', 'High Conviction Long', 
                                  'Medium Conviction Short', 'High Conviction Short')
),
Actual_Outcomes AS (
    -- 2. Standard Next-Day Outcome Calculation (Unchanged)
    SELECT
        trading_date,
        asset_id,
        (close - open) / open AS daily_return,
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
    FROM asset_daily_views
),
Signal_Performance AS (
    -- 3. Calculate Daily P&L and Win/Loss Status (Unchanged)
    SELECT
        S.trading_date AS signal_date,
        S.asset_id,
        -- P&L for a hypothetical trade
        CASE
            WHEN S.final_dcs_score > 0 THEN O.daily_return
            WHEN S.final_dcs_score < 0 THEN -O.daily_return
            ELSE 0 
        END AS trade_pnl,
        -- 1 if Win, 0 if Loss (or break-even)
        CASE
            WHEN S.final_dcs_score > 0 AND O.daily_return > 0 THEN 1 
            WHEN S.final_dcs_score < 0 AND O.daily_return < 0 THEN 1 
            ELSE 0 
        END AS is_win
        
    FROM Model_Signals S
    INNER JOIN Actual_Outcomes O 
        ON S.trading_date = O.signal_date_to_join 
        AND S.asset_id = O.asset_id
),
Streak_Detection AS (
    -- 4. STEP 1 OF FIX: Detect the start of a new streak using LAG in its own CTE
    SELECT
        *,
        -- Use LAG to get the previous win/loss status
        LAG(is_win, 1, -1) OVER (PARTITION BY asset_id ORDER BY signal_date) AS prev_is_win,
        -- Flag where the streak starts (is_win is different from previous)
        CASE 
            WHEN is_win != LAG(is_win, 1, -1) OVER (PARTITION BY asset_id ORDER BY signal_date) 
            THEN 1 
            ELSE 0 
        END AS is_new_streak_start
    FROM Signal_Performance
),
Streak_Analysis AS (
    -- 5. STEP 2 OF FIX: Create a unique Group ID by summing the start flags
    SELECT
        *,
        -- SUM is NOT a window function here; it's an aggregate over the window,
        -- using the result of a previous window function (LAG)
        SUM(is_new_streak_start) OVER (PARTITION BY asset_id ORDER BY signal_date) AS streak_group_id
    FROM Streak_Detection
),
Run_Length_Calculation AS (
    -- 6. Calculate the length of each winning and losing run (Unchanged)
    SELECT
        asset_id,
        is_win,
        streak_group_id,
        COUNT(*) AS streak_length
    FROM Streak_Analysis
    GROUP BY 1, 2, 3
)
-- 7. Final Aggregation (Unchanged logic, referencing the correct CTEs)
SELECT
    SP.asset_id,
    -- PNL Extremes
    ROUND(MAX(SP.trade_pnl)::NUMERIC, 4) AS largest_win,
    ROUND(ABS(MIN(SP.trade_pnl))::NUMERIC, 4) AS largest_loss,
    -- Streak Extremes
    MAX(CASE WHEN R.is_win = 1 THEN R.streak_length ELSE 0 END) AS longest_winning_run,
    MAX(CASE WHEN R.is_win = 0 THEN R.streak_length ELSE 0 END) AS longest_losing_run
    
FROM Signal_Performance SP
INNER JOIN Run_Length_Calculation R 
    ON SP.asset_id = R.asset_id -- Join is not strictly necessary here, but keeping it for clarity in this complex structure.
GROUP BY 1;

----------------------------------------
-- win/loss conviction test
-----------------------------------

WITH Model_Signals AS (
    -- 1. Standard Signal Processing
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score
    FROM asset_daily_composite_score
    -- Filter here to get ONLY the Low Conviction range: (0.25 < |DCS| <= 0.35)
    WHERE ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) > 0.25
      AND ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) <= 0.35
),
Actual_Outcomes AS (
    -- 2. Standard Next-Day Outcome Calculation
    SELECT
        trading_date,
        asset_id,
        (close - open) / open AS daily_return,
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
    FROM asset_daily_views
),
Signal_Performance AS (
    -- 3. Calculate Win/Loss Status for only the filtered signals
    SELECT
        S.asset_id,
        -- 1 if Win, 0 if Loss (or break-even)
        CASE
            -- Win: Long Signal (DCS > 0) AND Positive Return (daily_return > 0)
            WHEN S.final_dcs_score > 0 AND O.daily_return > 0 THEN 1 
            -- Win: Short Signal (DCS < 0) AND Negative Return (daily_return < 0)
            WHEN S.final_dcs_score < 0 AND O.daily_return < 0 THEN 1 
            ELSE 0 
        END AS is_win
        
    FROM Model_Signals S
    INNER JOIN Actual_Outcomes O 
        ON S.trading_date = O.signal_date_to_join 
        AND S.asset_id = O.asset_id
)
-- 4. Final Aggregation: Calculate Win Rate for this isolated group
SELECT
    asset_id,
    COUNT(*) AS total_weak_signals,
    SUM(is_win) AS total_wins_weak,
    -- Calculate Win Rate and handle division by zero with NULLIF
    ROUND(
        (SUM(is_win)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 
        2
    ) AS weak_signal_win_rate_percent
FROM Signal_Performance
GROUP BY asset_id;

----------------------------------------------
-- worstt case 
---------------------------------------------

WITH Model_Signals AS (
    -- 1. Standard Signal Processing
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score
    FROM asset_daily_composite_score
    WHERE ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) > 0.25 -- Ensure only actionable signals are included
),
Actual_Outcomes AS (
    -- 2. Standard Next-Day Outcome Calculation
    SELECT
        trading_date,
        asset_id,
        (close - open) / open AS daily_return,
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
    FROM asset_daily_views
),
Signal_Performance AS (
    -- 3. Calculate Trade P&L (Always positive for a win, negative for a loss)
    SELECT
        S.trading_date AS signal_date,
        S.asset_id,
        CASE
            WHEN S.final_dcs_score > 0 THEN O.daily_return  -- Long Trade P&L
            WHEN S.final_dcs_score < 0 THEN -O.daily_return -- Short Trade P&L
            ELSE 0 
        END AS trade_pnl
    FROM Model_Signals S
    INNER JOIN Actual_Outcomes O 
        ON S.trading_date = O.signal_date_to_join 
        AND S.asset_id = O.asset_id
)
-- 4. Final Aggregation: Find the worst cumulative P&L in any 10-trade block
SELECT
    asset_id,
    signal_date AS end_date_of_worst_block,
    -- Calculate the rolling P&L over the last 10 trades (including the current one)
    ROUND(
        SUM(trade_pnl) OVER (
            PARTITION BY asset_id 
            ORDER BY signal_date 
            ROWS BETWEEN 9 PRECEDING AND CURRENT ROW -- Defines the 10-trade window
        )::NUMERIC, 
        4
    ) AS rolling_10_day_pnl
    
FROM Signal_Performance
-- Find the minimum (worst) P&L value across all calculated 10-day blocks
ORDER BY rolling_10_day_pnl ASC
LIMIT 1;
-------------------
--- close to close
---------------------

WITH Model_Signals AS (
    -- 1. Standard Signal Processing - Filter for Low Conviction Signals (0.25 < |DCS| <= 0.35)
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score
    FROM asset_daily_composite_score
    WHERE ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) > 0.25
      AND ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) <= 0.35
),
Actual_Outcomes AS (
    -- 2. NEW: Calculate Close-to-Close Return for the Outcome Day
    SELECT
        trading_date,
        asset_id,
        -- Get the previous day's close (C0)
        LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC) AS previous_close,
        close AS current_close,
        -- Calculate C/C Daily Return: (C1 - C0) / C0
        (close - LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC)) / 
        LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC) AS daily_return,
        -- The Signal Date is still the day *before* the return, adjusted for the join
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
        
    FROM asset_daily_views
),
Signal_Performance AS (
    -- 3. Calculate Win/Loss Status using the C/C return
    SELECT
        S.asset_id,
        -- 1 if Win, 0 if Loss (or break-even)
        CASE
            -- Win: Long Signal (DCS > 0) AND Positive C/C Return (daily_return > 0)
            WHEN S.final_dcs_score > 0 AND O.daily_return > 0 THEN 1 
            -- Win: Short Signal (DCS < 0) AND Negative C/C Return (daily_return < 0)
            WHEN S.final_dcs_score < 0 AND O.daily_return < 0 THEN 1 
            ELSE 0 
        END AS is_win
        
    FROM Model_Signals S
    INNER JOIN Actual_Outcomes O 
        ON S.trading_date = O.signal_date_to_join 
        AND S.asset_id = O.asset_id
)
-- 4. Final Aggregation
SELECT
    asset_id,
    COUNT(*) AS total_weak_signals,
    SUM(is_win) AS total_wins_weak,
    -- Calculate Win Rate and handle division by zero
    ROUND(
        (SUM(is_win)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 
        2
    ) AS weak_signal_win_rate_percent
FROM Signal_Performance
GROUP BY asset_id;

-------------------------
-- low convition with hudel
-----------------------------

WITH Model_Signals AS (
    -- 1. Standard Signal Processing - Filter for Low Conviction Signals (0.25 < |DCS| <= 0.35)
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score
    FROM asset_daily_composite_score
    WHERE ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) > 0.25
      AND ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) <= 0.35
),
Actual_Outcomes AS (
    -- 2. Calculate Close-to-Close Return (C/C)
    SELECT
        trading_date,
        asset_id,
        (close - LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC)) / 
        LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC) AS daily_return,
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
    FROM asset_daily_views
),
Signal_Performance AS (
    -- 3. APPLY COST HURDLE: Define a Win only if gross profit > 0.0005
    SELECT
        S.asset_id,
        -- 1 if Win, 0 if Loss (or break-even)
        CASE
            -- Condition 1: Long Signal (DCS > 0)
            WHEN S.final_dcs_score > 0 
                AND O.daily_return > 0.0005 -- Win if C/C return is greater than 0.05% cost
            THEN 1 
            -- Condition 2: Short Signal (DCS < 0)
            WHEN S.final_dcs_score < 0 
                AND O.daily_return < -0.0005 -- Win if C/C return is worse than -0.05% (i.e., profitable after costs)
            THEN 1 
            -- Loss: Any outcome that fails to clear the 0.05% profit hurdle
            ELSE 0 
        END AS is_win
    FROM Model_Signals S
    INNER JOIN Actual_Outcomes O 
        ON S.trading_date = O.signal_date_to_join 
        AND S.asset_id = O.asset_id
)
-- 4. Final Aggregation
SELECT
    asset_id,
    COUNT(*) AS total_weak_signals,
    SUM(is_win) AS total_wins_weak,
    ROUND(
        (SUM(is_win)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 
        2
    ) AS weak_signal_win_rate_percent
FROM Signal_Performance
GROUP BY asset_id;

--------------------------
-- see for yourself
-----------------------

WITH Model_Signals AS (
    -- 1. Select all actionable signals (ABS(DCS) > 0.25)
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score
    FROM asset_daily_composite_score
    WHERE ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) > 0.25
),
Actual_Outcomes AS (
    -- 2. Calculate Close-to-Close Return (C/C)
    SELECT
        trading_date,
        asset_id,
        -- Calculate C/C Daily Return: (C1 - C0) / C0
        (close - LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC)) / 
        LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC) AS daily_return,
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
    FROM asset_daily_views
),
Signal_Audit AS (
    -- 3. Join Signals and Outcomes, and classify the result
    SELECT
        S.trading_date AS signal_date,
        S.asset_id,
        S.final_dcs_score,
        O.daily_return AS outcome_return,
        -- Determine Trade Direction
        CASE 
            WHEN S.final_dcs_score > 0 THEN 'Long' 
            WHEN S.final_dcs_score < 0 THEN 'Short' 
            ELSE 'Neutral'
        END AS trade_direction,
        -- Determine Win/Loss Status (C/C definition)
        CASE
            WHEN S.final_dcs_score > 0 AND O.daily_return > 0 THEN 'Win'       -- Long Win
            WHEN S.final_dcs_score < 0 AND O.daily_return < 0 THEN 'Win'       -- Short Win
            WHEN S.final_dcs_score > 0 AND O.daily_return <= 0 THEN 'Loss'     -- Long Loss
            WHEN S.final_dcs_score < 0 AND O.daily_return >= 0 THEN 'Loss'     -- Short Loss
            ELSE 'Unclassified'
        END AS trade_outcome
        
    FROM Model_Signals S
    INNER JOIN Actual_Outcomes O 
        ON S.trading_date = O.signal_date_to_join 
        AND S.asset_id = O.asset_id
)
-- 4. Final Aggregation and Detailed Reporting
SELECT
    asset_id,
    -- Total Trade Summary
    COUNT(*) AS total_trades_taken,
    SUM(CASE WHEN trade_direction = 'Long' THEN 1 ELSE 0 END) AS total_long_trades,
    SUM(CASE WHEN trade_direction = 'Short' THEN 1 ELSE 0 END) AS total_short_trades,
    -- Win/Loss Breakdown
    SUM(CASE WHEN trade_direction = 'Long' AND trade_outcome = 'Win' THEN 1 ELSE 0 END) AS long_trade_wins,
    SUM(CASE WHEN trade_direction = 'Short' AND trade_outcome = 'Loss' THEN 1 ELSE 0 END) AS short_trade_losses,
    -- Dates of Interest (Aggregated as a list/array)
    STRING_AGG(CASE WHEN trade_direction = 'Long' AND trade_outcome = 'Win' THEN signal_date::TEXT END, ', ' ORDER BY signal_date) AS dates_of_long_wins,
    STRING_AGG(CASE WHEN trade_direction = 'Short' AND trade_outcome = 'Loss' THEN signal_date::TEXT END, ', ' ORDER BY signal_date) AS dates_of_short_losses

FROM Signal_Audit
GROUP BY asset_id;

------------------------
-- table 
----------------------


CREATE TABLE US2000_Trade_Audit (
    signal_date DATE,
    trade_direction TEXT,
    trade_outcome TEXT,
    trade_pnl DOUBLE PRECISION
);
TRUNCATE TABLE us2000_trade_audit;
INSERT INTO US2000_Trade_Audit (
    signal_date, 
    trade_direction, 
    trade_outcome, 
    trade_pnl
)
WITH Model_Signals AS (
    -- 1. Select ALL actionable signals (ABS(DCS) > 0.25)
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score
    FROM asset_daily_composite_score
    WHERE ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) > 0.60
    AND EXTRACT(YEAR FROM trading_date) = 2025
),
Actual_Outcomes AS (
    -- 2. Calculate Close-to-Close Return (C/C)
    SELECT
        trading_date,
        asset_id,
        -- Calculate C/C Daily Return: (C1 - C0) / C0
        (close - LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC)) / 
        LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC) AS daily_return,
        -- Signal Date for joining
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
    FROM asset_daily_views
),
Signal_Audit AS (
    -- 3. Join Signals and Outcomes, and classify the result
    SELECT
        S.trading_date AS signal_date,
        S.asset_id,
        S.final_dcs_score,
        O.daily_return AS outcome_return,
        -- Determine Trade Direction
        CASE 
            WHEN S.final_dcs_score > 0 THEN 'Long' 
            WHEN S.final_dcs_score < 0 THEN 'Short' 
            ELSE 'Neutral'
        END AS trade_direction,
        -- Determine Win/Loss Status (C/C definition)
        CASE
            WHEN S.final_dcs_score > 0 AND O.daily_return > 0 THEN 'Win'       -- Long Win
            WHEN S.final_dcs_score < 0 AND O.daily_return < 0 THEN 'Win'       -- Short Win
            WHEN S.final_dcs_score > 0 AND O.daily_return <= 0 THEN 'Loss'     -- Long Loss
            WHEN S.final_dcs_score < 0 AND O.daily_return >= 0 THEN 'Loss'     -- Short Loss
            ELSE 'Unclassified'
        END AS trade_outcome
        
    FROM Model_Signals S
    INNER JOIN Actual_Outcomes O 
        ON S.trading_date = O.signal_date_to_join 
        AND S.asset_id = O.asset_id
)
SELECT
    A.signal_date,
    A.trade_direction,
    A.trade_outcome,
    -- Calculate trade_pnl: Always positive for a win, negative for a loss.
    ROUND(
        CASE
            WHEN A.trade_direction = 'Long' THEN A.outcome_return
            WHEN A.trade_direction = 'Short' THEN -A.outcome_return
            ELSE 0 
        END::NUMERIC, 
        6
    ) AS trade_pnl
FROM Signal_Audit A;


--------------------
-- trades per year
------------------

SELECT
    -- Extract the year from the signal date
    EXTRACT(YEAR FROM signal_date) AS trade_year,
    -- Total Trades
    COUNT(*) AS total_annual_trades,
    -- Win/Loss Breakdown
    SUM(CASE WHEN trade_outcome = 'Win' THEN 1 ELSE 0 END) AS total_annual_wins,
    SUM(CASE WHEN trade_outcome = 'Loss' THEN 1 ELSE 0 END) AS total_annual_losses,
    -- Annual Win Rate
    ROUND(
        (SUM(CASE WHEN trade_outcome = 'Win' THEN 1 ELSE 0 END)::NUMERIC / 
         NULLIF(COUNT(*), 0) * 100)::NUMERIC, 
        2
    ) AS annual_win_rate_percent
FROM 
    US2000_Trade_Audit
GROUP BY 
    trade_year
ORDER BY 
    trade_year ASC;
    ---------------------
    hudrl
    ------------------

WITH Model_Signals AS (
    -- 1. Select Low Conviction Signals (0.25 < |DCS| <= 0.35)
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score
    FROM asset_daily_composite_score
    WHERE ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) > 0.25
      AND ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) <= 0.35
      -- *** NEW FILTER: ONLY INCLUDE 2025 SIGNALS ***
      AND EXTRACT(YEAR FROM trading_date) = 2025
),
Actual_Outcomes AS (
    -- 2. Calculate Close-to-Close Return (C/C)
    SELECT
        trading_date,
        asset_id,
        (close - LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC)) / 
        LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC) AS daily_return,
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
    FROM asset_daily_views
),
Signal_Performance AS (
    -- 3. APPLY 0.0005 (0.05%) COST HURDLE
    SELECT
        S.asset_id,
        -- 1 if Win (Profit > 0.0005), 0 otherwise
        CASE
            -- Condition 1: Long Signal (DCS > 0)
            WHEN S.final_dcs_score > 0 
                AND O.daily_return > 0.0005 -- Win if Gross Profit > 0.0005
            THEN 1 
            -- Condition 2: Short Signal (DCS < 0)
            WHEN S.final_dcs_score < 0 
                AND O.daily_return < -0.0005 -- Win if Gross Loss is < -0.0005 (i.e., profitable after cost)
            THEN 1 
            -- Loss: Any outcome that fails to clear the 0.05% profit hurdle
            ELSE 0 
        END AS is_win
        
    FROM Model_Signals S
    INNER JOIN Actual_Outcomes O 
        ON S.trading_date = O.signal_date_to_join 
        AND S.asset_id = O.asset_id
)
-- 4. Final Aggregation
SELECT
    asset_id,
    COUNT(*) AS total_weak_signals_2025,
    SUM(is_win) AS total_wins_after_cost_2025,
    ROUND(
        (SUM(is_win)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 
        2
    ) AS weak_signal_win_rate_after_cost_percent_2025
FROM Signal_Performance
GROUP BY asset_id;

-------------------
-- high cost
----------------

WITH Model_Signals AS (
    -- 1. Select Low Conviction Signals (0.25 < |DCS| <= 0.35)
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score
    FROM asset_daily_composite_score
    WHERE ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) > 0.25
      AND ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) <= 0.35
      -- FILTER: ONLY 2025 SIGNALS
      AND EXTRACT(YEAR FROM trading_date) = 2025
),
Actual_Outcomes AS (
    -- 2. Calculate Close-to-Close Return (C/C)
    SELECT
        trading_date,
        asset_id,
        (close - LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC)) / 
        LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC) AS daily_return,
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
    FROM asset_daily_views
),
Signal_Performance AS (
    -- 3. APPLY 0.0050 (0.50%) COST HURDLE
    SELECT
        S.asset_id,
        -- 1 if Win (Profit > 0.0050), 0 otherwise
        CASE
            -- Condition 1: Long Signal (DCS > 0)
            WHEN S.final_dcs_score > 0 
                AND O.daily_return > 0.0050 -- Win if Gross Profit > 0.50%
            THEN 1 
            -- Condition 2: Short Signal (DCS < 0)
            WHEN S.final_dcs_score < 0 
                AND O.daily_return < -0.0050 -- Win if Gross Loss is < -0.0050 (i.e., profitable after cost)
            THEN 1 
            -- Loss: Any outcome that fails to clear the 0.50% profit hurdle
            ELSE 0 
        END AS is_win
        
    FROM Model_Signals S
    INNER JOIN Actual_Outcomes O 
        ON S.trading_date = O.signal_date_to_join 
        AND S.asset_id = O.asset_id
)
-- 4. Final Aggregation
SELECT
    asset_id,
    COUNT(*) AS total_weak_signals_2025,
    SUM(is_win) AS total_wins_after_cost_2025,
    ROUND(
        (SUM(is_win)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 
        2
    ) AS weak_signal_win_rate_after_cost_percent_2025
FROM Signal_Performance
GROUP BY asset_id;

----------------------------
-- above 50%
--------------------------

WITH Model_Signals AS (
    -- 1. Select ONLY High Conviction Signals (|DCS| > 0.60)
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score
    FROM asset_daily_composite_score
    WHERE ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) > 0.60
    AND EXTRACT(YEAR FROM trading_date) = 2025
),
Actual_Outcomes AS (
    -- 2. Calculate Close-to-Close Return (C/C)
    SELECT
        trading_date,
        asset_id,
        -- Calculate C/C Daily Return: (C1 - C0) / C0
        (close - LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC)) / 
        LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC) AS daily_return,
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
    FROM asset_daily_views
),
Signal_Performance AS (
    -- 3. Calculate Win/Loss Status (C/C return used)
    SELECT
        S.asset_id,
        -- 1 if Win, 0 if Loss (or break-even)
        CASE
            -- Win: Long Signal (DCS > 0) AND Positive C/C Return (daily_return > 0)
            WHEN S.final_dcs_score > 0 AND O.daily_return > 0 THEN 1 
            -- Win: Short Signal (DCS < 0) AND Negative C/C Return (daily_return < 0)
            WHEN S.final_dcs_score < 0 AND O.daily_return < 0 THEN 1 
            ELSE 0 
        END AS is_win
        
    FROM Model_Signals S
    INNER JOIN Actual_Outcomes O 
        ON S.trading_date = O.signal_date_to_join 
        AND S.asset_id = O.asset_id
)
-- 4. Final Aggregation
SELECT
    asset_id,
    COUNT(*) AS total_high_conviction_trades,
    SUM(is_win) AS total_high_conviction_wins,
    ROUND(
        (SUM(is_win)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 
        2
    ) AS high_conviction_win_rate_percent
FROM Signal_Performance
GROUP BY asset_id;

--------------------------------
-- pullback
----------------------------

WITH Model_Signals AS (
    -- 1. Select ONLY High Conviction Signals (|DCS| > 0.60)
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score
    FROM asset_daily_composite_score
    WHERE ABS(REPLACE(dcs, '%', '')::NUMERIC / 100.0) > 0.60
),
Actual_Day_Data AS (
    -- 2. Get Open, High, Low, and Outcome Data for the next day
    SELECT
        trading_date,
        asset_id,
        open, high, low,
        (close - LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC)) / 
        LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC) AS daily_cc_return,
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
    FROM asset_daily_views
),
Pullback_Analysis AS (
    -- 3. Check for 0.15% Pullback Opportunity on Winning Trades
    SELECT
        S.asset_id,
        S.trading_date AS signal_date,
        T.open, T.high, T.low,
        T.daily_cc_return,
        -- Trade is a High Conviction Win (C/C definition)
        CASE
            WHEN S.final_dcs_score > 0 AND T.daily_cc_return > 0 THEN 'Long Win'
            WHEN S.final_dcs_score < 0 AND T.daily_cc_return < 0 THEN 'Short Win'
            ELSE 'Loss/Neutral'
        END AS trade_result,
        -- Identify the Pullback Opportunity (0.15% threshold = 0.0015)
        CASE
            -- LONG Win: Did price drop from Open to Low by 0.15% or more?
            WHEN S.final_dcs_score > 0 
                 AND T.daily_cc_return > 0
                 AND (T.open - T.low) / T.open >= 0.0015
            THEN 1
            -- SHORT Win: Did price rally from Open to High by 0.15% or more?
            WHEN S.final_dcs_score < 0 
                 AND T.daily_cc_return < 0
                 AND (T.high - T.open) / T.open >= 0.0015
            THEN 1
            
            ELSE 0
        END AS has_pullback_entry
        
    FROM Model_Signals S
    INNER JOIN Actual_Day_Data T 
        ON S.trading_date = T.signal_date_to_join 
        AND S.asset_id = T.asset_id
)
-- 4. Final Aggregation: Count total wins and those with a pullback opportunity
SELECT
    asset_id,
    COUNT(CASE WHEN trade_result IN ('Long Win', 'Short Win') THEN 1 END) AS total_high_conviction_wins,
    SUM(has_pullback_entry) AS wins_with_pullback_opportunity,
    ROUND(
        (SUM(has_pullback_entry)::NUMERIC / 
         NULLIF(COUNT(CASE WHEN trade_result IN ('Long Win', 'Short Win') THEN 1 END), 0) * 100)::NUMERIC, 
        2
    ) AS pullback_success_rate_percent
FROM Pullback_Analysis
GROUP BY asset_id;

--------------------------------
-- short 2025
------------------

WITH Model_Signals AS (
    -- 1. Select ONLY High Conviction Short Signals (DCS < -0.60)
    SELECT
        trading_date,
        asset_id,
        (REPLACE(dcs, '%', '')::NUMERIC / 100.0) AS final_dcs_score -- Define the alias here
    FROM asset_daily_composite_score
    -- FIX: Use the full expression for filtering, as the alias is not yet available in the WHERE clause
    WHERE (REPLACE(dcs, '%', '')::NUMERIC / 100.0) < -0.60 
),
Actual_Outcomes AS (
    -- 2. Calculate Close-to-Close Return (C/C)
    SELECT
        trading_date,
        asset_id,
        (close - LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC)) / 
        LAG(close, 1) OVER (PARTITION BY asset_id ORDER BY trading_date ASC) AS daily_return,
        LAG(trading_date, 1) OVER (PARTITION BY asset_id ORDER BY trading_date DESC) AS signal_date_to_join 
    FROM asset_daily_views
),
Signal_Performance AS (
    -- 3. Join and Calculate Win/Loss Status (C/C return used)
    SELECT
        S.asset_id,
        S.trading_date AS signal_date,
        S.final_dcs_score, -- This now safely references the column from Model_Signals
        O.daily_return,
        -- 1 if Win, 0 if Loss
        CASE
            WHEN S.final_dcs_score < 0 AND O.daily_return < 0 THEN 1 -- Short Win: Price goes down
            ELSE 0 
        END AS is_win
        
    FROM Model_Signals S
    INNER JOIN Actual_Outcomes O 
        ON S.trading_date = O.signal_date_to_join 
        AND S.asset_id = O.asset_id
)
-- 4. Final Aggregation and Annual Breakdown
SELECT
    EXTRACT(YEAR FROM signal_date) AS trade_year,
    COUNT(*) AS total_high_conviction_short_trades,
    SUM(is_win) AS total_short_wins,
    ROUND(
        (SUM(is_win)::NUMERIC / NULLIF(COUNT(*), 0) * 100)::NUMERIC, 
        2
    ) AS short_win_rate_percent
FROM Signal_Performance
GROUP BY trade_year
ORDER BY trade_year ASC;