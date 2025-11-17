use log::{info, error};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer, CommitMode}, 
    Message,
    TopicPartitionList,
    message::BorrowedMessage,
};
use deadpool_postgres::Pool;
use shared_models::{market_data::MarketData, time_utils};
use crate::producer_config::{IngestionCoordinatorConfig, ProducerConfig}; // Added ProducerConfig just in case
use anyhow::{Context, Result}; 
use std::time::Duration; 
use futures::StreamExt;
use tokio_postgres::types::{ToSql, Type}; 
use deadpool_postgres::Client;
use std::env;
use std::collections::HashMap; // Used for fast lookup of Topic -> Asset ID

// --- BATCH INSERT IMPLEMENTATION ---

/// Executes a batch of market data inserts within a single database transaction.
async fn execute_batch(client: &mut Client, batch: &mut Vec<MarketData>, sql_template: &str) -> Result<u64> {
    
    if batch.is_empty() {
        return Ok(0);
    }
    
    // 1. Build the multi-row INSERT statement and parameter list
    let batch_size = batch.len();
    let mut sql = String::from(sql_template);
    
    let mut params: Vec<Box<dyn ToSql + Sync + Send>> = Vec::new(); 

    let transaction = client.transaction().await.context("Failed to begin transaction")?;

    // Append ($1, $2, ..., $7), ($8, $9, ..., $14), ...
    for (i, data) in batch.iter().enumerate() {
        let offset = i * 7;
        sql.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${})",
            offset + 1, offset + 2, offset + 3, offset + 4, offset + 5, offset + 6, offset + 7
        ));
        
        if i < batch_size - 1 {
            sql.push_str(", ");
        }

        // Prepare the parameters
        let timestamp = time_utils::ts_to_utc_datetime(data.ts)
            .context(format!("Failed to convert timestamp {} for asset {}", data.ts, data.asset_id))?;
            
        params.push(Box::new(timestamp));
        params.push(Box::new(data.asset_id.clone()));
        params.push(Box::new(data.open));
        params.push(Box::new(data.high));
        params.push(Box::new(data.low));
        params.push(Box::new(data.close));
        params.push(Box::new(data.volume));
    }

    // Append ON CONFLICT DO UPDATE SET
    sql.push_str(r#" 
        ON CONFLICT ("time", asset_id) DO UPDATE SET
            open = EXCLUDED.open,
            high = EXCLUDED.high,
            low = EXCLUDED.low,
            close = EXCLUDED.close,
            volume = EXCLUDED.volume
    "#);

    // 2. Prepare the statement with dynamic parameter types
    let param_types = vec![
        Type::TIMESTAMPTZ, Type::TEXT, Type::FLOAT8, Type::FLOAT8, 
        Type::FLOAT8, Type::FLOAT8, Type::FLOAT8 
    ].into_iter().cycle().take(batch_size * 7).collect::<Vec<_>>();
    
    let statement = transaction.prepare_typed(&sql, &param_types).await
        .context("Failed to prepare multi-row INSERT statement")?;

    // 3. Execute and Commit
    let references: Vec<&(dyn ToSql + Sync + Send)> = params.iter().map(|b| b.as_ref()).collect();
    let final_references: Vec<&(dyn ToSql + Sync)> = references.iter()
        .map(|r| *r as &(dyn ToSql + Sync))
        .collect();
    
    let rows_affected = transaction.execute(&statement, final_references.as_slice()).await
        .context("Failed to execute batch insert statement")?;

    transaction.commit().await
        .context("Failed to commit transaction. Data might be invalid or connection lost.")?;
        
    info!("Batch processed {} record(s) (inserts or updates)", rows_affected);
    
    batch.clear();

    Ok(rows_affected)
}

/// Commits the Kafka offset asynchronously.
async fn commit_offset(consumer: &StreamConsumer, last_message: &BorrowedMessage<'_>) -> Result<()> {
    let mut tpl = TopicPartitionList::new();
    tpl.add_partition_offset(
        last_message.topic(),
        last_message.partition(),
        rdkafka::Offset::Offset(last_message.offset() + 1), 
    )?;

    consumer.commit(&tpl, CommitMode::Async)
        .context("Failed to commit Kafka offset")?;
        
    Ok(())
}


// --- MAIN MULTI-TOPIC CONSUMER LOOP ---

