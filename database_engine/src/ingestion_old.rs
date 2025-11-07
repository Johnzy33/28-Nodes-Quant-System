use log::{info, error};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer}, 
    Message,
};
use deadpool_postgres::Pool;
use shared_models::{market_data::MarketData, time_utils};
use crate::config::ConsumerConfig;
use anyhow::{Context, Result}; 
use std::time::Duration; 
use futures::StreamExt; // CRITICAL: Required for StreamConsumer.stream().next().await

/// Executes a batch of market data inserts within a single database transaction.
/// NOTE: In this debug version, 'batch' will typically contain only ONE message.
async fn execute_batch(pool: &Pool, sql_stmt: &str, batch: &[MarketData]) -> Result<()> {
    
    // --- STEP 1: PRE-CHECK TIMESTAMP CONVERSION ---
    let mut db_rows = Vec::with_capacity(batch.len());
    for data in batch {
        // NOTE: If you are using chrono::DateTime<Utc>, ensure you have the "chrono" feature 
        // enabled on deadpool-postgres and "with-chrono-0_4" on tokio-postgres.
        let timestamp = time_utils::ts_to_utc_datetime(data.ts)
            .context(format!("Failed to convert timestamp {} for asset {}", data.ts, data.asset_id))?;
        
        // Assuming open, high, low, close, volume are all f64 (Double Precision in Postgres)
        db_rows.push((timestamp, data.asset_id.clone(), data.open, data.high, data.low, data.close, data.volume));
    }

    let mut client = pool.get().await.context("Failed to get DB client from pool")?;
    
    // NOTE: Transaction is essential for guaranteed inserts/rollback
    let transaction = client.transaction().await.context("Failed to begin transaction")?;
    let statement = transaction.prepare(sql_stmt).await.context("Failed to prepare statement")?;

    for (timestamp, asset_id, open, high, low, close, volume) in db_rows {
        // CRITICAL: Database execution happens here.
        // This execution does NOT use the complex Box<dyn ToSql + Send + Sync> logic 
        // because it is within a simple `for` loop, which simplifies lifetime/thread requirements.
        transaction.execute(
            &statement,
            &[&timestamp, &asset_id, &open, &high, &low, &close, &volume],
        ).await.context("Failed to execute single insert statement")?;
    }

    transaction.commit().await
        .context("Failed to commit transaction. Data might be invalid or connection lost.")?;
        
    info!("Successfully inserted {} record(s) into market_data", batch.len());
    Ok(())
}


/// Main asynchronous consumer loop for Kafka to TimescaleDB ingestion.
pub async fn run_consumer(config: ConsumerConfig, pool: Pool) -> Result<()> {
    
    // Dynamic Asset ID Derivation from Topic
    let asset_symbol = config.kafka_topic
        .split('_')
        .next()
        .context("Invalid Kafka topic format")?
        .to_uppercase();
    // Assuming the assets table has "assets:US2000:FundedNext"
    let injected_asset_id = format!("assets:{}:{}", asset_symbol, "FundedNext"); 
    
    // ClientConfig setup with robust settings
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id","final_batch_run_1")
        .set("bootstrap.servers", "127.0.0.1:9092")
        .set("security.protocol", "plaintext") // Ensure no unexpected SSL requirement
        .set("metadata.request.timeout.ms", "10000") // CRITICAL FIX: Correct property name
        .set("queued.max.messages.kbytes", "1048576") 
        // IMPORTANT: We use auto-commit here since we're not doing complex batch-and-commit logic
        .set("enable.auto.commit", "true") 
        .set("auto.offset.reset", "earliest")
        .create()
        .context("Consumer creation error")?;
        
    consumer.subscribe(&[&config.kafka_topic])
        .context("Failed to subscribe to Kafka topic")?;
    
    info!("Consumer subscribed to topic: {}", config.kafka_topic);

    let sql_stmt = format!("INSERT INTO market_data (time, asset_id, open, high, low, close, volume) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (time, asset_id) DO NOTHING");

    // The core of the consumer logic is now an asynchronous stream
    let mut message_stream = consumer.stream(); 

    // 🛑 CRITICAL: Use async stream iteration (futures::StreamExt)
    while let Some(message_result) = message_stream.next().await {
        
        match message_result {
            Err(e) => {
                // This handles errors from the Kafka client (e.g., connection lost)
                error!("Kafka stream error: {:?}", e);
            },
            Ok(msg) => {
                // Process the received message
                if let Some(payload) = msg.payload() {
                    match serde_json::from_slice::<MarketData>(payload) {
                        Ok(mut market_data) => {
                            market_data.asset_id = injected_asset_id.clone();
                            
                            // 🛑 DEBUG: Pass the single message for immediate insertion
                            let single_message_vec = vec![market_data];
                                
                            match execute_batch(&pool, &sql_stmt, &single_message_vec).await {
                                Ok(_) => println!("DEBUG: SUCCESSFULLY WROTE 1 RECORD TO DB!"),
                                Err(e) => {
                                    // This will now crash the program and print the error if ANY database issue exists
                                    eprintln!("CRITICAL DB WRITE ERROR (Single Insert): {:?}", e);
                                    // When auto-commit is enabled, we rely on the DB failure to stop the consumer.
                                    return Err(e); // Propagate the error immediately
                                }
                            }
                        },
                        Err(e) => {
                            error!("Failed to deserialize payload: {:?} on message: {:?}", e, String::from_utf8_lossy(payload));
                        }
                    }
                }
            },
        }
    }
    
    // This part of the code is only reached if the stream terminates (e.g., broker disconnects)
    Ok(()) 
}