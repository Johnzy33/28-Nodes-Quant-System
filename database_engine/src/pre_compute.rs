use anyhow::{Context, Result};
use surrealdb::sql::Value;
use surrealdb::RecordId;
use crate::DB;

pub async fn define_aggregation_views(_db: &DB) -> Result<()> {
    println!(" - Defining aggregation views...");

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
            (SELECT VALUE open FROM market_data m WHERE m.asset_id = THIS.asset_id AND fn::session_from_timestamp(m.ts) = THIS.session ORDER BY m.ts ASC LIMIT 1),
            math::max(high),
            math::min(low),
            (SELECT VALUE close FROM market_data m WHERE m.asset_id = THIS.asset_id AND fn::session_from_timestamp(m.ts) = THIS.session ORDER BY m.ts DESC LIMIT 1)
        ) AS candle_pattern,
        (SELECT VALUE ts FROM market_data m WHERE m.asset_id = THIS.asset_id AND fn::session_from_timestamp(m.ts) = THIS.session AND m.high = math::max(high) ORDER BY m.ts ASC LIMIT 1) AS high_ts,
        (SELECT VALUE ts FROM market_data m WHERE m.asset_id = THIS.asset_id AND fn::session_from_timestamp(m.ts) = THIS.session AND m.low = math::min(low) ORDER BY m.ts ASC LIMIT 1) AS low_ts
    FROM market_data
    GROUP BY asset_id, time::format(time::floor(ts, '1d'), "%a"), fn::session_from_timestamp(ts);
    "#;

    let daily_view = r#"
    DEFINE TABLE daily_agg AS
    SELECT
        asset_id,
        time::format(time::floor(ts, '1d'), "%a") AS date,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume,
        fn::candle_pattern(
            (SELECT VALUE open FROM session_agg s WHERE s.asset_id = THIS.asset_id AND s.date = THIS.date ORDER BY s.session ASC LIMIT 1),
            math::max(high),
            math::min(low),
            (SELECT VALUE close FROM session_agg s WHERE s.asset_id = THIS.asset_id AND s.date = THIS.date ORDER BY s.session DESC LIMIT 1)
        ) AS candle_pattern,
        (SELECT VALUE high_ts FROM session_agg s WHERE s.asset_id = THIS.asset_id AND s.date = THIS.date AND s.high = math::max(high) ORDER BY s.high_ts ASC LIMIT 1) AS high_ts,
        (SELECT VALUE low_ts FROM session_agg s WHERE s.asset_id = THIS.asset_id AND s.date = THIS.date AND s.low = math::min(low) ORDER BY s.low_ts ASC LIMIT 1) AS low_ts
    FROM session_agg
    GROUP BY asset_id, time::format(time::floor(date, '1d'), "%a");
    "#;

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
            (SELECT VALUE open FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1w') = THIS.week AND d.date = THIS.date ORDER BY d.date ASC LIMIT 1),
            math::max(high),
            math::min(low),
            (SELECT VALUE close FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1w') = THIS.week AND d.date = THIS.date ORDER BY d.date DESC LIMIT 1)
        ) AS candle_pattern,
        (SELECT VALUE high_ts FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1w') = THIS.week AND d.high = math::max(high) ORDER BY d.high_ts ASC LIMIT 1) AS high_ts,
        (SELECT VALUE low_ts FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1w') = THIS.week AND d.low = math::min(low) ORDER BY d.low_ts ASC LIMIT 1) AS low_ts
    FROM daily_agg
    GROUP BY asset_id, week, time::format(date, "%a");
    "#;

    let weekly_view = r#"
    DEFINE TABLE weekly_agg AS
    SELECT
        asset_id,
        CONCAT("Week ", time::week(date)) AS week,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume,
        fn::candle_pattern(
            (SELECT VALUE open FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1w') = time::floor(THIS.date,'1w') ORDER BY d.date ASC LIMIT 1),
            math::max(high),
            math::min(low),
            (SELECT VALUE close FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1w') = time::floor(THIS.date,'1w') ORDER BY d.date DESC LIMIT 1)
        ) AS candle_pattern,
        (SELECT VALUE high_ts FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1w') = time::floor(THIS.date,'1w') AND d.high = math::max(high) ORDER BY d.high_ts ASC LIMIT 1) AS high_ts,
        (SELECT VALUE low_ts FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1w') = time::floor(THIS.date,'1w') AND d.low = math::min(low) ORDER BY d.low_ts ASC LIMIT 1) AS low_ts
    FROM daily_agg
    GROUP BY asset_id, time::week(date);
    "#;

    let monthly_view = r#"
    DEFINE TABLE monthly_agg AS
    SELECT
        asset_id,
        time::format(date, "%b") AS month,
        math::max(high) AS high,
        math::min(low) AS low,
        math::sum(volume) AS volume,
        fn::candle_pattern(
            (SELECT VALUE open FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1mo') = time::floor(THIS.date,'1mo') ORDER BY d.date ASC LIMIT 1),
            math::max(high),
            math::min(low),
            (SELECT VALUE close FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1mo') = time::floor(THIS.date,'1mo') ORDER BY d.date DESC LIMIT 1)
        ) AS candle_pattern,
        (SELECT VALUE high_ts FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1mo') = time::floor(THIS.date,'1mo') AND d.high = math::max(high) ORDER BY d.high_ts ASC LIMIT 1) AS high_ts,
        (SELECT VALUE low_ts FROM daily_agg d WHERE d.asset_id = THIS.asset_id AND time::floor(d.date,'1mo') = time::floor(THIS.date,'1mo') AND d.low = math::min(low) ORDER BY d.low_ts ASC LIMIT 1) AS low_ts
    FROM daily_agg
    GROUP BY asset_id, month;
    "#;

    _db.query(session_view).await.context("Failed to define session view")?;
    _db.query(daily_view).await.context("Failed to define daily view")?;
    _db.query(weekday_view).await.context("Failed to define weekday view")?;
    _db.query(weekly_view).await.context("Failed to define weekly view")?;
    _db.query(monthly_view).await.context("Failed to define monthly view")?;

    println!(" - All aggregation views defined. ✅");
    Ok(())
}

pub async fn get_session_data(db: &DB, asset_id: &RecordId) -> Result<Vec<Value>> {
    let mut res = db.query("SELECT * FROM session_agg WHERE asset_id = $asset_id")
        .bind(("asset_id", asset_id.clone()))
        .await
        .context("Failed to query session data")?;
    Ok(res.take(0)?)
}

pub async fn get_daily_data(db: &DB, asset_id: &RecordId) -> Result<Vec<Value>> {
    let mut res = db.query("SELECT * FROM daily_agg WHERE asset_id = $asset_id")
        .bind(("asset_id", asset_id.clone()))
        .await
        .context("Failed to query daily data")?;
    Ok(res.take(0)?)
}

pub async fn get_weekday_data(db: &DB, asset_id: &RecordId) -> Result<Vec<Value>> {
    let mut res = db.query("SELECT * FROM weekday_agg WHERE asset_id = $asset_id")
        .bind(("asset_id", asset_id.clone()))
        .await
        .context("Failed to query weekday data")?;
    Ok(res.take(0)?)
}

pub async fn get_weekly_data(db: &DB, asset_id: &RecordId) -> Result<Vec<Value>> {
    let mut res = db.query("SELECT * FROM weekly_agg WHERE asset_id = $asset_id")
        .bind(("asset_id", asset_id.clone()))
        .await
        .context("Failed to query weekly data")?;
    Ok(res.take(0)?)
}

pub async fn get_monthly_data(db: &DB, asset_id: &RecordId) -> Result<Vec<Value>> {
    let mut res = db.query("SELECT * FROM monthly_agg WHERE asset_id = $asset_id")
        .bind(("asset_id", asset_id.clone()))
        .await
        .context("Failed to query monthly data")?;
    Ok(res.take(0)?)
}
