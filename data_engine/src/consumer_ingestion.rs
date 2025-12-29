use log::{info, error};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer, CommitMode}, 
    Message,
    TopicPartitionList,
    message::{OwnedMessage},
};
use sqlx::{PgPool, QueryBuilder, Postgres};
use shared_models::{market_data::MarketData, time_utils};
use crate::{producer_config::{
    IngestionCoordinatorConfig, SyncCommand},
    DataService,
    watchdog::WatchdogState,
}; 
use anyhow::{Context, Result}; 
use std::time::Duration; 
use futures::StreamExt;
use std::env;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::collections::{HashSet, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex;
use lazy_static::lazy_static;



/// Executes a batch of market data inserts using Postgres ON CONFLICT logic.
async fn execute_batch(
    pool: &PgPool,
    batch: &Vec<MarketData>, // Changed to reference, as we deduplicate before calling this
) -> Result<u64> {
    if batch.is_empty() {
        return Ok(0);
    }
    
    // 1. Convert ms timestamps to NaiveDateTime
    let timestamps: Vec<chrono::DateTime<chrono::Utc>> = batch.iter()
        .map(|data| {
            time_utils::ts_to_utc_datetime(data.ts)
                .context(format!("Failed to convert timestamp {} for asset {}", data.ts, data.asset_id))
        })
        .collect::<Result<Vec<_>>>()?;
    
    // 2. Build the Batch Insert Query
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO market_data (time, asset_id, open, high, low, close, volume) "
    );

    query_builder.push_values(
        batch.iter().zip(timestamps.iter()),
        |mut b, (data, ts)| {
            b.push_bind(*ts)
             .push_bind(&data.asset_id)
             .push_bind(data.open)
             .push_bind(data.high)
             .push_bind(data.low)
             .push_bind(data.close)
             .push_bind(data.volume);
        }
    );

    // 3. Upsert Logic: Deduplication handled in memory, but ON CONFLICT handles live updates
    query_builder.push(r#" 
        ON CONFLICT ("time", asset_id) DO UPDATE SET
            open = EXCLUDED.open,
            high = EXCLUDED.high,
            low = EXCLUDED.low,
            close = EXCLUDED.close,
            volume = EXCLUDED.volume
    "#);

    let rows_affected = query_builder.build()
        .execute(pool).await
        .context("Failed to execute batch insert statement")?
        .rows_affected();

    info!("Batch processed {} record(s) in Postgres", rows_affected);

    Ok(rows_affected)
}

pub async fn initialize_producer() -> Result<FutureProducer> {
    let brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .create()
        .context("Producer creation error")?;

    Ok(producer)
}

pub async fn run_multi_topic_consumer(
    pool: PgPool, 
    producer: &FutureProducer,
    watchdog_state: WatchdogState // Added WatchdogState argument
) -> Result<()> {
    let config_path = env::var("INGESTION_CONFIG_PATH")
        .context("CRITICAL: Environment variable INGESTION_CONFIG_PATH must be set.")?;
        
    let coordinator_config = IngestionCoordinatorConfig::load_from_file(&config_path)
        .context(format!("Failed to load config from: {}", config_path))?;
        
    let data_service = DataService::new(pool.clone());
    publish_sync_requests(producer, &coordinator_config, &data_service).await
        .context("Failed to publish initial sync requests to Kafka")?;

    let mut topics = Vec::new();
    let mut asset_id_map = HashMap::new();

    for asset in coordinator_config.assets.iter().filter(|a| a.enabled) {
        let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
        let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), coordinator_config.data_source_id);
        
        topics.push(topic.clone());
        asset_id_map.insert(topic, asset_id);
    }

    let topic_refs: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();

    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "market_ingest_v2") 
        .set("bootstrap.servers", env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string()))
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create()
        .context("Consumer creation error")?;
        
    consumer.subscribe(&topic_refs).context("Failed to subscribe to topics")?;
    info!("Ingestion Engine online. Subscribed to: {:?}", topic_refs);

    const BATCH_SIZE: usize = 100;
    let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
    let mut last_message: Option<OwnedMessage> = None; 
    let mut message_stream = consumer.stream(); 

    loop {
        tokio::select! {
            message_result = message_stream.next() => {
                let msg = match message_result {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => { error!("Kafka error: {:?}", e); continue; },
                    None => break,
                };
                
                if let Some(payload) = msg.payload() {
                    let topic_name = msg.topic();
                    let asset_id = asset_id_map.get(topic_name).cloned().unwrap_or_default();

                    match serde_json::from_slice::<MarketData>(payload) {
                        Ok(mut market_data) => {
                            market_data.asset_id = asset_id; 
                            batch.push(market_data);
                            last_message = Some(msg.detach());
                            
                            if batch.len() >= BATCH_SIZE {
                                // Pass watchdog_state to update the heartbeat
                                execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
                            }
                        },
                        Err(e) => { error!("Failed to deserialize: {:?}. Raw: {:?}", e, String::from_utf8_lossy(payload)); }
                    }
                }
            },
            
            _ = tokio::time::sleep(Duration::from_secs(60)), if !batch.is_empty() => {
                info!("Interval reached. Flushing {} records.", batch.len());
                // Pass watchdog_state to update the heartbeat
                execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
            }
        }
    }

    Ok(()) 
}

