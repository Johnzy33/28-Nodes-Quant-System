// use log::{info, error};
// use rdkafka::{
//     config::ClientConfig,
//     consumer::{Consumer, StreamConsumer, CommitMode}, 
//     Message,
//     TopicPartitionList,
//     message::{OwnedMessage},
// };
// use sqlx::{PgPool, QueryBuilder, Postgres};
// use shared_models::{market_data::MarketData, time_utils};
// use crate::producer_config::{IngestionCoordinatorConfig, SyncCommand}; 
// use anyhow::{Context, Result}; 
// use std::time::Duration; 
// use futures::StreamExt;
// use std::env;
// use std::collections::HashMap;
// use rdkafka::producer::{FutureProducer, FutureRecord};



// /// Executes a batch of market data inserts within a single database transaction using QueryBuilder.
// // async fn execute_batch(
// //     pool: &PgPool,
// //     batch: &mut Vec<MarketData>,
// // ) -> Result<u64> {
    
// //     if batch.is_empty() {
// //         return Ok(0);
// //     }
    
// //     // 1. Precompute timestamps
// //     let timestamps: Vec<chrono::DateTime<chrono::Utc>> = batch.iter()
// //         .map(|data| {
// //             time_utils::ts_to_utc_datetime(data.ts)
// //                 .context(format!("Failed to convert timestamp {} for asset {}", data.ts, data.asset_id))
// //         })
// //         .collect::<Result<Vec<_>>>()?;
    
// //     // 2. Initialize Builder
// //     let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
// //         "INSERT INTO market_data (time, asset_id, open, high, low, close, volume) "
// //     );

// //     // 3. Push Values (non-fallible closure; timestamps already prepared)
// //     query_builder.push_values(
// //         batch.iter().zip(timestamps.iter()),
// //         |mut b, (data, ts)| {
// //             b.push_bind(ts.clone())
// //              .push_bind(data.asset_id.clone())
// //              .push_bind(data.open)
// //              .push_bind(data.high)
// //              .push_bind(data.low)
// //              .push_bind(data.close)
// //              .push_bind(data.volume);
// //         }
// //     );

// //     // 4. Append ON CONFLICT DO UPDATE SET (Upsert logic)
// //     query_builder.push(r#" 
// //         ON CONFLICT ("time", asset_id) DO UPDATE SET
// //             open = EXCLUDED.open,
// //             high = EXCLUDED.high,
// //             low = EXCLUDED.low,
// //             close = EXCLUDED.close,
// //             volume = EXCLUDED.volume
// //     "#);

// //     // 5. Execute on pool (no explicit transaction) and return affected rows
// //     let rows_affected = query_builder.build()
// //         .execute(pool).await
// //         .context("Failed to execute batch insert statement")?
// //         .rows_affected();

// //     info!("Batch processed {} record(s) (inserts or updates)", rows_affected);
    
// //     batch.clear();

// //     Ok(rows_affected)
// // }


// /// Executes a batch of market data inserts using Postgres ON CONFLICT logic.
// async fn execute_batch(
//     pool: &PgPool,
//     batch: &mut Vec<MarketData>,
// ) -> Result<u64> {
//     if batch.is_empty() {
//         return Ok(0);
//     }
    
//     // 1. Convert ms timestamps to NaiveDateTime for Postgres 'TIMESTAMP' or 'TIMESTAMPTZ'
//     // Since Python already corrected for Athens time, we just treat this as UTC.
//     let timestamps: Vec<chrono::DateTime<chrono::Utc>> = batch.iter()
//         .map(|data| {
//             // ts_to_utc_datetime is perfect here
//             time_utils::ts_to_utc_datetime(data.ts)
//                 .context(format!("Failed to convert timestamp {} for asset {}", data.ts, data.asset_id))
//         })
//         .collect::<Result<Vec<_>>>()?;
    
//     // 2. Build the Batch Insert Query
//     let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
//         "INSERT INTO market_data (time, asset_id, open, high, low, close, volume) "
//     );

//     query_builder.push_values(
//         batch.iter().zip(timestamps.iter()),
//         |mut b, (data, ts)| {
//             b.push_bind(*ts) // Matches "time" column
//              .push_bind(&data.asset_id)
//              .push_bind(data.open)
//              .push_bind(data.high)
//              .push_bind(data.low)
//              .push_bind(data.close)
//              .push_bind(data.volume);
//         }
//     );

