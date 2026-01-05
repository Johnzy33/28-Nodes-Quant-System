// // ingestion.rs

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
// use crate::data_service::traits::DataServiceBase;
// use crate::{producer_config::{
//     IngestionCoordinatorConfig, SyncCommand},
//     watchdog::WatchdogState,
// }; 
// use shared_models::data_model::DataService;
// use anyhow::{Context, Result}; 
// use std::time::Duration; 
// use futures::StreamExt;
// use std::env;
// use rdkafka::producer::{FutureProducer, FutureRecord};
// use std::collections::{HashSet, HashMap};
// use std::sync::Arc;
// use tokio::sync::Mutex;
// use lazy_static::lazy_static;
// use shared_models::traits::MarketDataHandler; // <--- Import the TRAIT instead of the concrete Orchestrator


// /// Executes a batch of market data inserts using Postgres ON CONFLICT logic.
// async fn execute_batch(
//     pool: &PgPool,
//     batch: &Vec<MarketData>, // Changed to reference, as we deduplicate before calling this
// ) -> Result<u64> {
//     if batch.is_empty() {
//         return Ok(0);
//     }
    
//     // 1. Convert ms timestamps to NaiveDateTime
//     let timestamps: Vec<chrono::DateTime<chrono::Utc>> = batch.iter()
//         .map(|data| {
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
//             b.push_bind(*ts)
//              .push_bind(&data.asset_id)
//              .push_bind(data.open)
//              .push_bind(data.high)
//              .push_bind(data.low)
//              .push_bind(data.close)
//              .push_bind(data.volume);
//         }
//     );

//     // 3. Upsert Logic: Deduplication handled in memory, but ON CONFLICT handles live updates
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

//     Ok(rows_affected)
// }

// pub async fn initialize_producer() -> Result<FutureProducer> {
//     let brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());

//     let producer: FutureProducer = ClientConfig::new()
//         .set("bootstrap.servers", &brokers)
//         .create()
//         .context("Producer creation error")?;

//     Ok(producer)
// }






// pub async fn run_multi_topic_consumer(
//     pool: PgPool, 
//     producer: &FutureProducer,
//     watchdog_state: WatchdogState,
//     orchestrator: Arc<dyn MarketDataHandler>, // <--- Direct tap to the SPG Brain
// ) -> Result<()> {
//     let config_path = env::var("INGESTION_CONFIG_PATH")
//         .context("CRITICAL: Environment variable INGESTION_CONFIG_PATH must be set.")?;
        
//     let coordinator_config = IngestionCoordinatorConfig::load_from_file(&config_path)
//         .context(format!("Failed to load config from: {}", config_path))?;
        
//     let data_service = DataService::new(pool.clone());
//     publish_sync_requests(producer, &coordinator_config, &data_service).await
//         .context("Failed to publish initial sync requests to Kafka")?;

//     let mut topics = Vec::new();
//     let mut asset_id_map = HashMap::new();

//     for asset in coordinator_config.assets.iter().filter(|a| a.enabled) {
//         let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
//         let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), coordinator_config.data_source_id);
        
//         topics.push(topic.clone());
//         asset_id_map.insert(topic, asset_id);
//     }

//     let topic_refs: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();

//     let consumer: StreamConsumer = ClientConfig::new()
//         .set("group.id", "market_ingest_v2") 
//         .set("bootstrap.servers", env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string()))
//         .set("enable.auto.commit", "false")
//         .set("auto.offset.reset", "latest") // Use latest for live context
//         .create()
//         .context("Consumer creation error")?;
        
//     consumer.subscribe(&topic_refs).context("Failed to subscribe to topics")?;
//     info!("Ingestion Engine online. Subscribed to: {:?}", topic_refs);

//     const BATCH_SIZE: usize = 100;
//     let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
//     let mut last_message: Option<OwnedMessage> = None; 
//     let mut message_stream = consumer.stream(); 

//     loop {
//         tokio::select! {
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
//                             market_data.asset_id = asset_id.clone(); 
                            
//                             // --- [ZERO LATENCY TAP] ---
//                             // Update SPG immediately. We don't wait for DB batching.
//                             let orch = Arc::clone(&orchestrator);
//                             let md = market_data.clone();
//                             // tokio::spawn(async move {
//                             //     orch.on_price_update(&md).await;
//                             // });
//                             orch.on_price_update(&md).await;

//                             // --- [DATABASE PERSISTENCE] ---
//                             batch.push(market_data);
//                             last_message = Some(msg.detach());
                            
//                             if batch.len() >= BATCH_SIZE {
//                                 execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
//                             }
//                         },
//                         Err(e) => { error!("Failed to deserialize: {:?}. Raw: {:?}", e, String::from_utf8_lossy(payload)); }
//                     }
//                 }
//             },
            
//             _ = tokio::time::sleep(Duration::from_secs(60)), if !batch.is_empty() => {
//                 info!("Interval reached. Flushing {} records.", batch.len());
//                 execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
//             }
//         }
//     }

//     Ok(()) 
// }



// /// Helper with added In-Memory Deduplication to prevent Postgres "row affected a second time" error.
// async fn execute_and_commit(
//     pool: &PgPool, 
//     consumer: &StreamConsumer, 
//     batch: &mut Vec<MarketData>, 
//     last_msg: &Option<OwnedMessage>,
//     watchdog_state: &WatchdogState
// ) -> Result<()> {
//     if batch.is_empty() { return Ok(()); }

//     // --- DEDUPLICATION LOGIC ---
//     // Postgres fails if one batch contains the same (time, asset_id) twice.
//     // We use a HashMap to keep only the LATEST occurrence of each unique key.
//     let mut dedup_map: HashMap<(chrono::DateTime<chrono::Utc>, String), MarketData> = HashMap::new();
    
//     for item in batch.drain(..) {
//         let ts = time_utils::ts_to_utc_datetime(item.ts)?;
//         let key = (ts, item.asset_id.clone());
//         dedup_map.insert(key, item);
//     }
    
