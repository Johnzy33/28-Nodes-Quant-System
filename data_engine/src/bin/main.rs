
use anyhow::{Result,Error};
use dotenvy;
use log::{info, error};
use database_engine::runtime;
use shared_models::data_model; 
use data_engine::data_service::traits::DataOrchestratorExt;
use data_engine::data_service::traits::DataMaintenanceExt;

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
    let data_service = data_model::DataService::new(pool);

    // 4. Run the Master Orchestrator
    // We call the method directly on the service instance.
    if let Err(e) = data_service.assets_master_etl().await {
        error!("🔴 MASTER ETL FAILED! Error: {:?}", e);
        std::process::exit(1);
    }

    // if let Err(e) = data_service.send_startup_handshake().await {
    //     error!("🔴 MASTER ETL FAILED! Error: {:?}", e);
    //     std::process::exit(1);
    // }

    info!("✅ MASTER ETL SHUTDOWN: All processes finished successfully.");
    Ok(())
}