//     // 3. Upsert Logic: If (time, asset_id) exists, update the OHLCV values.
//     // This is critical for the "Live Stream" where the current minute candle 
//     // is updated every second until it closes.
//     query_builder.push(r#" 
//         ON CONFLICT ("time", asset_id) DO UPDATE SET
//             open = EXCLUDED.open,
//             high = EXCLUDED.high,
//             low = EXCLUDED.low,
//             close = EXCLUDED.close,
//             volume = EXCLUDED.volume
//     "#);

//     let rows_affected = query_builder.build()
//         .execute(pool).await
//         .context("Failed to execute batch insert statement")?
//         .rows_affected();

//     info!("Batch processed {} record(s) in Postgres", rows_affected);
//     batch.clear();

//     Ok(rows_affected)
// }

// /// Commits the Kafka offset asynchronously using an OwnedMessage.
// async fn commit_offset(
//     consumer: &StreamConsumer, 
//     last_message: &OwnedMessage
// ) -> Result<()> {
//     let mut tpl = TopicPartitionList::new();
//     tpl.add_partition_offset(
//         last_message.topic(),
//         last_message.partition(),
//         rdkafka::Offset::Offset(last_message.offset() + 1), 
//     )?;

//     consumer.commit(&tpl, CommitMode::Async)
//         .context("Failed to commit Kafka offset")?;
        
//     Ok(())
// }




// /// Main asynchronous consumer loop for Kafka to TimescaleDB ingestion.
// /// It subscribes to ALL topics defined in the INGESTION_CONFIG_PATH file.
// // pub async fn run_multi_topic_consumer(pool: PgPool) -> Result<()> {
    
// //     //  Load ALL configuration and determine all topics to subscribe to.
// //     let config_path = env::var("INGESTION_CONFIG_PATH")
// //         .context("CRITICAL: Environment variable INGESTION_CONFIG_PATH must be set to the config file path.")?;
        
// //     let coordinator_config = IngestionCoordinatorConfig::load_from_file(&config_path)
// //         .context(format!("Failed to load config from: {}", config_path))?;
        
// //     // Build a list of all required topics (Vec<String>)
// //     let topic_strings: Vec<String> = coordinator_config.assets
// //         .iter()
// //         .map(|job| format!("{}_{}", job.symbol.to_lowercase(), job.topic_base))
// //         .collect();

// //     // Build a list of all required topics (Vec<&str>)
// //     let topics: Vec<&str> = topic_strings 
// //         .iter()
// //         .map(|s| s.as_str())
// //         .collect();
        
// //     //  Build the Asset ID Map for fast lookup (Topic name -> Asset ID)
// //     let asset_id_map: HashMap<String, String> = coordinator_config.assets
// //         .into_iter()
// //         .map(|job| {
// //             let topic = format!("{}_{}", job.symbol.to_lowercase(), job.topic_base);
// //             let asset_id = format!("assets:{}:{}", job.symbol.to_uppercase(), coordinator_config.data_source_id);
// //             (topic, asset_id)
// //         })
// //         .collect();

// //     //  ClientConfig setup
// //     let consumer: StreamConsumer = ClientConfig::new()
// //         .set("group.id", "final_batch_run_1") 
// //         .set("bootstrap.servers", env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string()))
// //         .set("security.protocol", "plaintext") 
// //         .set("metadata.request.timeout.ms", "10000") 
// //         .set("queued.max.messages.kbytes", "1048576") 
// //         .set("enable.auto.commit", "false")
// //         .set("auto.offset.reset", "earliest")
// //         .create()
// //         .context("Consumer creation error")?;
        
// //     consumer.subscribe(&topics) 
// //         .context("Failed to subscribe to Kafka topics")?;
    
// //     info!("Consumer subscribed to topics: {:?}", topics);

// //     const BATCH_SIZE: usize = 100;
// //     let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
// //     let mut last_message: Option<OwnedMessage> = None; 

// //     let mut message_stream = consumer.stream(); 

// //     loop {
// //         // Use tokio::select! to handle both new messages and a periodic timeout
// //         tokio::select! {
// //             // Case 1: A new message arrives from Kafka
// //             message_result = message_stream.next() => {
// //                 let msg = match message_result {
// //                     Some(Ok(msg)) => msg,
// //                     Some(Err(e)) => { error!("Kafka stream error: {:?}", e); continue; },
// //                     None => { info!("Kafka stream terminated. Flushing final batch and shutting down."); break; }
// //                 };
                