//     // Convert HashMap values back into a Vec for the execute_batch function
//     let unique_batch: Vec<MarketData> = dedup_map.into_values().collect();

//     // Execute the cleaned batch
//     execute_batch(pool, &unique_batch).await?;

//     // --- HEARTBEAT UPDATE ---
//     // Signal to the watchdog that we are successfully receiving and saving data
//     watchdog_state.update();

//     // Commit Offset
//     if let Some(msg) = last_msg {
//         let mut tpl = TopicPartitionList::new();
//         tpl.add_partition_offset(msg.topic(), msg.partition(), rdkafka::Offset::Offset(msg.offset() + 1))?;
//         consumer.commit(&tpl, CommitMode::Async)?;
//     }
//     Ok(())
// }

// lazy_static! {
//     // This tracks synced assets for the duration of the process run
//     static ref SYNCED_ASSETS: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
// }

// pub async fn publish_sync_requests(
//     producer: &FutureProducer,
//     config: &IngestionCoordinatorConfig,
//     data_service: &DataService,
// ) -> Result<()> {
//     // Lock the tracker
//     let mut tracker = SYNCED_ASSETS.lock().await;

//     for asset in &config.assets {
//         if !asset.enabled { continue; }

//         let asset_id = format!("assets:{}:{}", asset.system_symbol, config.data_source_id);

//         // --- CHECK IF ALREADY SYNCED THIS RUN ---
//         if tracker.contains(&asset_id) {
//             info!("⏩ Skipping Sync Request for {} (already requested this session)", asset.system_symbol);
//             continue;
//         }

//         let hwm = data_service.get_market_data_high_watermark(&asset_id).await?
//             .unwrap_or(0); 

//         let sync_cmd = SyncCommand {
//             command: "SYNC".to_string(),
//             system_symbol: asset.system_symbol.clone(),
//             mt5_symbol: asset.mt5_symbol.clone(),
//             start_timestamp_ms: hwm,
//             timeframe: asset.timeframe.clone(),
//         };

//         let payload = serde_json::to_vec(&sync_cmd)?;

//         producer.send(
//             FutureRecord::to("market_control")
//                 .payload(&payload)
//                 .key(&asset.system_symbol),
//             Duration::from_secs(5)
//         ).await.map_err(|(e, _)| e)?;

//        info!("Sent Sync Request for {} from HWM: {}", asset.system_symbol, hwm);
       
//        // --- MARK AS SYNCED ---
//        tracker.insert(asset_id);
//     }

//     Ok(())
// }

// use log::{info,debug, warn,error};
// use rdkafka::{
//     config::ClientConfig,
//     consumer::{Consumer, StreamConsumer, CommitMode}, 
//     Message,
//     TopicPartitionList,
//     message::{OwnedMessage},
// };
// use sqlx::{PgPool, QueryBuilder, Postgres};
// use shared_models::{market_data::MarketData, time_utils};
// use crate::data_service::traits::DataServiceBase;
// use crate::{producer_config::{
//     IngestionCoordinatorConfig, SyncCommand},
//     watchdog::WatchdogState,
// }; 
// use crate::maintenance::MaintenanceService;
// use shared_models::data_model::DataService;
// use anyhow::{Context, Result}; 
// use std::time::Duration; 
// use futures::StreamExt;
// use std::env;
// use rdkafka::producer::{FutureProducer, FutureRecord};
// use clokwerk::{AsyncScheduler, TimeUnits, Interval, Job};
// use std::collections::{HashSet, HashMap};
// use std::sync::Arc;
// use tokio::sync::{Mutex, watch};
// use lazy_static::lazy_static;
// use shared_models::traits::MarketDataHandler;


/// Executes a batch of market data inserts using Postgres ON CONFLICT logic.




// pub async fn run_multi_topic_consumer(
//     pool: PgPool, 
//     producer: &FutureProducer,
//     watchdog_state: WatchdogState,
//     orchestrator: Arc<dyn MarketDataHandler>,
//     sync_tx: watch::Sender<bool>, // Signal to main/orchestrator when sync is done
// ) -> Result<()> {
//     let config_path = env::var("INGESTION_CONFIG_PATH")
//         .context("CRITICAL: Environment variable INGESTION_CONFIG_PATH must be set.")?;
        
//     let coordinator_config = IngestionCoordinatorConfig::load_from_file(&config_path)
//         .context(format!("Failed to load config from: {}", config_path))?;
        
//     let data_service = DataService::new(pool.clone());
//     publish_sync_requests(producer, &coordinator_config, &data_service).await
//         .context("Failed to publish initial sync requests to Kafka")?;

//     let mut topics = Vec::new();
//     let mut asset_id_map = HashMap::new();

//     for asset in coordinator_config.assets.iter().filter(|a| a.enabled) {
//         let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
//         let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), coordinator_config.data_source_id);
        
//         topics.push(topic.clone());
//         asset_id_map.insert(topic, asset_id);
//     }

//     let topic_refs: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();

//     let consumer: StreamConsumer = ClientConfig::new()
//         .set("group.id", "market_ingest_v2") 
//         .set("bootstrap.servers", env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string()))
//         .set("enable.auto.commit", "false")
//         .set("auto.offset.reset", "latest") 
//         .create()
//         .context("Consumer creation error")?;
        
//     consumer.subscribe(&topic_refs).context("Failed to subscribe to topics")?;
//     info!("Ingestion Engine online. Subscribed to: {:?}", topic_refs);

//     const BATCH_SIZE: usize = 100;
//     let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
//     let mut last_message: Option<OwnedMessage> = None; 
//     let mut message_stream = consumer.stream(); 
//     //let mut is_caught_up = false;
//     let mut check_interval = tokio::time::interval(Duration::from_secs(5));
//     let mut is_caught_up = false;

//     loop {
//         tokio::select! {
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
//                             market_data.asset_id = asset_id.clone(); 
                            
//                             // --- [ZERO LATENCY TAP] ---
//                             // Note: SPG internally uses is_live to ignore sync data
//                             orchestrator.on_price_update(&market_data).await;

