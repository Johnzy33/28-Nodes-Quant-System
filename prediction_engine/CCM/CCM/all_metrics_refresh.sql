
-- Step 1: Create the master procedure that coordinates all calculations.
CREATE OR REPLACE PROCEDURE refresh_all_metrics()
LANGUAGE plpgsql
AS $$
BEGIN
    -- 1. Refresh TCS (Transition Confidence Score)
    -- This calculates the session-to-session confidence.
    RAISE NOTICE 'Executing refresh_tcs_metrics...';
    CALL refresh_tcs_metrics();

    -- 2. Refresh PCS (Predictive Confidence Score)
    -- This calculates the pattern-to-Daily-Outcome confidence (now corrected for 2nd order lookbacks).
    RAISE NOTICE 'Executing refresh_pcs_metrics...';
    CALL refresh_pcs_metrics();
    
    -- 3. Refresh CCM (Combined Confidence Metric)
    -- This integrates the PCS prediction with the TCS momentum confirmation.
    RAISE NOTICE 'Executing refresh_ccm_metrics...';
    CALL refresh_ccm_metrics();

    RAISE NOTICE 'All Confidence Metrics (TCS, PCS, and CCM) have been successfully refreshed.';

END;
$$;

-- Step 2: The actual command to run the entire data pipeline
-- You should execute this single line to trigger all calculations.

CALL refresh_all_metrics();