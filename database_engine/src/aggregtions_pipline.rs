use anyhow::Result;
use surrealdb::{RecordId};
use crate::DB;
use crate::surreal_aggregator_function::define_surreal_functions;
use crate::aggregators::*;
use crate::candle_pattern_cal_update::calculate_and_update_candle_patterns; 


pub async fn run_all_aggregations(db: &DB, asset_id: &RecordId) -> Result<()> {
    println!("Running aggregation pipeline for asset: {}", asset_id.to_string());
    
    //define_surreal_functions(db).await?;

    //aggregate_sessions(db, asset_id).await?;

   // aggregate_daily(db, asset_id).await?;
    
   // aggregate_weekly(db, asset_id).await?;

   // aggregate_weekday(db, asset_id).await?;

   // aggregate_monthly(db, asset_id).await?;

    calculate_and_update_candle_patterns(db, asset_id).await?;
    
    println!("Aggregation pipeline completed successfully. ✅");
    Ok(())
}