//                             // --- [DATABASE PERSISTENCE] ---
//                             batch.push(market_data);
//                             last_message = Some(msg.detach());
                            
//                             if batch.len() >= BATCH_SIZE {
//                                 execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
//                             }
//                         },
//                         Err(e) => { error!("Failed to deserialize: {:?}. Raw: {:?}", e, String::from_utf8_lossy(payload)); }
//                     }
//                 }
//             },
            
//             // --- CATCH-UP & FLUSH MONITOR ---
//            _ = check_interval.tick() => {
//                 if !is_caught_up {
//                     // Force a local check
//                     if check_actual_lag(&consumer, &topics).await {
//                         info!("🏁 KAFKA SYNC COMPLETE: Signaling SPG Orchestrator...");
//                         let _ = sync_tx.send(true);
//                         is_caught_up = true;
//                     }
//                 }

//                 // 2. Regular interval flush
//                 if !batch.is_empty() {
//                     debug!("Interval reached. Flushing {} records.", batch.len());
//                     execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
//                 }
//             }
//             // _ = tokio::signal::ctrl_c() => {
//             //     info!("🛑 Shutdown signal received. Preparing for final flush...");
//             //     break; // Exit the loop to hit the cleanup code below
//             // }
//         }
//     }

//     // --- [FORCE FLUSH] ---
//     if !batch.is_empty() {
//         warn!("💾 SHUTDOWN: Force flushing remaining {} records to database...", batch.len());
//         execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
//         info!("✅ Final flush complete.");
//     } else {
//         info!("✅ No pending records to flush. Shutdown clean.");
//     }
//     Ok(()) 
// }

// pub async fn run_multi_topic_consumer(
//     pool: PgPool, 
//     producer: FutureProducer,
//     watchdog_state: WatchdogState,
//     orchestrator: Arc<dyn MarketDataHandler>,
//     sync_tx: watch::Sender<bool>, 
// ) -> Result<()> {
//     let config_path = env::var("INGESTION_CONFIG_PATH")
//         .context("CRITICAL: Environment variable INGESTION_CONFIG_PATH must be set.")?;
        
//     let coordinator_config = Arc::new(IngestionCoordinatorConfig::load_from_file(&config_path)
//         .context(format!("Failed to load config from: {}", config_path))?);
        
//      // 2. Setup Maintenance Service
//     let data_service = DataService::new(pool.clone());
//     let maintenance = Arc::new(crate::maintenance::MaintenanceService::new(
//         Arc::new(producer), 
//         coordinator_config.clone(),
//         data_service
//     ));

//     // 3. Trigger Startup Sync (HWM)
//     maintenance.publish_sync_requests().await?;

//     // 4. Spawn Background Maintenance Loop (Daily/Weekend Heals)
//     let m_clone = maintenance.clone();
//     tokio::spawn(async move {
//         m_clone.run().await;
//     });

//     let mut topics = Vec::new();
//     let mut asset_id_map = HashMap::new();

//     for asset in coordinator_config.assets.iter().filter(|a| a.enabled) {
//         let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
//         let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), coordinator_config.data_source_id);
        
//         topics.push(topic.clone());
//         asset_id_map.insert(topic, asset_id);
//     }

//     let topic_refs: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();

//     let consumer: StreamConsumer = ClientConfig::new()
//         .set("group.id", "market_ingest_v2") 
//         .set("bootstrap.servers", env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string()))
//         .set("enable.auto.commit", "false")
//         .set("auto.offset.reset", "latest") 
//         .create()
//         .context("Consumer creation error")?;
        
//     consumer.subscribe(&topic_refs).context("Failed to subscribe to topics")?;
//     info!("Ingestion Engine online. Subscribed to: {:?}", topic_refs);

//     const BATCH_SIZE: usize = 100;
//     let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
//     let mut last_message: Option<OwnedMessage> = None; 
//     let mut message_stream = consumer.stream(); 
//     let mut check_interval = tokio::time::interval(Duration::from_secs(5));
//     let mut is_caught_up = false;

//     loop {
//         tokio::select! {
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
//                             market_data.asset_id = asset_id.clone(); 
                            
//                             // Tap the orchestrator (SPG)
//                             orchestrator.on_price_update(&market_data).await;

//                             // Buffer for DB Batching
//                             batch.push(market_data);
//                             last_message = Some(msg.detach());
                            
//                             if batch.len() >= BATCH_SIZE {
//                                 execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
//                             }
//                         },
//                         Err(e) => { error!("Failed to deserialize: {:?}. Raw: {:?}", e, String::from_utf8_lossy(payload)); }
//                     }
//                 }
//             },
            
//            _ = check_interval.tick() => {
//                 // Check if Kafka Sync is finished
//                 if !is_caught_up {
//                     if check_actual_lag(&consumer, &topics).await {
//                         info!("🏁 KAFKA SYNC COMPLETE: Signaling SPG Orchestrator...");
//                         let _ = sync_tx.send(true);
//                         is_caught_up = true;
//                     }
//                 }

//                 // Periodic Batch Flush
//                 if !batch.is_empty() {
//                     execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
//                 }
//             }
//         }
//     }

//     // Final Flush on Shutdown
//     if !batch.is_empty() {
//         warn!("💾 SHUTDOWN: Force flushing {} records...", batch.len());
//         execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
//     }
//     Ok(()) 
// }



// async fn check_actual_lag(consumer: &StreamConsumer, topics: &[String]) -> bool {
//     for topic in topics {
//         if let Ok((_low, high)) = consumer.fetch_watermarks(topic, 0, Duration::from_secs(1)) {
//             if high > 0 {
//                 if let Ok(tpl) = consumer.position() {
//                     let current = tpl.find_partition(topic, 0)
//                         .map(|p| match p.offset() {
//                             rdkafka::Offset::Offset(o) => o,
//                             _ => 0,
//                         }).unwrap_or(0);

//                     if current < (high) { 
//                         info!("DEBUG: Topic {} still has lag. Current: {}, High: {}", topic, current, high);
//                         return false; 
//                     }
//                 }
//             }
//         }
//     }
//     true
// }



