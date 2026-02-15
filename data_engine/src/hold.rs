async fn handle_mt5_ingestion(
    &self,
    mut socket: TcpStream,
    shutdown: CancellationToken,
    state: WatchdogState
) -> Result<(), Box<dyn Error>> {
    info!("✅ MT5 Connected. Initializing Startup Sync...");

    // 1. Initial Handshake & Calibration (Existing logic)
    self.perform_initial_handshake(&mut socket).await?;

    let (config, symbol_map) = self.load_ingestion_context().map_err(|e| e.to_string())?;
    let dbs = setup_database().await.map_err(|e| e.to_string())?;
    let mut batch = Vec::with_capacity(100);
    let mut flush_interval = interval(Duration::from_secs(2));
    
    let mut header = [0u8; 1];

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = flush_interval.tick() => {
                self.flush_batch_if_needed(&mut batch).await?;
            }

            // --- THE TRAFFIC CONTROLLER ---
            read_res = socket.read_exact(&mut header) => {
                match read_res {
                    Ok(_) => {
                        match header[0] {
                            0 => self.process_tick_header(&mut socket, &symbol_map, &config, &state, &mut batch).await?,
                            1 => self.process_bar_header(&mut socket, &symbol_map, &config, &mut batch).await?,
                            3 => self.process_session_header(&mut socket, &symbol_map, &config).await?,
                            4 => self.process_dna_header(&mut socket, dbs.clone()).await?,
                            5 => self.process_negotiation_header(&mut socket, &symbol_map, &config).await?,
                            254 => self.handle_calibration(&mut socket).await?,
                            _ => {
                                warn!("⚠️ Desync: Unknown Header {}", header[0]);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        self.handle_socket_error(e);
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}


async fn handle_mt5_ingestion(
    &self,
    mut socket: TcpStream,
    // ... args ...
) -> Result<(), Box<dyn Error>> {
    
    // Pull your context from the Static OnceCell
    let context = SYMBOL_MAP.get().ok_or("Ingestion Context not initialized")?;

    // ... loop ...
    match header[0] {
        0 => self.process_tick_header(&mut socket, context, &state, &mut batch).await?,
        1 => self.process_bar_header(&mut socket, context, &mut batch).await?,
        // ... etc ...
    }
}


async fn process_bar_header(
    &self,
    socket: &mut TcpStream,
    context: &IngestionContext,
    batch: &mut Vec<MarketData>,
) -> Result<(), Box<dyn Error>> {
    let mut buf = [0u8; 64];
    socket.read_exact(&mut buf).await?;

    if let Ok(bar) = bytemuck::try_from_bytes::<dm::SyncBar>(&buf) {
        let asset_id = self.resolve_asset_id(&bar.asset, context);
        let source_id = SmolStr::from(context.0.data_source_id.clone());
        let offset_ms = self.get_broker_offset() * 1000;

        let bar_time_ms = if bar.time < 10_000_000_000 { bar.time * 1000 } else { bar.time };
        let utc_ts_ms = bar_time_ms - offset_ms;

        let market_data = MarketData {
            ts: utc_ts_ms,
            asset_id: asset_id.clone(),
            open: bar.open, high: bar.high, low: bar.low, close: bar.close,
            volume: bar.volume as f64,
            source: Some(source_id),
            seq: None,
        };

        // Aggregator Seeding Logic
        let now_utc = Utc::now().timestamp_millis();
        if (now_utc - utc_ts_ms).abs() < (5 * 60 * 1000) {
            LIVE_AGGREGATOR.insert(asset_id.clone(), market_data.clone());
        }

        batch.push(market_data);
        SYNCED_ASSETS.insert(asset_id);

        if batch.len() >= 50 {
            self.execute_ingestion_batch(batch).await?;
        }
    }
    Ok(())
}

async fn process_tick_header(
    &self, 
    socket: &mut TcpStream, 
    context: &IngestionContext, // Using your alias type
    state: &WatchdogState,
    batch: &mut Vec<MarketData>
) -> Result<(), Box<dyn Error>> {
    let mut buf = [0u8; 64]; 
    socket.read_exact(&mut buf).await?;

    if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {
        let asset_id = self.resolve_asset_id(&tick.asset, context);
        
        // Use the config part of the context for the source_id
        let source_id = SmolStr::from(context.0.data_source_id.clone());

        if !self.check_session_status(&asset_id) || !Mt5Watchdog::new(state.clone()).is_market_open() {
            return Ok(());
        }

        LAST_LIVE_TS.insert(asset_id.clone(), tick.time_msc);
        let utc_ts_ms = tick.time_msc - (self.get_broker_offset() * 1000);

        let tick_market_data = MarketData {
            ts: utc_ts_ms,
            asset_id: asset_id.clone(),
            open: tick.bid, high: tick.bid, low: tick.bid, close: tick.bid,
            volume: tick.volume as f64,
            source: Some(source_id),
            seq: None,
        };

        state.update();
        if let Some(completed_bar) = self.handle_tick(tick_market_data, tick.bid, tick.ask, tick.volume).await {
            batch.push(completed_bar);
        }
    }
    Ok(())
}

fn resolve_asset_id(&self, asset_bytes: &[u8], context: &IngestionContext) -> SmolStr {
    let name = std::str::from_utf8(asset_bytes).unwrap_or("").trim_matches(char::from(0));
    
    // context.1 is the FxHashMap<SmolStr, SmolStr>
    context.1.get(name)
        .cloned()
        .unwrap_or_else(|| SmolStr::from(format!("assets:{}:{}", name, context.0.data_source_id)))
}

async fn handle_mt5_ingestion(
    &self,
    mut socket: TcpStream,
    shutdown: CancellationToken,
    state: WatchdogState
) -> Result<(), Box<dyn Error>> {
    info!("✅ MT5 Connected. Initializing Startup Sync...");

    // 1. Pull the static context once per connection
    let context = SYMBOL_MAP.get().ok_or("❌ CRITICAL: SYMBOL_MAP OnceCell is empty!")?;

    // 2. Perform startup handshake using the config from context
    self.send_startup_handshake(&mut socket, &context.0).await?;

    // ... (Your existing gap check and timer setup) ...

    let mut header = [0u8; 1];
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = flush_interval.tick() => {
                self.flush_batch_if_needed(&mut batch).await?;
            }
            
            read_res = socket.read_exact(&mut header) => {
                match read_res {
                    Ok(_) => {
                        match header[0] {
                            0 => self.process_tick_header(&mut socket, context, &state, &mut batch).await?,
                            1 => self.process_bar_header(&mut socket, context, &mut batch).await?,
                            3 => self.process_session_header(&mut socket, context).await?,
                            4 => self.process_dna_header(&mut socket).await?, // DNA doesn't need context for resolution usually
                            5 => self.process_negotiation_header(&mut socket, context).await?,
                            254 => self.handle_calibration(&mut socket).await?,
                            _ => {
                                warn!("⚠️ Desync: Unknown Header {}", header[0]);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        self.handle_socket_error(e);
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn initialize_ingestion_system(config_path: &str) -> Result<(), Box<dyn Error>> {
    // 1. Load the raw config from file
    let config = IngestionCoordinatorConfig::load_from_file(config_path)?;
    let data_source_id = config.data_source_id.clone();
    
    // 2. Build the FxHashMap (Symbol -> Asset ID)
    let mut symbol_map = FxHashMap::default();
    for asset_job in &config.assets {
        // Map "XAGUSD" -> "assets:XAGUSD:broker_name"
        let asset_id = SmolStr::from(format!("assets:{}:{}", asset_job.mt5_symbol, data_source_id));
        symbol_map.insert(SmolStr::from(asset_job.mt5_symbol.clone()), asset_id);
    }

    // 3. Initialize the OnceCell
    // This will fail if called a second time, which is good for safety.
    let context: IngestionContext = (Arc::new(config), symbol_map);
    
    SYMBOL_MAP.set(context)
        .map_err(|_| "❌ CRITICAL: SYMBOL_MAP was already initialized!")?;

    info!("🚀 Ingestion Context initialized with {} assets", SYMBOL_MAP.get().unwrap().1.len());
    Ok(())
}

async fn process_dna_header(
    &self, 
    socket: &mut TcpStream, 
    dbs: Surreal<Client> // Passed from handle_mt5_ingestion
) -> Result<(), Box<dyn Error>> {
    // size_of is safe here because of #[repr(C)] and bytemuck
    let mut buf = vec![0u8; std::mem::size_of::<db::AssetInfoPacket>()];
    socket.read_exact(&mut buf).await?;

    // Use bytemuck to cast the bytes to our Packet struct
    let packet: &db::AssetInfoPacket = bytemuck::from_bytes(&buf);
    
    // Map and Save to SurrealDB
    if let Err(e) = register_assets(dbs, *packet).await {
        error!("❌ DNA Registration Failed: {}", e);
    } else {
        let symbol = std::str::from_utf8(&packet.asset)?
            .trim_matches(char::from(0));
        info!("🧬 Asset DNA Updated: {}", symbol);
    }
    
    Ok(())
}


pub fn load_ingestion_context(&self) -> anyhow::Result<&IngestionContext> {
    SYMBOL_MAP.get_or_try_init(|| {
        info!("📂 First connection detected: Loading config.json into memory...");
        
        let config_path = std::env::var("INGESTION_CONFIG_PATH")
            .unwrap_or_else(|_| "config.json".into());
            
        let config_data = IngestionCoordinatorConfig::load_from_file(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
        
        let arc_config = Arc::new(config_data);
        let mut symbol_map = FxHashMap::default();
        
        for asset in &arc_config.assets {
            // Using system_symbol for the ID (Database side) 
            // and mt5_symbol for the Key (Socket side)
            let full_id = SmolStr::from(format!(
                "assets:{}:{}", 
                asset.system_symbol.to_uppercase(), 
                arc_config.data_source_id
            ));

            symbol_map.insert(
                SmolStr::from(asset.mt5_symbol.clone()), 
                full_id
            );
        }

        Ok((arc_config, symbol_map))
    })
}