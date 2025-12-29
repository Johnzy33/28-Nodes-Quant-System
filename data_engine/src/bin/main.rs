// src/bin/run_master_etl.rs
// Run with: cargo run --bin run_master_etl

use anyhow::Result;
use std::env;
use dotenvy;
use log::info;


use database_engine::runtime::setup_database_pool;
use data_engine::run_all_assets_master_etl;
// You might need to add `env_logger::init();` if you haven't set up logging

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging for the 'info!' calls in the runners
    env_logger::init();
    dotenvy::dotenv().ok();


    // 2. Connect to DB
    // Use a higher connection limit since the orchestrator runs multiple parallel tasks internally.
    let pool = setup_database_pool().await?;

    // 3. Define Test Parameters
    //let asset_id = "assets:US2000:FundedNext"; // The asset to process
   // let limit = 1000000;          // Used only by the L1 Session ETL

    println!("==================================================");
    println!("📈 Starting Full ETL Pipeline for All Assets");
    println!("==================================================");

    // 4. Run the Master Orchestrator
    // We pass a reference to the pool, and the orchestrator clones it for its sub-tasks.
    run_all_assets_master_etl(&pool)
        .await
        .map_err(|e| {
            // Log the full error chain if the orchestrator fails
            eprintln!("🔴 MASTER ETL FAILED! Error: {:?}", e);
            e
        })?;

    println!("==================================================");
    println!("✅ All ETLs (L1 to L5) Completed Successfully.");
    println!("==================================================");
    
    Ok(())
}