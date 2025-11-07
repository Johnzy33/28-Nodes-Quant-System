CREATE OR REPLACE PROCEDURE refresh_all_dcs_metrics()
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE NOTICE 'Starting ALL DCS Metrics Pipeline...';

    -- 1. Update/Calculate historical data used as the base for all daily metrics.
    RAISE NOTICE 'Step 1/3: Calling update_dbs_history()...';
    CALL update_dbs_history();
    RAISE NOTICE '   ...update_dbs_history() complete.';

    -- 2. Refresh the daily metrics snapshot using the updated history.
    RAISE NOTICE 'Step 2/3: Calling refresh_daily_metrics_snapshot()...';
    CALL refresh_daily_metrics_snapshot();
    RAISE NOTICE '   ...refresh_daily_metrics_snapshot() complete.';

    -- 3. Calculate the final daily composite score using the refreshed snapshot.
    RAISE NOTICE 'Step 3/3: Calling refresh_daily_composite_score()...';
    CALL refresh_daily_composite_score();
    RAISE NOTICE '   ...refresh_daily_composite_score() complete.';

    RAISE NOTICE 'ALL DCS Metrics Pipeline complete.';

END;
$$;

