
-------------------------------
--- referesh views
---------------------------

-- Assuming you have the pg_cron extension installed and enabled.

-- Drop any previous schedule to avoid duplicates
SELECT cron.unschedule('daily-views-full-refresh');

-- Create a single scheduled job to refresh all three materialized objects:
-- Runs every day at 23:00 UTC (shortly after the NY close).
SELECT cron.schedule(
    'daily-views-full-refresh',              -- Job name
    '0 23 * * *',                            -- Schedule: Every day at 23:00 UTC
    $BODY$
    -- 1. Refresh Layer 1 (Session & Daily Base from raw market_data)
    REFRESH MATERIALIZED VIEW session_base_mv;
    REFRESH MATERIALIZED VIEW daily_base_mv;

    -- 2. Refresh Layer 2 (Daily & Session Final Tables)
    CALL refresh_daily_views();  -- Must run before weekly (lag dependency)
    CALL refresh_session_views();

    -- 3. Refresh Layer 3 (Weekly Final Table)
    CALL refresh_weekly_views(); 
    $BODY$
);