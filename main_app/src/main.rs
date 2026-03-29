
// use anyhow::{Context, Result};
// use data_engine::{ingestion, producer_config};
// use log::info;
// use std::env;
// use std::sync::Arc; // Added for Arc
// use tokio;
// use dotenvy;
// use database_engine::runtime::setup_database_pool;

// // Import your SPG Orchestrator and DataService
// use shared_models::data_model::DataService;
// use shared_models::traits::MarketDataHandler;
// use prediction_engine::ui_main::run_tui;

// // Import WatchdogState
// use data_engine::
// {watchdog::{Mt5Watchdog, WatchdogState},
// producer_config::IngestionCoordinatorConfig};
// use data_engine::traits::{DataIngestionExt, DataMaintenanceExt};


fn main()  {}




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
//     orchestrator.bootstrap_assets().await
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
        // async {
        //     info!("🛠️ Starting MT5 Watchdog...");
        //     let mut watchdog = Mt5Watchdog::new(
        //         &python_worker_path, 
        //         watchdog_state.clone()
        //     );
            
        //     watchdog.run().await
        //         .context("Watchdog service failed")
        // },

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

// #[tokio::main]
// async fn main() -> Result<()> {
//     env_logger::init();
//     dotenvy::dotenv().ok();

//     let pool = setup_database_pool().await.context("DB Pool failed")?;
    
//     // --- CLEAN SPG SETUP ---
//     let data_service = DataService::new(pool.clone());
//     let orchestrator = prediction_engine::symmetric_price_grid::orchestrator::init_spg_engine(data_service).await
//         .map_err(|e| anyhow::anyhow!("SPG Init failed: {}", e))?;

//     let watchdog_state = WatchdogState::new();
//     let producer = ingestion::initialize_producer().await?;
//     // 6. Prepare Paths for Watchdog

//     let python_worker_path = env::var("PYTHON_WORKER_PATH")
//         .unwrap_or_else(|_| "./mt5_worker.py".to_string());

//     tokio::try_join!(
//         // Watchdog stays the same
//         async {
//             info!("🛠️ Starting MT5 Watchdog...");
//             let mut watchdog = Mt5Watchdog::new(
//                 &python_worker_path, 
//                 watchdog_state.clone()
//             );
            
//             watchdog.run().await
//                 .context("Watchdog service failed")
//         },

//         // Ingestion Engine uses the orchestrator we just initialized
//         async {
//             ingestion::run_multi_topic_consumer(
//                 pool, 
//                 &producer, 
//                 watchdog_state.clone(),
//                 orchestrator // <--- Cleanly passed here
//             ).await.context("Ingestion Engine failed")
//         }
//     )?;

//     Ok(())
// }



// #[tokio::main]
// async fn main() -> Result<()> {
//     env_logger::init();
//     dotenvy::dotenv().ok();

//     let pool = setup_database_pool().await.context("DB Pool failed")?;
    
//     // 1. Initialize Orchestrator (Initially Dormant)
//     let data_service = DataService::new(pool.clone());
//     let orchestrator = prediction_engine::symmetric_price_grid::orchestrator::init_spg_engine(data_service).await
//         .map_err(|e| anyhow::anyhow!("SPG Init failed: {}", e))?;

//     let handler: Arc<dyn MarketDataHandler> = orchestrator.clone();

//     // 2. Create the Sync Signal Channel
//     // initial value is 'false' (not synced)
//     let (sync_tx, sync_rx) = tokio::sync::watch::channel(false);

//     let watchdog_state = WatchdogState::new();
//     let producer = ingestion::initialize_producer().await?;
//     let python_worker_path = env::var("PYTHON_WORKER_PATH")
//         .unwrap_or_else(|_| "./mt5_worker.py".to_string());

//     // 3. Run all three services concurrently
//     tokio::try_join!(
//         // TASK A: MT5 Watchdog
//         async {
//             info!("🛠️ Starting MT5 Watchdog...");
//             let mut watchdog = Mt5Watchdog::new(&python_worker_path, watchdog_state.clone());
//             watchdog.run().await.context("Watchdog service failed")
//         },

//         // TASK B: Ingestion Engine (Reports sync status via sync_tx)
//         async {
//             ingestion::run_multi_topic_consumer(
//                 pool, 
//                 producer, 
//                 watchdog_state.clone(),
//                 handler,
//                 sync_tx // <--- Pass the Sender
//             ).await.context("Ingestion Engine failed")
//         },