// lazy_static! {
//     static ref SYNCED_ASSETS: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
// }

// pub async fn publish_sync_requests(
//     producer: &FutureProducer,
//     config: &IngestionCoordinatorConfig,
//     data_service: &DataService,
// ) -> Result<()> {
//     let mut tracker = SYNCED_ASSETS.lock().await;

//     for asset in &config.assets {
//         if !asset.enabled { continue; }
//         let asset_id = format!("assets:{}:{}", asset.system_symbol, config.data_source_id);

//         if tracker.contains(&asset_id) {
//             info!("⏩ Skipping Sync Request for {} (already requested)", asset.system_symbol);
//             continue;
//         }

//         let hwm = data_service.get_market_data_high_watermark(&asset_id).await?
//             .unwrap_or(0); 

//         let sync_cmd = SyncCommand {
//             command: "SYNC".to_string(),
//             system_symbol: asset.system_symbol.clone(),
//             mt5_symbol: asset.mt5_symbol.clone(),
//             start_timestamp_ms: hwm,
//             end_timestamp_ms:None,
//             timeframe: asset.timeframe.clone(),
//         };

//         let payload = serde_json::to_vec(&sync_cmd)?;
//         producer.send(
//             FutureRecord::to("market_control")
//                 .payload(&payload)
//                 .key(&asset.system_symbol),
//             Duration::from_secs(5)
//         ).await.map_err(|(e, _)| e)?;

//        info!("Sent Sync Request for {} from HWM: {}", asset.system_symbol, hwm);
//        tracker.insert(asset_id);
//     }
//     Ok(())
// }

// pub async fn trigger_heal(
//     producer: &FutureProducer,
//     config: &IngestionCoordinatorConfig,
//     lookback_hours: i64,
// ) -> Result<()> {
//     let now = chrono::Utc::now();
//     let start_timestamp_ms = (now - chrono::Duration::hours(lookback_hours)).timestamp_millis();
//     let end_timestamp_ms = now.timestamp_millis();

//     for asset in &config.assets {
//         if !asset.enabled { continue; }

//         let heal_cmd = SyncCommand {
//             command: "HEAL".to_string(),
//             system_symbol: asset.system_symbol.clone(),
//             mt5_symbol: asset.mt5_symbol.clone(),
//             start_timestamp_ms,
//             end_timestamp_ms: Some(end_timestamp_ms),
//             timeframe: asset.timeframe.clone(),
//         };

//         let payload = serde_json::to_vec(&heal_cmd)?;
        
//         match producer.send(
//             FutureRecord::to("market_control")
//                 .payload(&payload)
//                 .key(&asset.system_symbol),
//             Duration::from_secs(5)
//         ).await {
//             Ok(_) => info!("🩹 HEAL request published for {} (window: -{}h)", asset.system_symbol, lookback_hours),
//             Err((e, _)) => error!("❌ Failed to publish HEAL for {}: {}", asset.system_symbol, e),
//         }
//     }
//     Ok(())
// }


 // or wherever your trigger_heal lives






use chrono::Utc;

// pub async fn start_healing_scheduler(
//     producer: Arc<FutureProducer>,
//     config: Arc<IngestionCoordinatorConfig>,
// ) {
//     let mut scheduler = AsyncScheduler::with_tz(Utc);

//     // --- 1. DAILY HEAL (Mon-Fri at 22:05 UTC) ---
//     // Runs shortly after FX/Index market close to fill any daily gaps
//     scheduler.every(Interval::Weekday).at("22:05:00").run({
//         let p = producer.clone();
//         let c = config.clone();
//         move || {
//             let p = p.clone();
//             let c = c.clone();
//             async move {
//                 info!("⏰ Scheduled Task: Starting Daily Self-Heal (24h lookback)");
//                 if let Err(e) = trigger_heal_internal(&p, &c, 24).await {
//                     error!("Daily heal failed: {}", e);
//                 }
//             }
//         }
//     });

//     // --- 2. WEEKEND DEEP HEAL (Saturday at 10:00 UTC) ---
//     // Full sweep of the week while markets are closed
//     scheduler.every(Interval::Saturday).at("10:00:00").run({
//         let p = producer.clone();
//         let c = config.clone();
//         move || {
//             let p = p.clone();
//             let c = c.clone();
//             async move {
//                 info!("⏰ Scheduled Task: Starting Weekend Deep-Heal (168h lookback)");
//                 if let Err(e) = trigger_heal_internal(&p, &c, 168).await {
//                     error!("Weekend heal failed: {}", e);
//                 }
//             }
//         }
//     });

//     // Run the scheduler loop in the background
//     loop {
//         scheduler.run_pending().await;
//         tokio::time::sleep(Duration::from_secs(60)).await;
//     }
// }

// async fn trigger_heal_internal(
//     producer: &FutureProducer,
//     config: &IngestionCoordinatorConfig,
//     lookback_hours: i64,
// ) -> Result<()> {
//     let now = Utc::now();
//     let start_ts = (now - chrono::Duration::hours(lookback_hours)).timestamp_millis();
//     let end_ts = now.timestamp_millis();

//     for asset in &config.assets {
//         if !asset.enabled { continue; }

//         let heal_cmd = SyncCommand {
//             command: "HEAL".to_string(),
//             system_symbol: asset.system_symbol.clone(),
//             mt5_symbol: asset.mt5_symbol.clone(),
//             start_timestamp_ms: start_ts,
//             end_timestamp_ms: Some(end_ts),
//             timeframe: asset.timeframe.clone(),
//         };

//         let payload = serde_json::to_vec(&heal_cmd)?;
//         producer.send(
//             FutureRecord::to("market_control")
//                 .payload(&payload)
//                 .key(&asset.system_symbol),
//             Duration::from_secs(5)
//         ).await.map_err(|(e, _)| e)?;
        
//         info!("🩹 HEAL command sent for {}", asset.system_symbol);
//     }
//     Ok(())
// }


