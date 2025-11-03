
use anyhow::Result;
use tokio::join;

// Import everything needed from the library crates
use database_engine::{
    setup_database_pool,
    runtime::run_consumer_service,
    config::ConsumerConfig as DbConfig,
    Pool // The DeadPool type
};
use data_engine::{ingest_from_csv, config::ProducerConfig};

// --- ONE-TIME JOB ---

/// Runs the one-time historical data ingestion job.
/// This should run before the continuous services start.
async fn run_historical_ingestion() -> Result<()> {
    let config = ProducerConfig::default();
    println!("--- Running Historical Data Producer Job ---");
    // This calls the ingestion logic from the data_engine library
    ingest_from_csv(config).await
}

// --- MAIN ORCHESTRATOR ---

#[tokio::main]
async fn main() -> Result<()> {
    
    println!("--- Monolithic Trading System Starting ---");

    // ====================================================================
    // PHASE 1: SEQUENTIAL SETUP & ONE-TIME JOBS (The main thread waits here)
    // ====================================================================

    // 1. Initialize the central DB Pool and apply schema (Critical Setup)
    let pool: Pool = match setup_database_pool().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("\n🔴 CRITICAL SETUP FAILURE: Database Pool/Schema failed to initialize: {:?}", e);
            return Err(e); // Halt execution if the DB isn't ready
        }
    };
    
    // 2. Run Historical Data Load (Optional, runs once, process continues after completion/failure)
    if let Err(e) = run_historical_ingestion().await {
        eprintln!("\n🟡 WARNING: Historical Producer Job Failed, continuing with live services: {:?}", e);
    }
    
    // ====================================================================
    // PHASE 2: CONCURRENT LIVE SERVICE STARTUP (Services run via tokio::join!)
    // ====================================================================

    // 3. Configuration 
    // This configures the US1000 Consumer to read from 'us1000_candles_1hr_raw'
    let db_config_us1000 = DbConfig {
        kafka_topic: "us1000_candles_1hr_raw".to_string(),
        kafka_group_id: "ingestion-group-us1000".to_string(),
        // Note: The db_url is loaded from environment/default in DbConfig::default()
        ..DbConfig::default() 
    }; 
    
    // --- 4. Concurrent Startup ---
    
    // Task A: Long-running Ingestion Service (Kafka Consumer)
    // Pass a clone of the Pool to this service
    let ingestion_task = run_consumer_service(db_config_us1000, pool.clone()); 
    
    // Task B: Prediction Engine (Step 5) - Placeholder for Strategy Runner
    // The Strategy Runner will also use a clone of the same central Pool
    // let strategy_task_daily = prediction_engine::run_strategy(pool.clone(), strategy_config_daily); 
    
    // Run all CONTINUOUS services concurrently
    // The application blocks here, running the tasks until one fails or exits
    let (res_ingestion) = join!(
        ingestion_task, 
        // strategy_task_daily
    );

    // --- 5. Handle Results ---
    if let Err(e) = res_ingestion { 
        eprintln!("\n🔴 CRITICAL: Ingestion Service Failed and caused application exit: {:?}", e); 
    }

    Ok(())
}