// //                 // Process the received message
// //                 if let Some(payload) = msg.payload() {
                    
// //                     let topic_name = msg.topic();
// //                     let injected_asset_id = match asset_id_map.get(topic_name) {
// //                         Some(id) => id.clone(),
// //                         None => {
// //                             error!("Received message from unknown topic: {}", topic_name);
// //                             continue;
// //                         }
// //                     };

// //                     match serde_json::from_slice::<MarketData>(payload) {
// //                         Ok(mut market_data) => {
// //                             market_data.asset_id = injected_asset_id; 
// //                             batch.push(market_data);
// //                             // detach OwnedMessage so it can be held across await points
// //                             last_message = Some(msg.detach());
                            
// //                             if batch.len() >= BATCH_SIZE {
                               
// //                                 match execute_batch(&pool, &mut batch).await {
// //                                     Ok(rows) => {
// //                                         info!("BATCH PROCESSED {} RECORDS TO DB!", rows);
// //                                         if let Some(ref lm) = last_message { commit_offset(&consumer, lm).await?; }
// //                                     },
// //                                     Err(e) => {
// //                                         error!("CRITICAL DB WRITE ERROR (Batch Insert): {:?}", e);
// //                                         return Err(e).context("Database batch write failed"); 
// //                                     }
// //                                 }
// //                             }
// //                         },
// //                         Err(e) => { error!("Failed to deserialize payload: {:?}", e); }
// //                     }
// //                 }
// //             },
            
// //             // Case 2: A timeout occurs (flushing partial batch)
// //             _ = tokio::time::sleep(Duration::from_secs(5)), if !batch.is_empty() => {
// //                 info!("Timeout reached. Committing partial batch of size {}.", batch.len());
                
// //                 match execute_batch(&pool, &mut batch).await {
// //                     Ok(rows) => {
// //                         info!("PARTIAL BATCH PROCESSED {} RECORDS TO DB!", rows);
// //                         if let Some(ref lm) = last_message { commit_offset(&consumer, lm).await?; }
// //                     },
// //                     Err(e) => {
// //                         error!("CRITICAL DB WRITE ERROR (Timeout Batch Insert): {:?}", e);
// //                         return Err(e).context("Database timeout batch write failed");
// //                     }
// //                 }
// //             }
// //         }
// //     }

// //     // Final flush of any remaining messages before consumer shutdown
// //     if !batch.is_empty() {
// //         info!("Final stream terminated. Flushing final batch of size {}.", batch.len());
// //         match execute_batch(&pool, &mut batch).await {
// //             Ok(rows) => {
// //                 info!("FINAL BATCH PROCESSED {} RECORDS TO DB!", rows);
// //                 if let Some(ref lm) = last_message { commit_offset(&consumer, lm).await?; }
// //             },
// //             Err(e) => {
// //                 error!("CRITICAL DB WRITE ERROR (Final Flush): {:?}", e);
// //                 return Err(e).context("Database final flush failed");
// //             }
// //         }
// //     }
    
// //     Ok(()) 
// // }

// pub async fn run_multi_topic_consumer(pool: PgPool, producer: &FutureProducer) -> Result<()> {
//     // 1. Load Configuration
//     let config_path = env::var("INGESTION_CONFIG_PATH")
//         .context("CRITICAL: Environment variable INGESTION_CONFIG_PATH must be set.")?;
        
//     let coordinator_config = IngestionCoordinatorConfig::load_from_file(&config_path)
//         .context(format!("Failed to load config from: {}", config_path))?;
        
//     // 2. Trigger the SYNC phase (New Step)
//     // This tells the Python worker to fill the gaps from the DB High Water Mark
//     let data_service = DataService::new(pool.clone()); // Assuming you have a wrapper for DB queries
//     publish_sync_requests(producer, &coordinator_config, &data_service).await
//         .context("Failed to publish initial sync requests to Kafka")?;

//     // 3. Build Topic List and Asset ID Map (using new system_symbol field)
//     let mut topics = Vec::new();
//     let mut asset_id_map = HashMap::new();

//     for asset in coordinator_config.assets.iter().filter(|a| a.enabled) {
//         let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
//         let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), coordinator_config.data_source_id);
        
