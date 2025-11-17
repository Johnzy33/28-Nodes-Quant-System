use anyhow::Result;
use database_engine::runtime::setup_database_pool; 
// Import the new function name
use database_engine::ingestion::run_multi_topic_consumer; 
use log::info;
use dotenvy;
// ... other imports ...

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize logging (Crucial for a consumer service!)
    env_logger::init(); 
    dotenvy::dotenv().ok();

    info!("Starting multi-topic Kafka Consumer Service...");

    // 2. Setup Database Pool
    let pool = setup_database_pool().await?;
    
    // 3. Run the single, multi-topic consumer instance
    // NOTE: The function no longer takes the old ConsumerConfig struct.
    if let Err(e) = run_multi_topic_consumer(pool).await {
        // If the consumer fails (e.g., DB error, Kafka disconnect), the service stops.
        log::error!("CRITICAL: Multi-Topic Consumer terminated with error: {:?}", e);
        // Returning the error allows the application to exit with a non-zero status code.
        return Err(e);
    }

    info!("Consumer service shut down gracefully.");
    
    Ok(())
}