//         // TASK C: The "Post-Sync" Bootstrapper
        // async {
        //     let mut rx = sync_rx;
        //     info!("⏳ SPG Bootstrapper: Waiting for Kafka Sync to reach 0 lag...");

        //     // Wait until ingestion.rs sends the 'true' signal
        //     while !*rx.borrow() {
        //         if rx.changed().await.is_err() {
        //             return Err(anyhow::anyhow!("Sync signal channel closed unexpectedly"));
        //         }
        //     }

        //     info!("🛡️ Sync Complete. Running Master ETL...");
        //     orchestrator.run_master_etl().await
        //         .map_err(|e| anyhow::anyhow!("Master ETL failed: {}", e))?;

        //     info!("🥾 Anchoring Grids from fresh DB state...");
        //     orchestrator.bootstrap_assets().await
        //         .map_err(|e| anyhow::anyhow!("Bootstrap failed: {}", e))?;

        //     // FINAL STEP: Flip the switch to enable on_price_update logic
        //     orchestrator.is_live.store(true, std::sync::atomic::Ordering::SeqCst);
        //     info!("🚀 SPG Engine is now LIVE and processing ticks.");
            
        //     // This task is done, we just keep it alive so try_join doesn't exit
        //     std::future::pending::<()>().await; 
        //    // run_tui(orchestrator.clone()).await?;
        //     Ok(())
        // }
//     )?;

//     Ok(())
// }



//     env_logger::init();
//     dotenvy::dotenv().ok();

//     let pool = setup_database_pool().await.context("DB Pool failed")?;
    
//     // 1. Initialize Main Guy & Orchestrator
//     let data_service = DataService::new(pool.clone());
    
//     // Initialize SPG (The orchestrator will use data_service for DB views)
//     let orchestrator = prediction_engine::symmetric_price_grid::orchestrator::init_spg_engine(data_service.clone()).await
//         .map_err(|e| anyhow::anyhow!("SPG Init failed: {}", e))?;

//     let handler: Arc<dyn MarketDataHandler> = orchestrator.clone();

//     // 2. Create the Sync Signal Channel
//     let (sync_tx, sync_rx) = tokio::sync::watch::channel(false);
//     let watchdog_state = WatchdogState::new();

//     // 3. Run all three services concurrently
//     tokio::try_join!(
//         // TASK A: MT5 Watchdog (External Process Manager)
//         async {
//             let python_worker_path = env::var("PYTHON_WORKER_PATH").unwrap_or_else(|_| "./mt5_worker.py".to_string());
//             info!("🛠️ Starting MT5 Watchdog...");
//             let mut watchdog = Mt5Watchdog::new(&python_worker_path, watchdog_state.clone());
//             watchdog.run().await.context("Watchdog service failed")
//         },

//         // TASK B: Ingestion & Maintenance (The "Main Guy" Extensions)
//         async {
//             // Initialize the shared Producer
//             let producer = Arc::new(DataService::initialize_producer().await?);
            
//             // Load Config
//             let config_path = env::var("INGESTION_CONFIG_PATH").context("INGESTION_CONFIG_PATH not set")?;
//             let config = Arc::new(IngestionCoordinatorConfig::load_from_file(&config_path)?);

//             // 1. Dispatch HWM Sync Requests
//             data_service.publish_startup_sync(producer.clone(), config.clone()).await?;

//             // 2. Start Background Healing (Daily/Weekend)
//             data_service.start_maintenance_loop(producer.clone(), config.clone()).await?;

//             // 3. Run Live Ingestion Engine (This blocks and does the "Tap" to SPG)
//             data_service.run_ingestion(
//                 config, 
//                 handler, 
//                 watchdog_state.clone(), 
//                 sync_tx
//             ).await.context("Ingestion Engine failed")
//         },

//         // TASK C: The "Post-Sync" Bootstrapper
//         async {
//             let mut rx = sync_rx;
//             info!("⏳ SPG Bootstrapper: Waiting for Kafka Sync to reach 0 lag...");

//             // Wait until ingestion.rs sends the 'true' signal
//             while !*rx.borrow() {
//                 if rx.changed().await.is_err() {
//                     return Err(anyhow::anyhow!("Sync signal channel closed unexpectedly"));
//                 }
//             }

//             info!("🛡️ Sync Complete. Running Master ETL...");
//             orchestrator.run_master_etl().await
//                 .map_err(|e| anyhow::anyhow!("Master ETL failed: {}", e))?;

//             info!("🥾 Anchoring Grids from fresh DB state...");
//             orchestrator.bootstrap_assets().await
//                 .map_err(|e| anyhow::anyhow!("Bootstrap failed: {}", e))?;

//             // FINAL STEP: Flip the switch to enable on_price_update logic
//             orchestrator.is_live.store(true, std::sync::atomic::Ordering::SeqCst);
//             info!("🚀 SPG Engine is now LIVE and processing ticks.");
            
//             // This task is done, we just keep it alive so try_join doesn't exit
//             std::future::pending::<()>().await; 
//         //    run_tui(orchestrator.clone()).await?;
//             Ok(())
//         }
//     )?;

//     Ok(())
// }