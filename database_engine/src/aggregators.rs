use anyhow::{Context, Ok, Result};
use surrealdb::{RecordId};
use crate::DB;


pub async fn define_aggregation_views(db: &DB, asset_id: &RecordId) -> Result<()> {
    println!(" - Defining aggregation views...");

    // Session aggregation view
    let session_view = r#"
    DEFINE TABLE session_agg AS
    SELECT 
        asset_id,
        time::format(time::floor(ts, '1d'), "%a") AS date,
        fn::session_from_timestamp(ts) AS session,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume,
        fn::candle_pattern(
            (SELECT VALUE open FROM market_data WHERE asset_id = THIS.asset_id AND fn::session_from_timestamp(ts) = THIS.session ORDER BY ts ASC LIMIT 1),
            math::max(high),
            math::min(low),
            (SELECT VALUE close FROM market_data WHERE asset_id = THIS.asset_id AND fn::session_from_timestamp(ts) = THIS.session ORDER BY ts DESC LIMIT 1)
        ) AS candle_pattern,
        time::format((SELECT VALUE ts FROM market_data WHERE asset_id = THIS.asset_id AND high = THIS.high AND fn::session_from_timestamp(ts) = THIS.session ORDER BY ts ASC LIMIT 1), "%H:%M") AS high_ts,
        time::format((SELECT VALUE ts FROM market_data WHERE asset_id = THIS.asset_id AND low = THIS.low AND fn::session_from_timestamp(ts) = THIS.session ORDER BY ts ASC LIMIT 1), "%H:%M") AS low_ts
    FROM market_data 
    GROUP BY asset_id, time::format(time::floor(ts, '1d'), "%a"), session;
    "#;

    // Daily aggregation view
    let daily_view = r#"
    DEFINE TABLE daily_agg AS
    SELECT
        asset_id,
        time::format(date, "%a") AS date,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume,
        fn::candle_pattern(
            (SELECT VALUE open FROM session_agg WHERE asset_id = THIS.asset_id AND time::format(date, "%a") = THIS.date ORDER BY session ASC LIMIT 1),
            math::max(high),
            math::min(low),
            (SELECT VALUE close FROM session_agg WHERE asset_id = THIS.asset_id AND time::format(date, "%a") = THIS.date ORDER BY session DESC LIMIT 1)
        ) AS candle_pattern,
        time::format((SELECT VALUE high_ts FROM session_agg WHERE asset_id = THIS.asset_id AND time::format(date, "%a") = THIS.date AND high = THIS.high ORDER BY high_ts ASC LIMIT 1), "%H:%M") AS high_ts,
        time::format((SELECT VALUE low_ts FROM session_agg WHERE asset_id = THIS.asset_id AND time::format(date, "%a") = THIS.date AND low = THIS.low ORDER BY low_ts ASC LIMIT 1), "%H:%M") AS low_ts
    FROM session_agg
    GROUP BY asset_id, time::format(date, "%a");
    "#;

    // Weekday aggregation view
    let weekday_view = r#"
    DEFINE TABLE weekday_agg AS
    SELECT
        asset_id,
        time::floor(date, '1w') AS week,
        time::format(date, "%a") AS weekday,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume,
        fn::candle_pattern(
            (SELECT VALUE open FROM daily_agg WHERE asset_id = THIS.asset_id AND time::floor(date,'1w') = THIS.week AND time::format(date, "%a") = THIS.weekday ORDER BY date ASC LIMIT 1),
            math::max(high),
            math::min(low),
            (SELECT VALUE close FROM daily_agg WHERE asset_id = THIS.asset_id AND time::floor(date,'1w') = THIS.week AND time::format(date, "%a") = THIS.weekday ORDER BY date DESC LIMIT 1)
        ) AS candle_pattern,
        time::format((SELECT VALUE high_ts FROM daily_agg WHERE asset_id = THIS.asset_id AND time::floor(date,'1w') = THIS.week AND time::format(date, "%a") = THIS.weekday AND high = THIS.high ORDER BY high_ts ASC LIMIT 1), "%H:%M") AS high_ts,
        time::format((SELECT VALUE low_ts FROM daily_agg WHERE asset_id = THIS.asset_id AND time::floor(date,'1w') = THIS.week AND time::format(date, "%a") = THIS.weekday AND low = THIS.low ORDER BY low_ts ASC LIMIT 1), "%H:%M") AS low_ts
    FROM daily_agg
    GROUP BY asset_id, week, time::format(date, "%a");
    "#;

    // Weekly aggregation view
    let weekly_view = r#"
    DEFINE TABLE weekly_agg AS
    SELECT
        asset_id,
        CONCAT("Week ", time::week(date)) AS week,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume,
        fn::candle_pattern(
            (SELECT VALUE open FROM daily_agg WHERE asset_id = THIS.asset_id AND time::floor(date,'1w') = time::floor(THIS.date,'1w') ORDER BY date ASC LIMIT 1),
            math::max(high),
            math::min(low),
            (SELECT VALUE close FROM daily_agg WHERE asset_id = THIS.asset_id AND time::floor(date,'1w') = time::floor(THIS.date,'1w') ORDER BY date DESC LIMIT 1)
        ) AS candle_pattern,
        time::format((SELECT VALUE high_ts FROM daily_agg WHERE asset_id = THIS.asset_id AND time::floor(date,'1w') = time::floor(THIS.date,'1w') AND high = THIS.high ORDER BY high_ts ASC LIMIT 1), "%H:%M") AS high_ts,
        time::format((SELECT VALUE low_ts FROM daily_agg WHERE asset_id = THIS.asset_id AND time::floor(date,'1w') = time::floor(THIS.date,'1w') AND low = THIS.low ORDER BY low_ts ASC LIMIT 1), "%H:%M") AS low_ts
    FROM daily_agg
    GROUP BY asset_id, time::week(date);
    "#;

    // Monthly aggregation view
    let monthly_view = r#"
    DEFINE TABLE monthly_agg AS
    SELECT
        asset_id,
        time::format(date, "%b") AS month,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume,
        fn::candle_pattern(
            (SELECT VALUE open FROM daily_agg WHERE asset_id = THIS.asset_id AND time::floor(date,'1mo') = time::floor(THIS.date,'1mo') ORDER BY date ASC LIMIT 1)[0].open,
            math::max(high),
            math::min(low),
        (SELECT VALUE high_ts FROM daily_agg WHERE asset_id = THIS.asset_id AND time::floor(date,'1mo') = THIS.month AND high = THIS.high ORDER BY high_ts ASC LIMIT 1)[0].high_ts AS high_ts,
        (SELECT VALUE low_ts FROM daily_agg WHERE asset_id = THIS.asset_id AND time::floor(date,'1mo') = THIS.month AND low = THIS.low ORDER BY low_ts ASC LIMIT 1)[0].low_ts AS low_ts

    FROM daily_agg
    GROUP BY asset_id, month;
    "#;

     // Execute all view definitions
    db.query(session_view).bind(("asset_id", asset_id.clone())).await.context("Failed to define session view")?;
    db.query(daily_view).bind(("asset_id", asset_id.clone())).await.context("Failed to define daily view")?;
    db.query(weekday_view).bind(("asset_id", asset_id.clone())).await.context("Failed to define weekday view")?;
    db.query(weekly_view).bind(("asset_id", asset_id.clone())).await.context("Failed to define weekly view")?;
    db.query(monthly_view).bind(("asset_id", asset_id.clone())).await.context("Failed to define monthly view")?;

    println!(" - All aggregation views defined. ✅");
    Ok(())  
}


// Query functions for each view
pub async fn get_session_data(db: &DB, asset_id: &RecordId) -> Result<Vec<surrealdb::Value>> {
    let mut results = db.query("SELECT * FROM session_agg WHERE asset_id = $asset_id")
        .bind(("asset_id", asset_id.clone()))
        .await
        .context("Failed to query session data")?;
    
    Ok(results.take(0)?)
}

pub async fn get_daily_data(db: &DB, asset_id: &RecordId) -> Result<Vec<surrealdb::Value>> {
    let mut  results = db.query("SELECT * FROM daily_agg WHERE asset_id = $asset_id")
        .bind(("asset_id", asset_id.clone()))
        .await
        .context("Failed to query daily data")?;
    
    Ok(results.take(0)?)
}

pub async fn get_weekday_data(db: &DB, asset_id: &RecordId) -> Result<Vec<surrealdb::Value>> {
    let mut results = db.query("SELECT * FROM weekday_agg WHERE asset_id = $asset_id")
        .bind(("asset_id", asset_id.clone()))
        .await
        .context("Failed to query weekday data")?;
    
    Ok(results.take(0)?)
}

pub async fn get_weekly_data(db: &DB, asset_id: &RecordId) -> Result<Vec<surrealdb::Value>> {
    let mut results = db.query("SELECT * FROM weekly_agg WHERE asset_id = $asset_id")
        .bind(("asset_id", asset_id.clone()))
        .await
        .context("Failed to query weekly data")?;
    
    Ok(results.take(0)?)
}

pub async fn get_monthly_data(db: &DB, asset_id: &RecordId) -> Result<Vec<surrealdb::Value>> {
    let mut results = db.query("SELECT * FROM monthly_agg WHERE asset_id = $asset_id")
        .bind(("asset_id", asset_id.clone()))
        .await
        .context("Failed to query daily data")?;
    Ok(results.take(0)?)
}