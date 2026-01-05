

pub async fn run_multi_topic_consumer(
    pool: PgPool, 
    producer: &FutureProducer,
    watchdog_state: WatchdogState,
    orchestrator: Arc<SpgOrchestrator>, // Shared SPG State
) -> Result<()> {
    // ... (Your existing setup logic: config loading, topic map) ...

    let mut message_stream = consumer.stream(); 
    const BATCH_SIZE: usize = 100;
    let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
    let mut last_message: Option<OwnedMessage> = None;

    loop {
        tokio::select! {
            message_result = message_stream.next() => {
                let msg = match message_result {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => { error!("Kafka error: {:?}", e); continue; },
                    None => break,
                };
                
                if let Some(payload) = msg.payload() {
                    if let Ok(mut market_data) = serde_json::from_slice::<MarketData>(payload) {
                        let asset_id = asset_id_map.get(msg.topic()).cloned().unwrap_or_default();
                        market_data.asset_id = asset_id.clone();

                        // --- THE DIRECT TAP: 0ms LATENCY ---
                        // We update the SPG BEFORE we even think about the database.
                        let orch_clone = Arc::clone(&orchestrator);
                        let md_clone = market_data.clone();
                        
                        // Process logic in the background so we don't block the Kafka stream
                        tokio::spawn(async move {
                            orch_clone.on_price_update(&md_clone.asset_id, md_clone.close, md_clone.ts).await;
                        });

                        // --- DATABASE BATCHING (Lower Priority) ---
                        batch.push(market_data);
                        last_message = Some(msg.detach());
                        
                        if batch.len() >= BATCH_SIZE {
                            execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
                        }
                    }
                }
            },
            // Flush timer...
        }
    }
}


pub async fn run_multi_topic_consumer(
    pool: PgPool, 
    producer: &FutureProducer,
    watchdog_state: WatchdogState,
    orchestrator: Arc<SpgOrchestrator>, // <--- Direct tap to the SPG Brain
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
        .set("auto.offset.reset", "latest") // Use latest for live context
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
                            market_data.asset_id = asset_id.clone(); 
                            
                            // --- [ZERO LATENCY TAP] ---
                            // Update SPG immediately. We don't wait for DB batching.
                            let orch = Arc::clone(&orchestrator);
                            let md = market_data.clone();
                            tokio::spawn(async move {
                                orch.on_price_update(&md).await;
                            });

                            // --- [DATABASE PERSISTENCE] ---
                            batch.push(market_data);
                            last_message = Some(msg.detach());
                            
                            if batch.len() >= BATCH_SIZE {
                                execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
                            }
                        },
                        Err(e) => { error!("Failed to deserialize: {:?}. Raw: {:?}", e, String::from_utf8_lossy(payload)); }
                    }
                }
            },
            
            _ = tokio::time::sleep(Duration::from_secs(60)), if !batch.is_empty() => {
                info!("Interval reached. Flushing {} records.", batch.len());
                execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
            }
        }
    }

    Ok(()) 
}