// use rdkafka::consumer::{Consumer, StreamConsumer, CommitMode};
// use rdkafka::config::ClientConfig;
// use rdkafka::message::{Message, OwnedMessage};
// use std::collections::HashMap;
// use std::sync::Arc;
// use tokio::sync::watch;
// use anyhow::{Context, Result};
// use log::{info, error, debug, warn};
// use sqlx::PgPool;

// use crate::watchdog::WatchdogState;
// use shared_models::data_model::DataService;
// use shared_models::market_data::MarketData;
// use shared_models::traits::MarketDataHandler;
// use crate::producer_config::IngestionCoordinatorConfig;

// pub struct IngestionService {
//     consumer: StreamConsumer,
//     pool: PgPool,
//     config: Arc<IngestionCoordinatorConfig>,
//     orchestrator: Arc<dyn MarketDataHandler>,
//     asset_id_map: HashMap<String, String>,
//     topics: Vec<String>,
// }

// impl IngestionService {
//     pub fn new(
//         pool: PgPool,
//         config: Arc<IngestionCoordinatorConfig>,
//         orchestrator: Arc<dyn MarketDataHandler>,
//     ) -> Result<Self> {
//         // 1. Initialize Kafka Consumer
//         let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());
//         let group_id = std::env::var("KAFKA_GROUP_ID").unwrap_or_else(|_| "market_ingest_v2".to_string());

//         let consumer: StreamConsumer = ClientConfig::new()
//             .set("group.id", &group_id)
//             .set("bootstrap.servers", &brokers)
//             .set("enable.auto.commit", "false")
//             .set("auto.offset.reset", "latest")
//             .create()
//             .context("Consumer creation error")?;

//         // 2. Build Asset Maps
//         let mut topics = Vec::new();
//         let mut asset_id_map = HashMap::new();

//         for asset in config.assets.iter().filter(|a| a.enabled) {
//             let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
//             let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), config.data_source_id);
            
//             topics.push(topic.clone());
//             asset_id_map.insert(topic, asset_id);
//         }

//         Ok(Self {
//             consumer,
//             pool,
//             config,
//             orchestrator,
//             asset_id_map,
//             topics,
//         })
//     }

//     pub async fn initialize_producer() -> Result<FutureProducer> {
//         let brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());
//         let producer: FutureProducer = ClientConfig::new()
//             .set("bootstrap.servers", &brokers)
//             .create()
//             .context("Producer creation error")?;
//         Ok(producer)
//     }

//     pub async fn run(self, watchdog_state: WatchdogState, sync_tx: watch::Sender<bool>) -> Result<()> {
//         let topic_refs: Vec<&str> = self.topics.iter().map(|s| s.as_str()).collect();
//         self.consumer.subscribe(&topic_refs).context("Failed to subscribe to topics")?;
        
//         info!("🚀 Ingestion Service Online. Listening to: {:?}", topic_refs);

//         const BATCH_SIZE: usize = 100;
//         let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
//         let mut last_message: Option<OwnedMessage> = None;
//         let mut is_caught_up = false;
//         let mut check_interval = tokio::time::interval(std::time::Duration::from_secs(5));

//         use futures::StreamExt;
//         let mut message_stream = self.consumer.stream();

//         loop {
//             tokio::select! {
//                 message_result = message_stream.next() => {
//                     let msg = match message_result {
//                         Some(Ok(msg)) => msg,
//                         Some(Err(e)) => { error!("Kafka error: {:?}", e); continue; },
//                         None => break,
//                     };
                    
//                     if let Some(payload) = msg.payload() {
//                         let topic_name = msg.topic();
//                         let asset_id = self.asset_id_map.get(topic_name).cloned().unwrap_or_default();

//                         match serde_json::from_slice::<MarketData>(payload) {
//                             Ok(mut market_data) => {
//                                 market_data.asset_id = asset_id;
                                
//                                 // Direct to SPG Engine
//                                 self.orchestrator.on_price_update(&market_data).await;

//                                 // Buffer for DB
//                                 batch.push(market_data);
//                                 last_message = Some(msg.detach());
                                
//                                 if batch.len() >= BATCH_SIZE {
//                                     self.execute_and_commit(&mut batch, &last_message, &watchdog_state).await?;
//                                 }
//                             },
//                             Err(e) => error!("Deserialization error on {}: {:?}", topic_name, e),
//                         }
//                     }
//                 },
                
//                 _ = check_interval.tick() => {
//                     if !is_caught_up {
//                         if crate::ingestion::check_actual_lag(&self.consumer, &self.topics).await {
//                             info!("🏁 KAFKA SYNC COMPLETE");
//                             let _ = sync_tx.send(true);
//                             is_caught_up = true;
//                         }
//                     }

//                     if !batch.is_empty() {
//                         self.execute_and_commit(&mut batch, &last_message, &watchdog_state).await?;
//                     }
//                 }
//             }
//         }

//         // Final cleanup
//         if !batch.is_empty() {
//             self.execute_and_commit(&mut batch, &last_message, &watchdog_state).await?;
//         }
//         Ok(())
//     }



//     async fn execute_batch(
//         &self,
//         pool: &PgPool,
//         batch: &Vec<MarketData>,
//     ) -> Result<u64> {
//         if batch.is_empty() {
//             return Ok(0);
//         }
        
//         let timestamps: Vec<chrono::DateTime<chrono::Utc>> = batch.iter()
//             .map(|data| {
//                 time_utils::ts_to_utc_datetime(data.ts)
//                     .context(format!("Failed to convert timestamp {} for asset {}", data.ts, data.asset_id))
//             })
//             .collect::<Result<Vec<_>>>()?;
        
//         let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
//             "INSERT INTO market_data (time, asset_id, open, high, low, close, volume) "
//         );

//         query_builder.push_values(
//             batch.iter().zip(timestamps.iter()),
//             |mut b, (data, ts)| {
//                 b.push_bind(*ts)
//                 .push_bind(&data.asset_id)
//                 .push_bind(data.open)
//                 .push_bind(data.high)
//                 .push_bind(data.low)
//                 .push_bind(data.close)
//                 .push_bind(data.volume);
//             }
//         );

