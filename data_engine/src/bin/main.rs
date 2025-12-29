// src/bin/run_master_etl.rs
// Run with: cargo run --bin run_master_etl

// use anyhow::Result;
// use std::env;
// use dotenvy;
// use log::info;


// use database_engine::runtime::setup_database_pool;
// // You might need to add `env_logger::init();` if you haven't set up logging

// #[tokio::main]
// async fn main() -> Result<()> {
//     // Initialize logging for the 'info!' calls in the runners
//     env_logger::init();
//     dotenvy::dotenv().ok();


//     // 2. Connect to DB
//     // Use a higher connection limit since the orchestrator runs multiple parallel tasks internally.
//     let pool = setup_database_pool().await?;

    

//     // 4. Run the Master Orchestrator
//     // We pass a reference to the pool, and the orchestrator clones it for its sub-tasks.
//     run_all_assets_master_etl(&pool)
//         .await
//         .map_err(|e| {
//             // Log the full error chain if the orchestrator fails
//             eprintln!("🔴 MASTER ETL FAILED! Error: {:?}", e);
//             e
//         })?;

    
    
//     Ok(())
// }


// src/bin/run_master_etl.rs
// Run with: cargo run --bin run_master_etl

use anyhow::Result;
use dotenvy;
use log::{info, error};
use database_engine::runtime;
use data_engine::data_service; 

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Setup Environment
    env_logger::init();
    dotenvy::dotenv().ok();

    info!("🚀 Initializing Master ETL Process...");

    // 2. Connect to DB
    // The orchestrator uses a semaphore (e.g., 8) to control concurrency,
    // so ensure your pool size is at least concurrency_limit + 2.
    let pool = runtime::setup_database_pool().await?;

    // 3. Initialize DataService
    // We pass the pool into the service. 
    let data_service = data_service::DataService::new(pool);

    // 4. Run the Master Orchestrator
    // We call the method directly on the service instance.
    if let Err(e) = data_service.assets_master_etl().await {
        error!("🔴 MASTER ETL FAILED! Error: {:?}", e);
        std::process::exit(1);
    }

    info!("✅ MASTER ETL SHUTDOWN: All processes finished successfully.");
    Ok(())
}