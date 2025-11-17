
DROP PROCEDURE refresh_core_data();

CREATE OR REPLACE PROCEDURE refresh_core_data()
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE NOTICE '==================================================';
    RAISE NOTICE '          STARTING MASTER CORE DATA REFRESH         ';
    RAISE NOTICE '==================================================';

    -- ###############################################################
    -- # PHASE 1: REFRESH CORE VIEWS (BASE DATA & CONTEXT)
    -- ###############################################################
    RAISE NOTICE '--- PHASE 1: REFRESHING CORE VIEWS (Data Source) ---';

    -- 1. Refresh Session Base Materialized View
    RAISE NOTICE '1.1: Refreshing session_base MV...';
    REFRESH MATERIALIZED VIEW session_base;

    -- 2. Call Session Views Refresh Procedure
    RAISE NOTICE '1.2: Calling refresh_session_views()...';
    CALL refresh_session_views();

    -- 3. Refresh the critical session_context table (relies on session_views)
    RAISE NOTICE '1.3: Calling refresh_session_context()...';
    CALL refresh_session_context();

    -- 4. Refresh Daily Base Materialized View
    RAISE NOTICE '1.4: Refreshing daily_base MV...';
    REFRESH MATERIALIZED VIEW daily_base;

    -- 5. Call Daily Views Refresh Procedure
    RAISE NOTICE '1.5: Calling refresh_daily_views()...';
    
    CALL refresh_daily_views();

    -- 6. Call Weekly Views Refresh Procedure
    RAISE NOTICE '1.6: Calling refresh_weekly_views()...';
    CALL refresh_weekly_views();

    RAISE NOTICE '--- PHASE 1 COMPLETE ---';

    -- ###############################################################
    -- # PHASE 2: REFRESH ALL CORE METRICS METRICS (TCS, PCS, TCS_Cont, FCI)
    -- ###############################################################
    RAISE NOTICE '--- PHASE 2: CALCULATING CONFIDENCE METRICS (TCS/PCS/FCI) ---';

    -- 7. Refresh TCS (Transition Confidence Score) - Relies on session_context
    RAISE NOTICE '2.1: Executing refresh_metrics_pipeline...';
    CALL refresh_metrics_pipeline();


    RAISE NOTICE '--- PHASE 2 COMPLETE ---';

    -- ###############################################################
    -- # PHASE 3: REFRESH ALL DCS METRICS (DAILY SCORES)
    -- ###############################################################
    -- RAISE NOTICE '--- PHASE 3: CALCULATING DAY COUNT SCORES (DCS) ---';

    -- -- 10. Update/Calculate historical data used as the base for all daily metrics.
    -- RAISE NOTICE '3.1: Calling update_dbs_history()...';
    -- CALL update_dbs_history();

    -- -- 11. Refresh the daily metrics snapshot using the updated history.
    -- RAISE NOTICE '3.2: Calling refresh_daily_metrics_snapshot()...';
    -- CALL refresh_daily_metrics_snapshot();

    -- -- 12. Calculate the final daily composite score using the refreshed snapshot.
    -- RAISE NOTICE '3.3: Calling refresh_daily_composite_score()...';
    -- CALL refresh_daily_composite_score();

    RAISE NOTICE '--- PHASE 3 COMPLETE ---';
    
    RAISE NOTICE '==================================================';
    RAISE NOTICE '     MASTER CORE DATA REFRESH SUCCESSFULLY COMPLETED!    ';
    RAISE NOTICE '==================================================';

END;
$$;

CALL refresh_core_data();
