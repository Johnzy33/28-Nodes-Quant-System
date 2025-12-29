use anyhow::{Context, Result};
use data_engine::consumer_ingestion;
use log::info;
use std::env;
use tokio;
use dotenvy;
use database_engine::runtime::setup_database_pool;

// Import WatchdogState from your updated watchdog module
use data_engine::watchdog::{Mt5Watchdog, WatchdogState};

mod producer_config;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize Logging and Environment
    env_logger::init();
    dotenvy::dotenv().ok();

    info!("🚀 Starting Trading Data Pipeline...");

    // 2. Initialize Postgres Connection Pool
    let pool = setup_database_pool().await
        .context("Failed to set up database connection pool")?;

    // 3. Initialize Shared Watchdog State (The Heartbeat Tracker)
    // This state is shared between the Consumer (who updates it) 
    // and the Watchdog (who checks it).
    let watchdog_state = WatchdogState::new();

    // 4. Kafka Producer (Shared for sending Sync Commands)
    let producer = consumer_ingestion::initialize_producer().await
        .context("Failed to initialize Kafka producer")?;

    // 5. Prepare Paths for Watchdog
    let python_worker_path = env::var("PYTHON_WORKER_PATH")
        .unwrap_or_else(|_| "./mt5_worker.py".to_string());

    // 6. Orchestration: Run Watchdog and Ingestion Engine concurrently
    tokio::try_join!(
        // Task A: The Watchdog
        // Monitors the OS process and the data "freshness" in the watchdog_state
        async {
            info!("🛠️ Starting MT5 Watchdog...");
            let mut watchdog = Mt5Watchdog::new(
                &python_worker_path, 
                watchdog_state.clone()
            );
            
            let res: Result<()> = watchdog.run().await
                .context("Watchdog service failed");
            res
        },

        // Task B: The Ingestion Engine
        // Processes Kafka messages and updates watchdog_state on every successful DB write
        async {
            info!("📥 Starting Ingestion Engine...");
            let res: Result<()> = consumer_ingestion::run_multi_topic_consumer(
                pool, 
                &producer, 
                watchdog_state.clone()
            ).await
                .context("Ingestion Engine failed");
            res
        }
    )?;

    Ok(())
}