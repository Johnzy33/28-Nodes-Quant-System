use anyhow::{Context, Ok, Result};
use surrealdb::{RecordId};
use crate::DB;

pub async fn define_aggregation_views(db: &DB, asset_id: &RecordId) -> Result<()> {
    println!("  - Defining aggregation views...");

    // Session aggregation view
    let session_view = r#"
    DEFINE TABLE session_agg AS
    SELECT 
        asset_id,
        time::format(time::floor(ts, '1d'), "%a") AS date,
        fn::session_from_timestamp(ts) AS session,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume
    FROM market_data 
    GROUP BY asset_id, date, session;
    "#;

    // Daily aggregation view
    let daily_view = r#"
    DEFINE TABLE daily_agg AS
    SELECT
        asset_id,
        time::format(date, "%a") AS date,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume
    FROM session_agg
    GROUP BY asset_id, date;
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
        math::sum(volume) AS volume
    FROM daily_agg
    GROUP BY asset_id, week, weekday;
    "#;

    // Weekly aggregation view
    let weekly_view = r#"
    DEFINE TABLE weekly_agg AS
    SELECT
        asset_id,
        time::floor(date, '1w') AS week,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume
    FROM daily_agg
    GROUP BY asset_id, week;
    "#;

    // Monthly aggregation view
    let monthly_view = r#"
    DEFINE TABLE monthly_agg AS
    SELECT
        asset_id,
        time::format(date, "%b") AS month,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume
    FROM daily_agg
    GROUP BY asset_id, month;
    "#;

     // Execute all view definitions
    db.query(session_view).await.context("Failed to define session view")?;
    db.query(daily_view).await.context("Failed to define daily view")?;
    db.query(weekday_view).await.context("Failed to define weekday view")?;
    db.query(weekly_view).await.context("Failed to define weekly view")?;
    db.query(monthly_view).await.context("Failed to define monthly view")?;

    println!("  - All aggregation views defined. ✅");
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