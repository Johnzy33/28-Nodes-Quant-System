
use anyhow::Context;
use tokio_postgres::Client;
use chrono::{DateTime, Utc};
use crate::candle_pattern::classify_with_defaults;


#[derive(Debug)]
pub struct SessionCandle {
    time: DateTime<Utc>,
    asset_id: String,
    session_name: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

/// Queries unclassified sessions and updates the table with the calculated pattern.
pub async fn process_unclassified_sessions(client: &mut Client) -> anyhow::Result<()> {
    log::info!("Querying for unclassified session candles...");

    // We query the materialized view which generates the data we need.
    // The view is session_aggregate_view, but to update, we must target the base table
    // that the CAGG writes to, which is session_candles.
    let query_result = client.query(
        "SELECT time, asset_id, session_name, open, high, low, close 
         FROM session_aggregate_view 
         WHERE session_pattern IS NULL 
         ORDER BY time ASC LIMIT 1000", // Process in batches
        &[],
    ).await.context("Failed to query unclassified session candles")?;

    let unclassified_candles: Vec<SessionCandle> = query_result.into_iter().map(|row| {
        SessionCandle {
            time: row.get("time"),
            asset_id: row.get("asset_id"),
            session_name: row.get("session_name"),
            open: row.get("open"),
            high: row.get("high"),
            low: row.get("low"),
            close: row.get("close"),
        }
    }).collect();

    if unclassified_candles.is_empty() {
        log::info!("No new session candles to classify. Sleeping...");
        return Ok(());
    }

    log::info!("Classifying {} new session candles.", unclassified_candles.len());

    let mut tx = client.transaction().await.context("Failed to start transaction")?;
    
    // Use an UPSERT approach to update the candle_pattern field
    let stmt = tx.prepare(
        "UPDATE session_pattern 
         SET session_pattern = $1
         WHERE time = $2 AND asset_id = $3 AND session_name = $4;"
    ).await.context("Failed to prepare update statement")?;

    for candle in unclassified_candles {
        // --- CORE CLASSIFICATION LOGIC ---
        let pattern = classify_with_defaults(
            candle.open, 
            candle.high, 
            candle.low, 
            candle.close
        );
        let pattern_str = pattern.as_str();
        
        // Execute the update
        tx.execute(
            &stmt, 
            &[&pattern_str, &candle.time, &candle.asset_id, &candle.session_name]
        ).await.context("Failed to execute update for candle pattern")?;
    }

    tx.commit().await.context("Failed to commit transaction")?;
    log::info!("Successfully classified and updated session patterns.");

    Ok(())
}