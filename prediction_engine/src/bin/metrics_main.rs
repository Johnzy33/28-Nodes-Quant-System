
use anyhow::Result;
use dotenvy;
use log::info;


use database_engine::runtime;
use prediction_engine::metric::metrics_pipeline;
//use prediction_engine::vortex::vortex::{VortexSnapshot, VortexPoint};

// You might need to add `env_logger::init();` if you haven't set up logging

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging for the 'info!' calls in the runners
    env_logger::init();
    dotenvy::dotenv().ok();


    // 2. Connect to DB
    
    let pool = runtime::setup_database_pool().await?;

    
    info!("==================================================");
    info!("📈 Starting Full ETL Pipeline for All Assets");
    info!("==================================================");
    

    
    metrics_pipeline::run_master_asset_metrics(&pool)
        .await
        .map_err(|e| {
            // Log the full error chain if the orchestrator fails
            eprintln!("🔴 MASTER ETL FAILED! Error: {:?}", e);
            e
        })?;
    
    metrics_pipeline::vortex(&pool)
        .await
        .map_err(|e| {
            // Log the full error chain if the orchestrator fails
            eprintln!("🔴 MASTER ETL FAILED! Error: {:?}", e);
            e
        })?;
    

    info!("==================================================");
    info!("✅ All Meterics Completed Successfully.");
    info!("==================================================");

    Ok(())
}