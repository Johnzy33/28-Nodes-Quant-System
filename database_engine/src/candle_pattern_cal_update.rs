use anyhow::{Result, Context};
use surrealdb::{RecordId, Response};
use crate::DB;
use shared_models::{candle, DailyAgg};

pub async fn calculate_and_update_candle_patterns(db: &DB, asset_id: &RecordId) -> Result<()> {
    println!("  - Calculating and updating daily candle patterns...");
    
    let mut query_result: Response = db.query("SELECT * FROM daily_agg WHERE asset_id = $asset_id ORDER BY date ASC")
        .bind(("asset_id", asset_id.clone()))
        .await
        .context("Failed to fetch daily aggregates")?;
    
    let daily_aggs: Vec<DailyAgg> = query_result.take(0).context("Failed to deserialize daily aggregates")?;
    
    for agg in daily_aggs {
        let pattern = candle::pattern_from_ohlc(
            agg.open, agg.high, agg.low, agg.close,
            candle::DEFAULT_DOJI_BODY_RATIO,
            candle::DEFAULT_BODY_WICK_RATIO_LONG,
            candle::DEFAULT_BODY_WICK_RATIO_SHORT,
            candle::DEFAULT_UPPER_VS_LOWER_RATIO,
            candle::DEFAULT_EPS,
        );
        
        let record_id = agg.id.as_ref().unwrap().clone();
        
        db.query("UPDATE $id SET day_candle_pattern = $pattern")
            .bind(("id", record_id))
            .bind(("pattern", pattern))
            .await
            .context("Failed to update daily aggregate with candle pattern")?;
    }

    println!("  - Candle patterns updated. ✅");
    Ok(())
}