/// Helper with added In-Memory Deduplication to prevent Postgres "row affected a second time" error.
async fn execute_and_commit(
    pool: &PgPool, 
    consumer: &StreamConsumer, 
    batch: &mut Vec<MarketData>, 
    last_msg: &Option<OwnedMessage>,
    watchdog_state: &WatchdogState
) -> Result<()> {
    if batch.is_empty() { return Ok(()); }

    // --- DEDUPLICATION LOGIC ---
    // Postgres fails if one batch contains the same (time, asset_id) twice.
    // We use a HashMap to keep only the LATEST occurrence of each unique key.
    let mut dedup_map: HashMap<(chrono::DateTime<chrono::Utc>, String), MarketData> = HashMap::new();
    
    for item in batch.drain(..) {
        let ts = time_utils::ts_to_utc_datetime(item.ts)?;
        let key = (ts, item.asset_id.clone());
        dedup_map.insert(key, item);
    }
    
    // Convert HashMap values back into a Vec for the execute_batch function
    let unique_batch: Vec<MarketData> = dedup_map.into_values().collect();

    // Execute the cleaned batch
    execute_batch(pool, &unique_batch).await?;

    // --- HEARTBEAT UPDATE ---
    // Signal to the watchdog that we are successfully receiving and saving data
    watchdog_state.update();

    // Commit Offset
    if let Some(msg) = last_msg {
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(msg.topic(), msg.partition(), rdkafka::Offset::Offset(msg.offset() + 1))?;
        consumer.commit(&tpl, CommitMode::Async)?;
    }
    Ok(())
}

lazy_static! {
    // This tracks synced assets for the duration of the process run
    static ref SYNCED_ASSETS: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
}

pub async fn publish_sync_requests(
    producer: &FutureProducer,
    config: &IngestionCoordinatorConfig,
    data_service: &DataService,
) -> Result<()> {
    // Lock the tracker
    let mut tracker = SYNCED_ASSETS.lock().await;

    for asset in &config.assets {
        if !asset.enabled { continue; }

        let asset_id = format!("assets:{}:{}", asset.system_symbol, config.data_source_id);

        // --- CHECK IF ALREADY SYNCED THIS RUN ---
        if tracker.contains(&asset_id) {
            info!("⏩ Skipping Sync Request for {} (already requested this session)", asset.system_symbol);
            continue;
        }

        let hwm = data_service.get_market_data_high_watermark(&asset_id).await?
            .unwrap_or(0); 

        let sync_cmd = SyncCommand {
            command: "SYNC".to_string(),
            system_symbol: asset.system_symbol.clone(),
            mt5_symbol: asset.mt5_symbol.clone(),
            start_timestamp_ms: hwm,
            timeframe: asset.timeframe.clone(),
        };

        let payload = serde_json::to_vec(&sync_cmd)?;

        producer.send(
            FutureRecord::to("market_control")
                .payload(&payload)
                .key(&asset.system_symbol),
            Duration::from_secs(5)
        ).await.map_err(|(e, _)| e)?;

       info!("Sent Sync Request for {} from HWM: {}", asset.system_symbol, hwm);
       
       // --- MARK AS SYNCED ---
       tracker.insert(asset_id);
    }

    Ok(())
}