//         query_builder.push(r#" 
//             ON CONFLICT ("time", asset_id) DO UPDATE SET
//                 open = EXCLUDED.open,
//                 high = EXCLUDED.high,
//                 low = EXCLUDED.low,
//                 close = EXCLUDED.close,
//                 volume = EXCLUDED.volume
//         "#);

//         let rows_affected = query_builder.build()
//             .execute(pool).await
//             .context("Failed to execute batch insert statement")?
//             .rows_affected();

//         info!("Batch processed {} record(s) in Postgres", rows_affected);

//         Ok(rows_affected)
//     }

//     /// Helper with added In-Memory Deduplication to prevent Postgres "row affected a second time" error.
//     async fn execute_and_commit(
//         pool: &PgPool, 
//         consumer: &StreamConsumer, 
//         batch: &mut Vec<MarketData>, 
//         last_msg: &Option<OwnedMessage>,
//         watchdog_state: &WatchdogState
//     ) -> Result<()> {
//         if batch.is_empty() { return Ok(()); }

//         let mut dedup_map: HashMap<(chrono::DateTime<chrono::Utc>, String), MarketData> = HashMap::new();
        
//         for item in batch.drain(..) {
//             if let Ok(ts) = time_utils::ts_to_utc_datetime(item.ts) {
//                 let key = (ts, item.asset_id.clone());
//                 dedup_map.insert(key, item);
//             }
//         }
        
//         let unique_batch: Vec<MarketData> = dedup_map.into_values().collect();
//         execute_batch(pool, &unique_batch).await?;

//         watchdog_state.update();

//         if let Some(msg) = last_msg {
//             let mut tpl = TopicPartitionList::new();
//             tpl.add_partition_offset(msg.topic(), msg.partition(), rdkafka::Offset::Offset(msg.offset() + 1))?;
//             consumer.commit(&tpl, CommitMode::Async)?;
//         }
//         Ok(())
//     }
//     async fn check_actual_lag(consumer: &StreamConsumer, topics: &[String]) -> bool {
//     for topic in topics {
//         match consumer.fetch_watermarks(topic, 0, Duration::from_secs(1)) {
//             Ok((_low, high)) => {
//                 if high == 0 { continue; } // Empty topic is fine

//                 let current = match consumer.position() {
//                     Ok(tpl) => tpl.find_partition(topic, 0)
//                         .map(|p| match p.offset() {
//                             rdkafka::Offset::Offset(o) => o,
//                             _ => -1, // Not set yet
//                         }).unwrap_or(-1),
//                     Err(_) => -1,
//                 };

//                 // If current is -1, it means the consumer hasn't "attached" to a real offset yet.
//                 // In 'latest' mode, this usually means we are at the head.
//                 if current != -1 && current < high {
//                     info!("DEBUG: Syncing {}... [{}/{}]", topic, current, high);
//                     return false;
//                 }
//             }
//             Err(e) => {
//                 error!("DEBUG: Could not fetch watermarks for {}: {:?}", topic, e);
//                 return false; 
//             }
//         }
//     }
//     true
// }
// }

use rdkafka::consumer::{Consumer, StreamConsumer, CommitMode};
use rdkafka::config::ClientConfig;
use rdkafka::message::{Message, OwnedMessage};
use rdkafka::TopicPartitionList;
use rdkafka::producer::FutureProducer;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use anyhow::{Context, Result};
use log::{info, error, debug, warn};
use sqlx::{PgPool, QueryBuilder, Postgres};

use crate::watchdog::WatchdogState;
use shared_models::data_model::DataService;
use shared_models::market_data::MarketData;
use shared_models::traits::MarketDataHandler;
use shared_models::time_utils;
use crate::producer_config::IngestionCoordinatorConfig;

use async_trait::async_trait; 

// pub struct DataIngestionExt {
//     consumer: StreamConsumer,
//     pool: PgPool,
//     config: Arc<IngestionCoordinatorConfig>,
//     orchestrator: Arc<dyn MarketDataHandler>,
//     asset_id_map: HashMap<String, String>,
//     topics: Vec<String>,
// }


// impl DataIngestionExt for  DataService {
//     fn new(
//         pool: PgPool,
//         config: Arc<IngestionCoordinatorConfig>,
//         orchestrator: Arc<dyn MarketDataHandler>,
//     ) -> Result<Self> {
//         let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());
//         let group_id = std::env::var("KAFKA_GROUP_ID").unwrap_or_else(|_| "market_ingest_v2".to_string());

//         let consumer: StreamConsumer = ClientConfig::new()
//             .set("group.id", &group_id)
//             .set("bootstrap.servers", &brokers)
//             .set("enable.auto.commit", "false")
//             .set("auto.offset.reset", "latest")
//             .create()
//             .context("Consumer creation error")?;

//         let mut topics = Vec::new();
//         let mut asset_id_map = HashMap::new();

//         for asset in config.assets.iter().filter(|a| a.enabled) {
//             let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
//             let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), config.data_source_id);
            
//             topics.push(topic.clone());
//             asset_id_map.insert(topic, asset_id);
//         }

//         Ok(Self {
//             consumer,
//             pool,
//             config,
//             orchestrator,
//             asset_id_map,
//             topics,
//         })
//     }

//     /// Static helper to initialize the producer for the Maintenance Service
//     async fn initialize_producer() -> Result<FutureProducer> {
//         let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());
//         let producer: FutureProducer = ClientConfig::new()
//             .set("bootstrap.servers", &brokers)
//             .create()
//             .context("Producer creation error")?;
//         Ok(producer)
//     }

//     async fn run(self, watchdog_state: WatchdogState, sync_tx: watch::Sender<bool>) -> Result<()> {
//         let topic_refs: Vec<&str> = self.topics.iter().map(|s| s.as_str()).collect();
//         self.consumer.subscribe(&topic_refs).context("Failed to subscribe to topics")?;
        
//         info!("🚀 Ingestion Service Online. Listening to: {:?}", topic_refs);

