use anyhow::{Result, Context};
use std::time::Duration;
use std::process; // Used for the process exit (though not strictly necessary with proper error returns)

// Assuming these crates are configured in the Cargo.toml workspace
use database_engine::{
    ingestion::run_consumer, 
    config::ConsumerConfig,
    runtime::setup_database_pool, 
};
use data_engine::{
    ingestion::ingest_from_csv, 
    config::ProducerConfig,
};


#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Full Trading System Pipeline Orchestrator...");

    // 1. Load Configurations from environment (.env)
    let consumer_config = ConsumerConfig::load()?;
    let producer_config = ProducerConfig::load()?;

    // Clone the topic into an owned String so spawned tasks can use it without borrowing local config
    let kafka_topic = consumer_config.kafka_topic.clone();

    println!("Configuration Loaded:");
    println!("  -> Kafka Brokers: {}", consumer_config.kafka_brokers);
    println!("  -> Target Topic: {}", kafka_topic);
    println!("  -> Data Source ID: {}", producer_config.data_source_id); 
    println!("  -> DB URL: [REDACTED]");
    println!("  -> CSV Path: {:?}", producer_config.file_path);

    // 2. Initialize Database Pool 
    let pool = setup_database_pool().await?;
    
    // CRITICAL: Clone configs and pool for the consumer
    let consumer_pool = pool.clone();
    let consumer_config_cloned = consumer_config.clone();

    // 3. Spawn Consumer (runs indefinitely, must be spawned first to receive data)
    let consumer_handle = tokio::spawn(async move {
        eprintln!("DEBUG: Consumer task started. Attempting to call run_consumer.");
        // Run consumer, capture result
        let result = run_consumer(consumer_config_cloned, consumer_pool).await;
        
        match result {
            Ok(_) => {
                println!("\n[CONSUMER] Graceful shutdown.");
                Ok(()) // Explicitly return Ok(())
            }
            Err(e) => {
                eprintln!("\n[CONSUMER] CRITICAL FAILURE: {:?}", e);
                Err(e) // Return the original Err(e)
            }
        }
    });

    // Wait a moment for the consumer to subscribe before starting the producer
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // CRITICAL FIX: Clone the config for the producer before moving it into the closure
    let producer_config_cloned = producer_config.clone();
    
    // 4. Spawn Producer (sends the historical CSV data, then exits)
    let producer_handle = tokio::spawn(async move {
        // This ingests the data from the actual CSV file
        match ingest_from_csv(producer_config_cloned).await { // Use the cloned config
            Ok(_) => {
                println!("\n[PRODUCER] Historical Ingestion Complete. Data sent to topic: {}", kafka_topic);
                Ok(()) // Explicitly return Ok(())
            }
            Err(e) => {
                eprintln!("\n[PRODUCER] Critical Failure: {:?}", e);
                Err(e) // Return the original Err(e)
            }
        }
    });

    // 5. Wait for the producer to finish
    let producer_join_result = producer_handle.await.context("Producer task failed during execution (join error)")?;
    // CRITICAL: Check the result of the producer's internal execution
    producer_join_result.context("Producer reported internal error during ingestion")?;


    // 6. Keep the consumer running until interrupted (Ctrl+C)
    println!("\n=======================================================");
    println!("✅ STEP 4: DATA PIPELINE COMPLETE");
    println!("Raw historical data ingestion finished. Consumer is now running and waiting for real-time messages.");
    println!("=======================================================");
    
    // The main process waits on the consumer forever. This will capture the failure 
    // from the consumer_handle's inner execution.
    let consumer_join_result = consumer_handle.await.context("Consumer task crashed or failed to join")?;
    
    // CRITICAL: Check the result of the consumer's internal execution after joining the thread
    consumer_join_result.context("Consumer reported internal error after historical ingestion")?;

    Ok(())
}