/// Main asynchronous consumer loop for Kafka to TimescaleDB ingestion.
/// It subscribes to ALL topics defined in the INGESTION_CONFIG_PATH file.
pub async fn run_multi_topic_consumer(pool: Pool) -> Result<()> {
    
    // 1. Load ALL configuration and determine all topics to subscribe to.
    let config_path = env::var("INGESTION_CONFIG_PATH")
        .context("CRITICAL: Environment variable INGESTION_CONFIG_PATH must be set to the config file path.")?;
        
    let coordinator_config = IngestionCoordinatorConfig::load_from_file(&config_path)
        .context(format!("Failed to load config from: {}", config_path))?;
        
    // Build a list of all required topics (Vec<&str>)
    let topic_strings: Vec<String> = coordinator_config.assets
        .iter()
        .map(|job| format!("{}_{}", job.symbol.to_lowercase(), job.topic_base))
        .collect(); // Collect the owned Strings into the new variable

    // Build a list of all required topics (Vec<&str>)
    let topics: Vec<&str> = topic_strings // Now we iterate over the long-lived variable
        .iter()
        .map(|s| s.as_str()) // Get the &str references
        .collect();
        
    // 2. Build the Asset ID Map for fast lookup (Topic name -> Asset ID)
    let asset_id_map: HashMap<String, String> = coordinator_config.assets
        .into_iter()
        .map(|job| {
            let topic = format!("{}_{}", job.symbol.to_lowercase(), job.topic_base);
            let asset_id = format!("assets:{}:{}", job.symbol.to_uppercase(), coordinator_config.data_source_id);
            (topic, asset_id)
        })
        .collect();

    // 3. ClientConfig setup
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "final_batch_run_1") 
        .set("bootstrap.servers", "127.0.0.1:9092")
        .set("security.protocol", "plaintext") 
        .set("metadata.request.timeout.ms", "10000") 
        .set("queued.max.messages.kbytes", "1048576") 
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create()
        .context("Consumer creation error")?;
        
    // 💥 SUBSCRIBE TO ALL TOPICS AT ONCE
    consumer.subscribe(&topics) 
        .context("Failed to subscribe to Kafka topics")?;
    
    info!("Consumer subscribed to topics: {:?}", topics);

    let sql_template = String::from("INSERT INTO market_data (time, asset_id, open, high, low, close, volume) VALUES ");
    const BATCH_SIZE: usize = 100;
    let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
    let mut last_message: Option<BorrowedMessage<'_>> = None; 

    let mut message_stream = consumer.stream(); 
    let mut db_client = pool.get().await.context("Failed to get initial DB client")?;

    loop {
        // Use tokio::select! to handle both new messages and a periodic timeout
        tokio::select! {
            // Case 1: A new message arrives from Kafka
            message_result = message_stream.next() => {
                let msg = match message_result {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => { error!("Kafka stream error: {:?}", e); continue; },
                    None => { info!("Kafka stream terminated. Flushing final batch and shutting down."); break; }
                };
                
                // Process the received message
                if let Some(payload) = msg.payload() {
                    
                    // CRUCIAL: Determine asset ID based on message topic
                    let topic_name = msg.topic();
                    let injected_asset_id = match asset_id_map.get(topic_name) {
                        Some(id) => id.clone(),
                        None => {
                            error!("Received message from unknown topic: {}", topic_name);
                            continue; // Skip this message
                        }
                    };

                    match serde_json::from_slice::<MarketData>(payload) {
                        Ok(mut market_data) => {
                            // INJECT THE CORRECT ASSET ID
                            market_data.asset_id = injected_asset_id; 
                            batch.push(market_data);
                            last_message = Some(msg); 
                            
                            // Check if the batch is full
                            if batch.len() >= BATCH_SIZE {
                                // 🚀 BATCH IS FULL: EXECUTE WRITE & COMMIT
                                match execute_batch(&mut db_client, &mut batch, &sql_template).await {
                                    Ok(rows) => {
                                        println!("DEBUG: BATCH PROCESSED {} RECORDS TO DB!", rows);
                                        if let Some(lm) = last_message.as_ref() { commit_offset(&consumer, lm).await?; }
                                    },
                                    Err(e) => {
                                        error!("CRITICAL DB WRITE ERROR (Batch Insert): {:?}", e);
                                        return Err(e).context("Database batch write failed"); 
                                    }
                                }
                            }
                        },
                        Err(e) => { error!("Failed to deserialize payload: {:?}", e); }
                    }
                }
            },
            
            // Case 2: A timeout occurs (flushing partial batch)
            _ = tokio::time::sleep(Duration::from_secs(5)), if !batch.is_empty() => {
                info!("Timeout reached. Committing partial batch of size {}.", batch.len());
                
                match execute_batch(&mut db_client, &mut batch, &sql_template).await {
                    Ok(rows) => {
                        println!("DEBUG: PARTIAL BATCH PROCESSED {} RECORDS TO DB!", rows);
                        if let Some(lm) = last_message.as_ref() { commit_offset(&consumer, lm).await?; }
                    },
                    Err(e) => {
                        error!("CRITICAL DB WRITE ERROR (Timeout Batch Insert): {:?}", e);
                        return Err(e).context("Database timeout batch write failed");
                    }
                }
            }
        }
    }

    // Final flush of any remaining messages before consumer shutdown
    if !batch.is_empty() {
        info!("Final stream terminated. Flushing final batch of size {}.", batch.len());
        match execute_batch(&mut db_client, &mut batch, &sql_template).await {
            Ok(rows) => {
                println!("DEBUG: FINAL BATCH PROCESSED {} RECORDS TO DB!", rows);
                if let Some(lm) = last_message.as_ref() { commit_offset(&consumer, lm).await?; }
            },
            Err(e) => {
                error!("CRITICAL DB WRITE ERROR (Final Flush): {:?}", e);
                return Err(e).context("Database final flush failed");
            }
        }
    }
    
    Ok(()) 
}

// NOTE: You must update the caller (your consumer application's main.rs)
// to call run_multi_topic_consumer(pool).await;