//         const BATCH_SIZE: usize = 100;
//         let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
//         let mut last_message: Option<OwnedMessage> = None;
//         let mut is_caught_up = false;
//         let mut check_interval = tokio::time::interval(Duration::from_secs(5));

//         use futures::StreamExt;
//         let mut message_stream = self.consumer.stream();

//         loop {
//             tokio::select! {
//                 message_result = message_stream.next() => {
//                     let msg = match message_result {
//                         Some(Ok(msg)) => msg,
//                         Some(Err(e)) => { error!("Kafka error: {:?}", e); continue; },
//                         None => break,
//                     };
                    
//                     if let Some(payload) = msg.payload() {
//                         let topic_name = msg.topic();
//                         let asset_id = self.asset_id_map.get(topic_name).cloned().unwrap_or_default();

//                         match serde_json::from_slice::<MarketData>(payload) {
//                             Ok(mut market_data) => {
//                                 market_data.asset_id = asset_id;
//                                 self.orchestrator.on_price_update(&market_data).await;

//                                 batch.push(market_data);
//                                 last_message = Some(msg.detach());
                                
//                                 if batch.len() >= BATCH_SIZE {
//                                     self.execute_and_commit(&mut batch, &last_message, &watchdog_state).await?;
//                                 }
//                             },
//                             Err(e) => error!("Deserialization error on {}: {:?}", topic_name, e),
//                         }
//                     }
//                 },
                
//                 _ = check_interval.tick() => {
//                     if !is_caught_up {
//                         if self.check_actual_lag().await {
//                             info!("🏁 KAFKA SYNC COMPLETE: Switching SPG to Live Mode");
//                             let _ = sync_tx.send(true);
//                             is_caught_up = true;
//                         }
//                     }

//                     if !batch.is_empty() {
//                         self.execute_and_commit(&mut batch, &last_message, &watchdog_state).await?;
//                     }
//                 }
//             }
//         }

//         if !batch.is_empty() {
//             self.execute_and_commit(&mut batch, &last_message, &watchdog_state).await?;
//         }
//         Ok(())
//     }

//     async fn execute_batch(&self, batch: &Vec<MarketData>) -> Result<u64> {
//         let timestamps: Vec<chrono::DateTime<chrono::Utc>> = batch.iter()
//             .map(|data| {
//                 time_utils::ts_to_utc_datetime(data.ts)
//                     .context(format!("Timestamp conversion error for {}", data.asset_id))
//             })
//             .collect::<Result<Vec<_>>>()?;
        
//         let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
//             "INSERT INTO market_data (time, asset_id, open, high, low, close, volume) "
//         );

//         query_builder.push_values(
//             batch.iter().zip(timestamps.iter()),
//             |mut b, (data, ts)| {
//                 b.push_bind(*ts)
//                 .push_bind(&data.asset_id)
//                 .push_bind(data.open)
//                 .push_bind(data.high)
//                 .push_bind(data.low)
//                 .push_bind(data.close)
//                 .push_bind(data.volume);
//             }
//         );

//         query_builder.push(r#" 
//             ON CONFLICT ("time", asset_id) DO UPDATE SET
//                 open = EXCLUDED.open, high = EXCLUDED.high, low = EXCLUDED.low, 
//                 close = EXCLUDED.close, volume = EXCLUDED.volume
//         "#);

//         let rows_affected = query_builder.build()
//             .execute(&self.pool).await
//             .context("Failed to execute batch insert statement")?
//             .rows_affected();

//         debug!("Batch processed {} record(s)", rows_affected);
//         Ok(rows_affected)
//     }

//     async fn execute_and_commit(
//         &self, 
//         batch: &mut Vec<MarketData>, 
//         last_msg: &Option<OwnedMessage>,
//         watchdog_state: &WatchdogState
//     ) -> Result<()> {
//         if batch.is_empty() { return Ok(()); }

//         let mut dedup_map: HashMap<(chrono::DateTime<chrono::Utc>, String), MarketData> = HashMap::new();
//         for item in batch.drain(..) {
//             if let Ok(ts) = time_utils::ts_to_utc_datetime(item.ts) {
//                 dedup_map.insert((ts, item.asset_id.clone()), item);
//             }
//         }
        
//         let unique_batch: Vec<MarketData> = dedup_map.into_values().collect();
//         self.execute_batch(&unique_batch).await?;

//         watchdog_state.update();

//         if let Some(msg) = last_msg {
//             let mut tpl = TopicPartitionList::new();
//             tpl.add_partition_offset(msg.topic(), msg.partition(), rdkafka::Offset::Offset(msg.offset() + 1))?;
//             self.consumer.commit(&tpl, CommitMode::Async)?;
//         }
//         Ok(())
//     }

//     async fn check_actual_lag(&self) -> bool {
//         for topic in &self.topics {
//             match self.consumer.fetch_watermarks(topic, 0, Duration::from_secs(1)) {
//                 Ok((_low, high)) => {
//                     if high == 0 { continue; }
//                     let current = match self.consumer.position() {
//                         Ok(tpl) => tpl.find_partition(topic, 0)
//                             .map(|p| match p.offset() {
//                                 rdkafka::Offset::Offset(o) => o,
//                                 _ => -1,
//                             }).unwrap_or(-1),
//                         Err(_) => -1,
//                     };
//                     if current != -1 && current < high { return false; }
//                 }
//                 Err(e) => { error!("Lag check error for {}: {:?}", topic, e); return false; }
//             }
//         }
//         true
//     }
// }

use crate::traits::DataIngestionExt;
use futures::StreamExt;

#[async_trait]
impl DataIngestionExt for DataService {
    
