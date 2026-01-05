// // 

// use anyhow::{Context, Result};
// use data_engine::ingestion;
// use log::info;
// use std::env;
// use std::sync::Arc; // Added for Arc
// use tokio;
// use dotenvy;
// use database_engine::runtime::setup_database_pool;

// // Import your SPG Orchestrator and DataService
// use prediction_engine::symmetric_price_grid::SpgOrchestrator; 
// use shared_models::data_model::DataService;

// // Import WatchdogState
// use data_engine::watchdog::{Mt5Watchdog, WatchdogState};

// mod producer_config;

// #[tokio::main]
// async fn main() -> Result<()> {
//     // 1. Initialize Logging and Environment
//     env_logger::init();
//     dotenvy::dotenv().ok();

//     info!("🚀 Starting Trading Data Pipeline with SPG Integration...");

//     // 2. Initialize Postgres Connection Pool
//     let pool = setup_database_pool().await
//         .context("Failed to set up database connection pool")?;

//     // 3. Initialize SPG Orchestrator (The Real-Time Brain)
//     // We create the DataService first, then the Orchestrator, then bootstrap it.
//     let data_service = DataService::new(pool.clone());
//     let orchestrator = Arc::new(SpgOrchestrator::new(data_service));

//     info!("🧠 Bootstrapping SPG Trackers (Weekly Chains & Daily Grids)...");
//     orchestrator.bootstrap_all().await
//         .map_err(|e| anyhow::anyhow!("SPG Bootstrap failed: {}", e))?;

//     // 4. Initialize Shared Watchdog State
//     let watchdog_state = WatchdogState::new();

//     // 5. Kafka Producer (Shared for sending Sync Commands)
//     let producer = ingestion::initialize_producer().await
//         .context("Failed to initialize Kafka producer")?;

//     // 6. Prepare Paths for Watchdog
//     let python_worker_path = env::var("PYTHON_WORKER_PATH")
//         .unwrap_or_else(|_| "./mt5_worker.py".to_string());

//     // 7. Orchestration: Run Watchdog and Ingestion Engine concurrently
//     tokio::try_join!(
//         // Task A: The Watchdog
//         async {
//             info!("🛠️ Starting MT5 Watchdog...");
//             let mut watchdog = Mt5Watchdog::new(
//                 &python_worker_path, 
//                 watchdog_state.clone()
//             );
            
//             watchdog.run().await
//                 .context("Watchdog service failed")
//         },

//         // Task B: The Ingestion Engine (Now with SPG Orchestrator)
//         async {
//             info!("📥 Starting Ingestion Engine with Zero-Latency SPG Tap...");
//             ingestion::run_multi_topic_consumer(
//                 pool, 
//                 &producer, 
//                 watchdog_state.clone(),
//                 orchestrator.clone() // <--- This passes the Arc<SpgOrchestrator> as Arc<dyn MarketDataHandler>
//             ).await
//                 .context("Ingestion Engine failed")
//         }
//     )?;

//     Ok(())
// }
fn main(){}