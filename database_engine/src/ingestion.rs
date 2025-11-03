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

/// Executes a batch of market data inserts within a single database transaction.
async fn execute_batch(pool: &Pool, sql_stmt: &str, batch: &[MarketData]) -> Result<()> {
    
    // --- STEP 1: PRE-CHECK TIMESTAMP CONVERSION ---
    // Perform timestamp conversion check here to rule out data issue before DB connection
    let mut db_rows = Vec::with_capacity(batch.len());
    for data in batch {
        let timestamp = time_utils::ts_to_utc_datetime(data.ts)
            .context(format!("Failed to convert timestamp {} for asset {}", data.ts, data.asset_id))?;
        db_rows.push((timestamp, data.asset_id.clone(), data.open, data.high, data.low, data.close, data.volume));
    }
    // --- END PRE-CHECK ---

    let mut client = pool.get().await.context("Failed to get DB client from pool")?;
    
    info!("Starting transaction for batch size: {}", batch.len());
    
    let transaction = client.transaction().await.context("Failed to begin transaction")?;
    let statement = transaction.prepare(sql_stmt).await.context("Failed to prepare statement")?;

    for (timestamp, asset_id, open, high, low, close, volume) in db_rows {
        transaction.execute(
            &statement,
            &[&timestamp, &asset_id, &open, &high, &low, &close, &volume],
        ).await.context("Failed to execute batch insert")?;
    }

    // CRITICAL: Log failure if commit fails
    transaction.commit().await
        .context("Failed to commit transaction. Data might be invalid or connection lost.")?;
        
    info!("Successfully inserted {} records into market_data", batch.len());
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
    let injected_asset_id = format!("assets:{}", asset_symbol);
    
    // ClientConfig setup (omitted for brevity)
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", &config.kafka_group_id)
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .set("queued.max.messages.kbytes", "512000") 
        .set("fetch.min.bytes", "1")
        .create()
        .context("Consumer creation error")?;
        
    consumer.subscribe(&[&config.kafka_topic])
        .context("Failed to subscribe to Kafka topic")?;
    
    info!("Consumer subscribed to topic: {}", config.kafka_topic);

    let sql_stmt = format!("INSERT INTO market_data (time, asset_id, open, high, low, close, volume) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (time, asset_id) DO NOTHING");

    let mut batch: Vec<MarketData> = Vec::with_capacity(10);
        
    loop {
        match consumer.recv().await { 
            Err(e) => {
                if let rdkafka::error::KafkaError::NoMessageReceived = e {
                    if !batch.is_empty() { 
                        info!("Timeout detected: Flushing final batch of {} records.", batch.len());
                        if let Err(e) = execute_batch(&pool, &sql_stmt, &batch).await {
                            error!("TimescaleDB partial batch insert failed: {:?}", e);
                            return Err(e);
                        }
                        batch.clear();
                    }
                } else {
                    error!("Kafka error: {:?}", e);
                }
            },
            Ok(msg) => {
                if let Some(payload) = msg.payload() {
                    match serde_json::from_slice::<MarketData>(payload) {
                        Ok(mut market_data) => {
                            market_data.asset_id = injected_asset_id.clone();
                            eprintln!("DEBUG: Received message. TS: {} | Open: {}", market_data.ts, market_data.open);                            
                            batch.push(market_data);
                            
                            if batch.len() >= 10 {
                                if let Err(e) = execute_batch(&pool, &sql_stmt, &batch).await {
                                    error!("TimescaleDB batch insert failed: {:?}", e);
                                    return Err(e); // Exit immediately on insert failure
                                }
                                batch.clear();
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
}