    fn create_consumer(&self, group_id: &str) -> Result<StreamConsumer> {
        let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());
        ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", &brokers)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .create()
            .context("Failed to create Kafka Consumer")
    }

    async fn run_ingestion(
        &self,
        config: Arc<IngestionCoordinatorConfig>,
        orchestrator: Arc<dyn MarketDataHandler>,
        watchdog_state: WatchdogState,
        sync_tx: watch::Sender<bool>,
    ) -> Result<()> {
        let group_id = std::env::var("KAFKA_GROUP_ID").unwrap_or_else(|_| "market_ingest_v2".to_string());
        let consumer = self.create_consumer(&group_id)?;

        // Build Topic Map
        let mut topics = Vec::new();
        let mut asset_id_map = HashMap::new();
        for asset in config.assets.iter().filter(|a| a.enabled) {
            let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
            let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), config.data_source_id);
            topics.push(topic.clone());
            asset_id_map.insert(topic, asset_id);
        }

        let topic_refs: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();
        consumer.subscribe(&topic_refs)?;

        info!("🚀 DataService Ingestion Extension Online.");

        let mut batch = Vec::with_capacity(100);
        let mut last_message: Option<OwnedMessage> = None;
        // let mut is_caught_up = false;
        let mut check_interval = tokio::time::interval(Duration::from_secs(5));
        let mut message_stream = consumer.stream();

        loop {
            tokio::select! {
                res = message_stream.next() => {
                    
                    let msg = match res { Some(Ok(m)) => m, Some(Err(e)) => { error!("{}", e); continue; } None => break };

                    // info!("📥 Message received on topic: {}", msg.topic());
                    if let Some(payload) = msg.payload() {
                        let asset_id = asset_id_map.get(msg.topic()).cloned().unwrap_or_default();
                        if let Ok(mut data) = serde_json::from_slice::<MarketData>(payload) {
                            data.asset_id = asset_id;
                            orchestrator.on_price_update(&data).await;
                            batch.push(data);
                            last_message = Some(msg.detach());

                            // info!("Received message from Kafka!");
                            
                            if batch.len() >= 100 {
                                self.execute_ingestion_batch(&mut batch, &last_message, &watchdog_state, &consumer).await?;
                            }
                        }
                    }
                }
                _ = check_interval.tick() => {
                    // if !is_caught_up {
                    //     if self.check_actual_lag(&consumer, &topics).await {
                    //         info!("🏁 KAFKA SYNC COMPLETE: Signaling SPG Orchestrator");
                    //         let _ = sync_tx.send(true);
                    //         is_caught_up = true;
                    //     }
                    // }

                    if !batch.is_empty() {
                        debug!("Interval reached. Flushing {} records to Postgres.", batch.len());
                        self.execute_ingestion_batch(&mut batch, &last_message, &watchdog_state, &consumer).await?;
                    }
                }
            }
        }
        Ok(())
    }


    // async fn check_actual_lag(&self, consumer: &StreamConsumer, topics: &[String]) -> bool {
    //     for topic in topics {
    //         match consumer.fetch_watermarks(topic, 0, Duration::from_secs(1)) {
    //             Ok((_, high)) => {
    //                 if high == 0 { continue; }
    //                 let current = consumer.position().ok().and_then(|tpl| {
    //                     tpl.find_partition(topic, 0).and_then(|p| match p.offset() {
    //                         rdkafka::Offset::Offset(o) => Some(o),
    //                         _ => None,
    //                     })
    //                 }).unwrap_or(-1);
    //                 if current != -1 && current < high { return false; }
    //             }
    //             Err(e) => { error!("Lag check error for {}: {:?}", topic, e); return false; }
    //         }
    //     }
    //     true
    // }

    async fn check_actual_lag(&self, consumer: &StreamConsumer, topics: &[String]) -> bool {
        for topic in topics {
            if let Ok((_low, high)) = consumer.fetch_watermarks(topic, 0, Duration::from_secs(1)) {
                // If high is 0, Python hasn't sent anything yet. Don't finish sync.
                if high == 0 { return false; } 
                
                let current = consumer.position().ok().and_then(|tpl| {
                    tpl.find_partition(topic, 0).and_then(|p| match p.offset() {
                        rdkafka::Offset::Offset(o) => Some(o),
                        _ => None,
                    })
                }).unwrap_or(-1);

                if current < high { return false; }
            }
        }
        true
    }

    async fn execute_ingestion_batch(
        &self,
        batch: &mut Vec<MarketData>,
        last_msg: &Option<OwnedMessage>,
        watchdog_state: &WatchdogState,
        consumer: &StreamConsumer,
    ) -> Result<()> {
        if batch.is_empty() { return Ok(()); }

        // 1. Deduplicate
        let mut dedup_map = HashMap::new();
        for item in batch.drain(..) {
            if let Ok(ts) = time_utils::ts_to_utc_datetime(item.ts) {
                dedup_map.insert((ts, item.asset_id.clone()), item);
            }
        }
        let unique_batch: Vec<MarketData> = dedup_map.into_values().collect();

        // 2. Insert into DB (Self.pool is available here because it's an impl for DataService)
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO market_data (time, asset_id, open, high, low, close, volume) "
        );
        
       
        query_builder.push_values(&unique_batch, |mut b, data| {
            let ts = time_utils::ts_to_utc_datetime(data.ts).unwrap();
            b.push_bind(ts)
             .push_bind(&data.asset_id)
             .push_bind(data.open)
             .push_bind(data.high)
             .push_bind(data.low)
             .push_bind(data.close)
             .push_bind(data.volume);
        });

        query_builder.push(r#" 
            ON CONFLICT ("time", asset_id) DO UPDATE SET
                open = EXCLUDED.open, high = EXCLUDED.high, low = EXCLUDED.low,
                close = EXCLUDED.close, volume = EXCLUDED.volume
        "#);

        let rows_affected = query_builder.build()
            .execute(&self.pool).await
            .context("Failed to execute batch insert in DataService")?
            .rows_affected();
        info!("Batch processed {} record(s) in Postgres", rows_affected);

        // 3. Commit Kafka Offsets & Heartbeat Watchdog
        if let Some(msg) = last_msg {
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(msg.topic(), msg.partition(), rdkafka::Offset::Offset(msg.offset() + 1))?;
            consumer.commit(&tpl, CommitMode::Async)?;
        }
        
        watchdog_state.update();
        Ok(())
    }
}