//         topics.push(topic.clone());
//         asset_id_map.insert(topic, asset_id);
//     }

//     let topic_refs: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();

//     // 4. Kafka Consumer Setup
//     let consumer: StreamConsumer = ClientConfig::new()
//         .set("group.id", "market_ingest_v2") 
//         .set("bootstrap.servers", env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string()))
//         .set("enable.auto.commit", "false")
//         .set("auto.offset.reset", "earliest")
//         .create()
//         .context("Consumer creation error")?;
        
//     consumer.subscribe(&topic_refs).context("Failed to subscribe to topics")?;
//     info!("Ingestion Engine online. Subscribed to: {:?}", topic_refs);

//     // 5. Processing Loop
//     const BATCH_SIZE: usize = 100;
//     let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
//     let mut last_message: Option<OwnedMessage> = None; 
//     let mut message_stream = consumer.stream(); 

//     loop {
//         tokio::select! {
//             // New message from Python Worker
//             message_result = message_stream.next() => {
//                 let msg = match message_result {
//                     Some(Ok(msg)) => msg,
//                     Some(Err(e)) => { error!("Kafka error: {:?}", e); continue; },
//                     None => break,
//                 };
                
//                 if let Some(payload) = msg.payload() {
//                     let topic_name = msg.topic();
//                     let asset_id = asset_id_map.get(topic_name).cloned().unwrap_or_default();

//                     match serde_json::from_slice::<MarketData>(payload) {
//                         Ok(mut market_data) => {
//                             // Python sends UTC ms, but we ensure the asset_id is the standardized one
//                             market_data.asset_id = asset_id; 
//                             batch.push(market_data);
//                             last_message = Some(msg.detach());
                            
//                             if batch.len() >= BATCH_SIZE {
//                                 execute_and_commit(&pool, &consumer, &mut batch, &last_message).await?;
//                             }
//                         },
//                         Err(e) => { error!("Failed to deserialize: {:?}. Raw: {:?}", e, String::from_utf8_lossy(payload)); }
//                     }
//                 }
//             },
            
//             // Timeout: Flush partial batches to keep DB current
//             _ = tokio::time::sleep(Duration::from_secs(5)), if !batch.is_empty() => {
//                 info!("Interval reached. Flushing {} records.", batch.len());
//                 execute_and_commit(&pool, &consumer, &mut batch, &last_message).await?;
//             }
//         }
//     }

//     Ok(()) 
// }

// /// Helper to reduce code duplication in the loop
// async fn execute_and_commit(
//     pool: &PgPool, 
//     consumer: &StreamConsumer, 
//     batch: &mut Vec<MarketData>, 
//     last_msg: &Option<OwnedMessage>
// ) -> Result<()> {
//     let rows = execute_batch(pool, batch).await?;
//     if let Some(msg) = last_msg {
//         commit_offset(consumer, msg).await?;
//     }
//     Ok(())
// }

// pub async fn publish_sync_requests(
//     producer: &FutureProducer,
//     config: &IngestionCoordinatorConfig,
//     data_service: &DataService, // Your existing service that queries the DB
// ) -> Result<()> {
    
//     for asset in &config.assets {
//         if !asset.enabled { continue; }

//         // 1. Get the High Water Mark from SurrealDB/Postgres
//         let asset_id = format!("assets:{}:{}", asset.system_symbol, config.data_source_id);
//         let hwm = data_service.get_market_data_high_watermark(&asset_id).await?
//             .unwrap_or(0); // If no data, start from 0 (all history)

//         // 2. Build the Sync Command
//         let sync_cmd = SyncCommand {
//             command: "SYNC".to_string(),
//             system_symbol: asset.system_symbol.clone(),
//             mt5_symbol: asset.mt5_symbol.clone(),
//             start_timestamp_ms: hwm,
//             timeframe: asset.timeframe.clone(),
//         };

//         let payload = serde_json::to_vec(&sync_cmd)?;

//         // 3. Publish to the 'market_control' topic
//         // We use the system_symbol as the key so all commands for one asset 
//         // stay in order within Kafka.
//         producer.send(
//             FutureRecord::to("market_control")
//                 .payload(&payload)
//                 .key(&asset.system_symbol),
//             Duration::from_secs(5)
//         ).await.map_err(|(e, _)| e)?;

//         println!("Sent Sync Request for {} starting from {}", asset.system_symbol, hwm);
//     }

//     Ok(())
// }