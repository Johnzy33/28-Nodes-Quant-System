use anyhow::{Result, Context};
use std::time::Duration;

// Assuming these crates are configured in the Cargo.toml workspace
use database_engine::{
    ingestion::run_consumer, 
    config::ConsumerConfig,
    runtime::setup_database_pool, // CORRECT: Importing the canonical pool setup function
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
    println!("  -> Data Source ID: {}", producer_config.data_source_id); // Assumed to be in ProducerConfig
    println!("  -> DB URL: [REDACTED]");
    println!("  -> CSV Path: {:?}", producer_config.file_path);

    // 2. Initialize Database Pool (Using the correct modular function)
    // setup_database_pool handles parsing the DB URL and building the pool internally.
    let pool = setup_database_pool().await?;

    // 3. Spawn Consumer (runs indefinitely, must be spawned first to receive data)
    let consumer_pool = pool.clone();
    let consumer_config_cloned = consumer_config.clone();
    let consumer_handle = tokio::spawn(async move {
        // The consumer needs to run first to listen for messages
        match run_consumer(consumer_config_cloned, consumer_pool).await {
            Ok(_) => println!("\n[CONSUMER] Graceful shutdown."),
            Err(e) => eprintln!("\n[CONSUMER] Critical Failure: {:?}", e),
        }
    });

    // Wait a moment for the consumer to subscribe before starting the producer
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 4. Spawn Producer (sends the historical CSV data, then exits)
    let producer_handle = tokio::spawn(async move {
        // This ingests the data from the actual CSV file
        match ingest_from_csv(producer_config).await {
            Ok(_) => println!("\n[PRODUCER] Historical Ingestion Complete. Data sent to topic: {}", kafka_topic),
            Err(e) => eprintln!("\n[PRODUCER] Critical Failure: {:?}", e),
        }
    });

    // 5. Wait for the producer to finish (since it processes a finite CSV)
    producer_handle.await.context("Producer task failed during execution")?;

    // 6. Keep the consumer running until interrupted (Ctrl+C)
    println!("\n=======================================================");
    println!("✅ STEP 4: DATA PIPELINE COMPLETE");
    println!("Raw historical data ingestion finished. Consumer is now running and waiting for real-time messages.");
    println!("=======================================================");
    
    // The main process waits on the consumer forever, as it's meant to run continuously
    consumer_handle.await.context("Consumer task failed during execution")?;

    Ok(())
}
