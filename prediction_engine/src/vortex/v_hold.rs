
use chrono::{DateTime, Utc};
use shared_models::market_classification::MarketType as MarketState;

pub struct StateAnchor {
    pub state: MarketState,      // Enum: Bullish, Bearish, FailedBullish, etc.
    pub raw_p: f64,              // P_all
    pub lift: f64,               // Lift (6m normalized)
    pub initial_score: f64,      // The Anchor Score calculated at T-0
    pub current_conviction: f64, // The live score after decay/match factors
}

pub struct VortexScorer {
    pub asset_id: String,
    pub session_name: String,
    pub anchors: [StateAnchor; 7], // The 7 possible states
    pub decay_k: f64,              // The k-constant based on session duration
    pub start_ts: DateTime<Utc>,
    pub total_duration_ms: i64,    // Total ms expected for this session
    pub primary_state: MarketState, // The state with the highest initial Lift
}

impl VortexScorer {

    pub fn calculate_initial_anchor(raw_p: f64, lift_6m: f64, lift_all: f64) -> f64 {
        // Logic: 6m Lift is the Edge, but we dampen it if it deviates 
        // too wildly from All-time Lift (Safety Anchor)
        let regime_consistency = if lift_all > 0.0 { lift_6m / lift_all } else { 1.0 };
        let norm_lift = VortexScorer::normalize_lift(db_lift_6m);
        let initial_score = (raw_p * 0.4) + (norm_lift * 0.6);

        let consistency_tax = if db_lift_all < 1.0 && db_lift_6m > 1.0 { 0.8 } else { 1.0 };
        let final_anchor_score = initial_score * consistency_tax;
        
        // Final Anchor Formula
        let score = (raw_p * 0.4) + (lift_6m * 0.6);
        
        // Boost if 6m and All-time are in agreement
        if regime_consistency > 1.0 { score * 1.1 } else { score }
    }

    /// Normalizes a raw lift value into a 0.0 - 1.0 "Edge Multiplier"
    pub fn normalize_lift(raw_lift: f64) -> f64 {
        if raw_lift <= 1.0 { return 0.0; } // No edge if lift is 1 or less
        
        // We use a log-based approach to dampen extreme outliers like 12.7
        // ln(raw_lift) / ln(MAX_EXPECTED_LIFT)
        let max_expected = 5.0; 
        let normalized = raw_lift.ln() / max_expected.ln();
        
        // Cap the result at 1.0 (Maximum Conviction)
        normalized.min(1.0).max(0.0)
    }

    pub fn calculate_live_conviction(
        &self, 
        current_ts: DateTime<Utc>, 
        last_high_ts: DateTime<Utc>, 
        last_low_ts: DateTime<Utc>
    ) -> f64 {
        let elapsed_ms = (current_ts - self.start_ts).num_milliseconds() as f64;
        let t = (elapsed_ms / self.total_duration_ms as f64).min(1.0);
        
        // 1. Determine Friction
        let mut friction_multiplier = 1.0;
        let time_since_expansion = match self.primary_state {
            MarketState::Bullish | MarketState::BullishReversal => (current_ts - last_high_ts).num_minutes(),
            MarketState::Bearish | MarketState::BearishReversal => (current_ts - last_low_ts).num_minutes(),
            _ => 0,
        };

        // If price hasn't expanded in 15 mins, double the decay speed
        if time_since_expansion > 15 {
            friction_multiplier = 2.0;
        }

        // 2. Exponential Decay Calculation
        // Formula: Initial * e^(-k * t * friction)
        let decay_factor = (-self.decay_k * t * friction_multiplier).exp();
        
        self.anchors.iter()
            .find(|a| a.state == self.primary_state)
            .map(|a| a.initial_score * decay_factor)
            .unwrap_or(0.0)
    }

    pub fn check_for_inverse_pivot(&self, live_conviction: f64, current_bias: MarketState) -> Option<MarketState> {
        // 1. Threshold for "Loss of Faith" in the Primary Anchor
        let loss_of_faith_threshold = 0.25; 

        // 2. Identify the logical "Enemy" of the Primary Anchor
        let inverse_target = match self.primary_state {
            MarketState::Bullish => MarketState::FailedBullish,
            MarketState::Bearish => MarketState::FailedBearish,
            MarketState::BullishReversal => MarketState::Bearish,
            MarketState::BearishReversal => MarketState::Bullish,
            _ => MarketState::PureIndecision,
        };

        // 3. The Pivot Trigger
        // If conviction is low AND the live price is forming the inverse state
        if live_conviction < loss_of_faith_threshold && current_bias == inverse_target {
            return Some(inverse_target);
        }

        None
    }

}

pub fn update_vortex_intelligence(&mut self, current_price: f64, ts: DateTime<Utc>) {
    // 1. Identify Live Bias (Reality)
    let live_bias = self.classify_bias(self.session.open, current_price);
    let elapsed_pct = self.calculate_elapsed_pct(ts);

    // 2. Update Session-Layer Scoring
    // This applies the Decay (k) and Friction (time since high/low)
    let session_conviction = self.session_scorer.calculate_live_conviction(
        ts, 
        self.session.last_high_ts, 
        self.session.last_low_ts
    );

    // 3. Check for the "Vortex Flip" (Inverse Signal)
    if let Some(new_state) = self.session_scorer.check_for_inverse_pivot(session_conviction, live_bias) {
        self.handle_vortex_flip(new_state); // Trigger Phase 4 Pivot
        return; 
    }

    // 4. Multi-Layer Confluence (Session + Bar)
    // We fetch the Bar conviction score (already decaying on its own clock)
    if let Some(bar_scorer) = &self.bar_scorer {
        let bar_conviction = bar_scorer.calculate_live_conviction(ts, self.bar.last_high_ts, self.bar.last_low_ts);
        
        // Calculate Total Agreement
        let total_score = (session_conviction * 0.6) + (bar_conviction * 0.4);
        
        // 5. Update UI/Dashboard Metadata
        self.dashboard.update_conviction(total_score);
        self.dashboard.set_confluence_state(
            if (session_conviction > 0.5) && (bar_conviction > 0.5) { "🔥 ALIGNED" } 
            else { "⚖️ CONFLICT" }
        );
    }
}

fn handle_vortex_flip(&mut self, new_state: MarketState) {
    info!("🔄 VORTEX FLIP: Rejection of primary anchor. Switching to {:?}", new_state);

    // 1. Snapshot the failure of the old state for ML logging later
    self.log_vortex_event("FAILURE", self.session_scorer.primary_state);

    // 2. Hot-swap the primary state to the Inverse State (e.g., FailedBullish)
    self.session_scorer.primary_state = new_state;

    // 3. Reset the Decay Clock, but use a "Failure-Velocity" Multiplier
    // Failed setups move faster; we give the market LESS time to prove the inverse
    self.session_scorer.start_ts = Utc::now(); 
    self.session_scorer.decay_k *= 1.5; 

    // 4. Update the Dashboard
    self.dashboard.alert_flip(new_state);
}

pub struct VortexManager {
    // Each asset has its own independent scorer and state
    pub assets: HashMap<String, AssetVortexState>,
    pub db: DataService, // Your SurrealDB/Postgres connection
}

pub struct AssetVortexState {
    pub asset_id: String,
    pub session_scorer: Option<VortexScorer>,
    pub bar_scorer: Option<VortexScorer>,
    pub last_price: f64,
    pub dashboard_tx: tokio::sync::broadcast::Sender<VortexPacket>,
}

impl VortexManager {
    pub async fn initialize_asset(&mut self, asset_id: String) {
        // 1. Fetch current PS2, PS1, and CS from your Live Context table
        let context = self.db.get_live_context(&asset_id).await.unwrap();

        // 2. Perform the Phase 1 Pre-fetch (The 7-state distribution)
        let distribution = self.db.fetch_vortex_distribution(
            &asset_id, 
            &context.ps2_bias, 
            &context.ps1_bias, 
            &context.cs_name
        ).await.unwrap();

        // 3. Build the VortexScorer
        let scorer = VortexScorer::new(asset_id.clone(), context.cs_name, distribution);

        // 4. Insert into the live manager
        self.assets.insert(asset_id.clone(), AssetVortexState {
            asset_id,
            session_scorer: Some(scorer),
            bar_scorer: None, // Bars handled similarly
            last_price: 0.0,
            ..Default::default()
        });
    }
}

pub fn on_kafka_tick(&mut self, tick: MarketData) {
    if let Some(state) = self.assets.get_mut(&tick.asset_id) {
        // Update the live price and internal High/Low tracking
        state.update_live_metrics(tick.price, tick.ts);

        // Calculate the Vortex scores across all layers
        if let Some(scorer) = &mut state.session_scorer {
            // Apply Phase 2 (Decay/Friction)
            let score = scorer.calculate_live_conviction(tick.ts, state.last_high_ts, state.last_low_ts);
            
            // Apply Phase 3 (Inverse Pivot Check)
            if let Some(new_state) = scorer.check_for_inverse_pivot(score, state.current_bias) {
                state.handle_vortex_flip(new_state);
            }
        }
        
        // Broadcast the JSON packet to your dashboard
        state.broadcast_update();
    } else {
        // If we haven't seen this asset, initialize it
        self.initialize_asset(tick.asset_id);
    }
}

impl AssetVortexState {
    pub fn classify_current_bias(&self, open: f64, high: f64, low: f64, close: f64) -> MarketState {
        let distance_up = high - open;
        let distance_down = open - low;
        let net_move = close - open;
        
        // Threshold for "Significant" movement (e.g., 2 pips or 0.01%)
        let threshold = self.get_volatility_threshold(); 

        if net_move > threshold {
            // Price is currently UP
            if distance_down > (distance_up * 0.5) {
                // It went down significantly before coming back up
                MarketState::FailedBearish 
            } else {
                MarketState::Bullish
            }
        } else if net_move < -threshold {
            // Price is currently DOWN
            if distance_up > (distance_down * 0.5) {
                // It went up significantly before coming back down
                MarketState::FailedBullish
            } else {
                MarketState::Bearish
            }
        } else {
            // Price is near Open
            MarketState::PureIndecision
        }
    }
}


// ... inside load_vortex_snapshot loop ...

// for (i, row) in rows.iter().enumerate() {
//     let m_type: String = row.get("m_type");
    
//     // 1. Capture Paths and Anchors from the first relevant rows
//     if i == 0 {
//         // Grab Session Context from the first row (Session to Session)
//         snapshot.context.session_path = (row.get("path_p2"), row.get("path_p1"));
//         snapshot.context.current_cs = row.get("anchor_name");
//     }
    
//     // Look for the first 'bar_to_bar' row to grab Bar Context
//     if m_type == "bar_to_bar" && snapshot.context.bar_path.0.is_empty() {
//         snapshot.context.bar_path = (row.get("path_p2"), row.get("path_p1"));
//         snapshot.context.current_cb = row.get("anchor_name");
//     }

//     // 2. Standard Point and State parsing
//     let state_raw: String = row.get("state");
//     let anchor_name: String = row.get("anchor_name");
//     let point = VortexPoint {
//         p_6m: row.get("p_6m"),
//         p_1y: row.get("p_1y"),
//         p_all: row.get("p_all"),
//         lift_6m: row.get("l_6m"),
//         lift_1y: row.get("l_1y"),
//         lift_all: row.get("l_all"),
//         count_6m: row.get("count_6m"),
//         count_1y: row.get("count_1y"),
//         count_all: row.get("count_all"),
//     };

//     // 3. Populate Maps
//     match m_type.as_str() {
//         "session_to_session" => {
//             if let Ok(st) = state_raw.parse::<MarketType>() { snapshot.session_to_session.insert(st, point); }
//         }
//         "session_to_daily" => {
//             if let Ok(st) = state_raw.parse::<MarketType>() { snapshot.session_to_daily.insert(st, point); }
//         }
//         "session_to_bar" => {
//             if let Ok(st) = state_raw.parse::<MarketType>() { snapshot.session_to_bar.insert(st, point); }
//         }
//         "bar_to_bar" => {
//             if let Ok(st) = state_raw.parse::<MarketType>() { snapshot.bar_to_bar.insert(st, point); }
//         }
//         "bar_to_daily" => {
//             if let Ok(st) = state_raw.parse::<MarketType>() { snapshot.bar_to_daily.insert(st, point); }
//         }
//         "session_extreme" => {
//             snapshot.session_extreme.insert(format!("{}:{}", state_raw, anchor_name), point);
//         }
//         "bar_extreme" => {
//             snapshot.bar_extreme.insert(format!("{}:{}", state_raw, anchor_name), point);
//         }
//         _ => {}
//     }
// }

// #[derive(Debug, Clone, Default)]
// pub struct VortexSnapshot {
//     pub asset_id: String,
//     pub anchor_time: chrono::DateTime<chrono::Utc>,
//     pub context: SnapshotContext, // Added this field
    
//     pub session_to_session: HashMap<MarketType, VortexPoint>,
//     pub session_to_daily: HashMap<MarketType, VortexPoint>,
//     pub session_to_bar: HashMap<MarketType, VortexPoint>,
//     pub bar_to_bar: HashMap<MarketType, VortexPoint>,
//     pub bar_to_daily: HashMap<MarketType, VortexPoint>,
//     pub session_extreme: HashMap<String, VortexPoint>,
//     pub bar_extreme: HashMap<String, VortexPoint>,
// }

// #[derive(Debug, Clone, Default)]
// pub struct SnapshotContext {
//     pub session_path: (String, String), 
//     pub bar_path: (String, String),     
//     pub current_cs: String,
//     pub current_cb: String, // Changed to String to match the 'anchor_name' in SQL
// }

impl LiveVortexMonitor {
    pub fn process_tick(
        &mut self, 
        anchor: &VortexAnchor, 
        live_bias: MarketType,
        current_high_ts: DateTime<Utc>,
        current_low_ts: DateTime<Utc>
    ) {
        let now = Utc::now();
        let elapsed_hours = (now - self.last_update).num_milliseconds() as f64 / 3_600_000.0;

        // 1. Dynamic Stagnation Threshold
        // High k (3.5) -> ~15 min threshold
        // Low k (0.8)  -> ~60 min threshold
        let wait_threshold_mins = if self.decay_k >= 2.5 {
            15 // Expansion expected now (Home Session)
        } else if self.decay_k >= 1.5 {
            30 // Steady trend expected
        } else {
            60 // Off-hours drift allowed
        };

        // 2. Apply Penalty based on the dynamic threshold
        let mut stagnation_penalty = 1.0;
        let mins_since_high = (now - current_high_ts).num_minutes();
        let mins_since_low = (now - current_low_ts).num_minutes();

        if anchor.primary_s2s == MarketType::Bullish && mins_since_high > wait_threshold_mins {
            stagnation_penalty = 0.85; 
        } else if anchor.primary_s2s == MarketType::Bearish && mins_since_low > wait_threshold_mins {
            stagnation_penalty = 0.85;
        }

        // 3. Accelerate Decay if stagnating
        let active_k = if stagnation_penalty < 1.0 { self.decay_k * 1.5 } else { self.decay_k };
        
        // ... rest of the scoring logic (Match, Friction, Decay) ...
    }
}


// ------------

#[derive(Debug, Clone)]
pub struct MarketData {
    pub body_ratio: f64,
    pub up_wick_ratio: f64,
    pub lo_wick_ratio: f64,
    pub total_wick_ratio: f64,
    pub is_bullish_candle: bool,
    pub range: f64,
    pub body_size: f64,
}

impl MarketData {
    pub fn new(open: f64, high: f64, low: f64, close: f64) -> Self {
        let range = (high - low).max(1e-10);
        let body_size = (close - open).abs();
        let real_body_high = close.max(open);
        let real_body_low = close.min(open);

        let up_wick_ratio = (high - real_body_high) / range;
        let lo_wick_ratio = (real_body_low - low) / range;

        Self {
            body_ratio: body_size / range,
            up_wick_ratio,
            lo_wick_ratio,
            total_wick_ratio: up_wick_ratio + lo_wick_ratio,
            is_bullish_candle: close > open,
            range,
            body_size,
        }
    }
}

async fn calculate_session_and_context(
    &self, 
    asset_id: &str,
    lookback_hours: Option<i64>
) -> Result<(Vec<dm::ClassifiedSession>, Vec<sm::SessionContextData>)> {

    // 1. Get HWM (High Water Mark)
    let hwm: Option<DateTime<Utc>> = self.get_hwm("session_context", "session_end_ts", asset_id).await?;

    let start_time = if let Some(hours) = lookback_hours {
        Utc::now() - Duration::hours(hours)
    } else {
        hwm.unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
    };
        
    // 2. Fetch raw data with a buffer
    let raw: Vec<dm::RawSessionData> = self.fetch_buffered(
        "SELECT * FROM session_base WHERE asset_id = $1", 
        asset_id, 
        hwm, 
        Duration::hours(180),
        "end_ts" 
    ).await?;

    if raw.is_empty() { return Ok((Vec::new(), Vec::new())); }

    // 3. Map to classified objects using the new MarketData/MarketRatios logic
    let mut classified: Vec<dm::ClassifiedSession> = raw.into_iter().map(|s| {
        // Build the physical data & ratios first
        let ratios = MarketRatios::new(s.open, s.high, s.low, s.close);
        
        // Use the ratios to get the classification
        let session_type = MarketType::get_classification(&ratios);

        dm::ClassifiedSession {
            session_type,
            bias: session_type.inverse(), // The predicted direction
            trading_date: s.trading_date, 
            asset_id: s.asset_id, 
            session_name: s.session_name,
            start_ts: s.start_ts, 
            end_ts: s.end_ts, 
            open: s.open, 
            high: s.high, 
            low: s.low, 
            close: s.close, 
            volume: s.volume, 
            bars: s.bars,
            ratios, // Now stored as part of the session data
        }
    }).collect();

    // 4. Sort by end_ts
    classified.sort_by_key(|s| s.end_ts);

    // 5. Generate Context (Storing the 'inverse' bias for p1 and p2)
    let contexts = generate_context(&classified, |curr, p1, p2| {
        p1.map(|prev1| sm::SessionContextData {
            trading_date: curr.trading_date,
            asset_id: curr.asset_id.clone(),
            session_end_ts: curr.end_ts,
            cs_name: Some(curr.session_name.clone()),
            cs_bias: Some(curr.bias.to_string()), // Using the calculated bias
            ps1_name: Some(prev1.session_name.clone()),
            ps1_bias: Some(prev1.bias.to_string()),
            ps2_name: p2.map(|p| p.session_name.clone()),
            ps2_bias: p2.map(|p| p.bias.to_string()),
        })
    });

    // 6. Filter results for persistence
    let limit = start_time;
    
    Ok((
        classified.into_iter().filter(|s| s.end_ts >= limit).collect(),
        contexts
    ))
}

use clokwerk::{AsyncScheduler, TimeUnits};

pub async fn start_healing_scheduler(producer: FutureProducer, config: IngestionCoordinatorConfig) {
    let mut scheduler = AsyncScheduler::with_tz(chrono::Utc);

    // 1. Daily Heal: Mon-Fri at 22:05 UTC (Market Close + buffer)
    scheduler.every(clokwerk::Interval::Weekday).at("22:05:00").run(move || {
        let prod = producer.clone();
        let cfg = config.clone();
        async move {
            let _ = trigger_heal(&prod, &cfg, 24).await; // Heal last 24 hours
        }
    });

    // 2. Weekend Deep Heal: Saturday at 10:00 AM
    scheduler.every(clokwerk::Interval::Saturday).at("10:00:00").run(move || {
        let prod = producer.clone();
        let cfg = config.clone();
        async move {
            let _ = trigger_heal(&prod, &cfg, 168).await; // Heal last 7 days
        }
    });

    loop {
        scheduler.run_pending().await;
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}


pub async fn run_multi_topic_consumer(
    pool: PgPool, 
    producer: FutureProducer,
    watchdog_state: WatchdogState,
    orchestrator: Arc<dyn MarketDataHandler>,
    sync_tx: watch::Sender<bool>, 
) -> Result<()> {
    let config_path = env::var("INGESTION_CONFIG_PATH")
        .context("CRITICAL: Environment variable INGESTION_CONFIG_PATH must be set.")?;
        
    let coordinator_config = Arc::new(IngestionCoordinatorConfig::load_from_file(&config_path)
        .context(format!("Failed to load config from: {}", config_path))?);
        
    let data_service = DataService::new(pool.clone());
    
    // Wrap producer in Arc to share with the healing scheduler
    let shared_producer = Arc::new(producer);

    // 1. Trigger Initial SYNC requests
    publish_sync_requests(&shared_producer, &coordinator_config, &data_service).await
        .context("Failed to publish initial sync requests to Kafka")?;

    // 2. Spawn the Self-Healing Scheduler
    let s_prod = shared_producer.clone();
    let s_cfg = coordinator_config.clone();
    tokio::spawn(async move {
        crate::scheduler::start_healing_scheduler(s_prod, s_cfg).await;
    });

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
        .set("auto.offset.reset", "latest") 
        .create()
        .context("Consumer creation error")?;
        
    consumer.subscribe(&topic_refs).context("Failed to subscribe to topics")?;
    info!("Ingestion Engine online. Subscribed to: {:?}", topic_refs);

    const BATCH_SIZE: usize = 100;
    let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
    let mut last_message: Option<OwnedMessage> = None; 
    let mut message_stream = consumer.stream(); 
    let mut check_interval = tokio::time::interval(Duration::from_secs(5));
    let mut is_caught_up = false;

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
                            
                            // Tap the orchestrator (SPG)
                            orchestrator.on_price_update(&market_data).await;

                            // Buffer for DB Batching
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
            
           _ = check_interval.tick() => {
                // Check if Kafka Sync is finished
                if !is_caught_up {
                    if check_actual_lag(&consumer, &topics).await {
                        info!("🏁 KAFKA SYNC COMPLETE: Signaling SPG Orchestrator...");
                        let _ = sync_tx.send(true);
                        is_caught_up = true;
                    }
                }

                // Periodic Batch Flush
                if !batch.is_empty() {
                    execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
                }
            }
        }
    }

    // Final Flush on Shutdown
    if !batch.is_empty() {
        warn!("💾 SHUTDOWN: Force flushing {} records...", batch.len());
        execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
    }
    Ok(()) 
}

pub async fn run_multi_topic_consumer(
    pool: PgPool, 
    producer: FutureProducer,
    watchdog_state: WatchdogState,
    orchestrator: Arc<dyn MarketDataHandler>,
    sync_tx: watch::Sender<bool>, 
) -> Result<()> {
    // 1. Load Config
    let config_path = env::var("INGESTION_CONFIG_PATH").context("Config path missing")?;
    let coordinator_config = Arc::new(IngestionCoordinatorConfig::load_from_file(&config_path)?);
    
    // 2. Setup Maintenance Service
    let data_service = DataService::new(pool.clone());
    let maintenance = Arc::new(crate::maintenance::MaintenanceService::new(
        Arc::new(producer), 
        coordinator_config.clone(),
        data_service
    ));

    // 3. Trigger Startup Sync (HWM)
    maintenance.publish_sync_requests().await?;

    // 4. Spawn Background Maintenance Loop (Daily/Weekend Heals)
    let m_clone = maintenance.clone();
    tokio::spawn(async move {
        m_clone.run().await;
    });

    // ... (rest of the Kafka consumer loop) ...


// data_engine/src/data_service/ingestion_ext.rs

#[async_trait]
pub trait DataIngestionExtTrait {
    fn new(
        pool: PgPool,
        config: Arc<IngestionCoordinatorConfig>,
        orchestrator: Arc<dyn MarketDataHandler>,
    ) -> Result<Self> where Self: Sized;

    async fn run(self, watchdog_state: WatchdogState, sync_tx: watch::Sender<bool>) -> Result<()>;
}

pub struct DataIngestionExt {
    consumer: StreamConsumer,
    pool: PgPool,
    config: Arc<IngestionCoordinatorConfig>,
    orchestrator: Arc<dyn MarketDataHandler>,
    asset_id_map: HashMap<String, String>,
    topics: Vec<String>,
}

#[async_trait]
impl DataIngestionExtTrait for DataIngestionExt {
    fn new(
        pool: PgPool,
        config: Arc<IngestionCoordinatorConfig>,
        orchestrator: Arc<dyn MarketDataHandler>,
    ) -> Result<Self> {
        let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());
        let group_id = std::env::var("KAFKA_GROUP_ID").unwrap_or_else(|_| "market_ingest_v2".to_string());

        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &group_id)
            .set("bootstrap.servers", &brokers)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .create()
            .context("Consumer creation error")?;

        let mut topics = Vec::new();
        let mut asset_id_map = HashMap::new();

        for asset in config.assets.iter().filter(|a| a.enabled) {
            let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
            let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), config.data_source_id);
            
            topics.push(topic.clone());
            asset_id_map.insert(topic, asset_id);
        }

        Ok(Self {
            consumer,
            pool,
            config,
            orchestrator,
            asset_id_map,
            topics,
        })
    }

    async fn run(self, watchdog_state: WatchdogState, sync_tx: watch::Sender<bool>) -> Result<()> {
        let topic_refs: Vec<&str> = self.topics.iter().map(|s| s.as_str()).collect();
        self.consumer.subscribe(&topic_refs).context("Failed to subscribe to topics")?;
        
        info!("🚀 Ingestion Extension Online. Listening to: {:?}", topic_refs);

        const BATCH_SIZE: usize = 100;
        let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
        let mut last_message: Option<OwnedMessage> = None;
        let mut is_caught_up = false;
        let mut check_interval = tokio::time::interval(Duration::from_secs(5));

        use futures::StreamExt;
        let mut message_stream = self.consumer.stream();

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
                        let asset_id = self.asset_id_map.get(topic_name).cloned().unwrap_or_default();

                        match serde_json::from_slice::<MarketData>(payload) {
                            Ok(mut market_data) => {
                                market_data.asset_id = asset_id;
                                self.orchestrator.on_price_update(&market_data).await;

                                batch.push(market_data);
                                last_message = Some(msg.detach());
                                
                                if batch.len() >= BATCH_SIZE {
                                    self.execute_and_commit(&mut batch, &last_message, &watchdog_state).await?;
                                }
                            },
                            Err(e) => error!("Deserialization error on {}: {:?}", topic_name, e),
                        }
                    }
                },
                _ = check_interval.tick() => {
                    if !is_caught_up {
                        if self.check_actual_lag().await {
                            info!("🏁 KAFKA SYNC COMPLETE");
                            let _ = sync_tx.send(true);
                            is_caught_up = true;
                        }
                    }
                    if !batch.is_empty() {
                        self.execute_and_commit(&mut batch, &last_message, &watchdog_state).await?;
                    }
                }
            }
        }
        Ok(())
    }
}

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
use sqlx::{Postgres, QueryBuilder};
use chrono::{Utc, Datelike, Timelike};
use std::fs;
use std::path::Path;

// Internal imports based on your project structure
use crate::watchdog::WatchdogState;
use shared_models::market_data::MarketData;
use shared_models::traits::MarketDataHandler;
use shared_models::time_utils;
use crate::producer_config::{IngestionCoordinatorConfig, SyncCommand};

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

        let mut topics = Vec::new();
        let mut asset_id_map = HashMap::new();
        for asset in config.assets.iter().filter(|a| a.enabled) {
            let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
            let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), config.data_source_id);
            topics.push(topic.clone());
            asset_id_map.insert(topic, asset_id);
        }

        let topic_refs: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();
        consumer.subscribe(&topic_refs).context("Subsciption failed")?;

        info!("🚀 Ingestion Extension Online. Monitoring {} topics.", topics.len());

        let mut batch = Vec::with_capacity(100);
        let mut last_message: Option<OwnedMessage> = None;
        let mut is_caught_up = false;
        let mut check_interval = tokio::time::interval(Duration::from_secs(5));

        use futures::StreamExt;
        let mut message_stream = consumer.stream();

        loop {
            tokio::select! {
                res = message_stream.next() => {
                    let msg = match res { 
                        Some(Ok(m)) => m, 
                        Some(Err(e)) => { error!("Kafka Stream Error: {}", e); continue; } 
                        None => break 
                    };

                    if let Some(payload) = msg.payload() {
                        let asset_id = asset_id_map.get(msg.topic()).cloned().unwrap_or_default();
                        if let Ok(mut data) = serde_json::from_slice::<MarketData>(payload) {
                            data.asset_id = asset_id;
                            
                            // Live Price Update to Orchestrator
                            orchestrator.on_price_update(&data).await;

                            batch.push(data);
                            last_message = Some(msg.detach());
                            
                            if batch.len() >= 100 {
                                self.execute_ingestion_batch(&mut batch, &last_message, &watchdog_state, &consumer).await?;
                            }
                        }
                    }
                }
                _ = check_interval.tick() => {
                    if !is_caught_up {
                        if self.check_actual_lag(&consumer, &topics).await {
                            info!("🏁 KAFKA SYNC COMPLETE: Signaling SPG Orchestrator");
                            let _ = sync_tx.send(true);
                            is_caught_up = true;
                        }
                    }
                    if !batch.is_empty() {
                        self.execute_ingestion_batch(&mut batch, &last_message, &watchdog_state, &consumer).await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn execute_ingestion_batch(
        &self,
        batch: &mut Vec<MarketData>,
        last_msg: &Option<OwnedMessage>,
        watchdog_state: &WatchdogState,
        consumer: &StreamConsumer,
    ) -> Result<()> {
        if batch.is_empty() { return Ok(()); }

        let mut dedup_map = HashMap::new();
        for item in batch.drain(..) {
            if let Ok(ts) = time_utils::ts_to_utc_datetime(item.ts) {
                dedup_map.insert((ts, item.asset_id.clone()), item);
            }
        }
        let unique_batch: Vec<MarketData> = dedup_map.into_values().collect();

        // Database UPSERT
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

        query_builder.push(r#" ON CONFLICT ("time", asset_id) DO UPDATE SET 
            open=EXCLUDED.open, high=EXCLUDED.high, low=EXCLUDED.low, 
            close=EXCLUDED.close, volume=EXCLUDED.volume "#);

        query_builder.build().execute(&self.pool).await.context("Batch DB insert failed")?;

        if let Some(msg) = last_msg {
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(msg.topic(), msg.partition(), rdkafka::Offset::Offset(msg.offset() + 1))?;
            consumer.commit(&tpl, CommitMode::Async).context("Kafka commit failed")?;
        }
        
        watchdog_state.update();
        Ok(())
    }

    async fn check_actual_lag(&self, consumer: &StreamConsumer, topics: &[String]) -> bool {
        for topic in topics {
            if let Ok((_, high)) = consumer.fetch_watermarks(topic, 0, Duration::from_secs(1)) {
                if high == 0 { continue; }
                let current = consumer.position().ok().and_then(|tpl| {
                    tpl.find_partition(topic, 0).and_then(|p| match p.offset() {
                        rdkafka::Offset::Offset(o) => Some(o),
                        _ => None,
                    })
                }).unwrap_or(-1);
                if current != -1 && current < high { return false; }
            }
        }
        true
    }
}

#[async_trait]
impl DataMaintenanceExt for DataService {
    async fn publish_startup_sync(
        &self,
        producer: Arc<FutureProducer>,
        config: Arc<IngestionCoordinatorConfig>,
    ) -> Result<()> {
        for asset in &config.assets {
            if !asset.enabled { continue; }
            let asset_id = format!("assets:{}:{}", asset.system_symbol, config.data_source_id);
            
            let hwm = self.get_market_data_high_watermark(&asset_id).await?.unwrap_or(0);

            let cmd = SyncCommand {
                command: "SYNC".to_string(),
                system_symbol: asset.system_symbol.clone(),
                mt5_symbol: asset.mt5_symbol.clone(),
                start_timestamp_ms: hwm,
                end_timestamp_ms: None,
                timeframe: asset.timeframe.clone(),
            };

            let payload = serde_json::to_vec(&cmd)?;
            producer.send(
                rdkafka::producer::FutureRecord::to("market_control").payload(&payload).key(&asset.system_symbol),
                Duration::from_secs(5)
            ).await.map_err(|(e, _)| e)?;
            
            info!("📡 Sent Startup SYNC for {} from HWM: {}", asset.system_symbol, hwm);
        }
        Ok(())
    }

    async fn start_maintenance_loop(
        &self,
        producer: Arc<FutureProducer>,
        config: Arc<IngestionCoordinatorConfig>,
    ) -> Result<()> {
        let base_dir = std::env::var("MARKET_DATA_DIR").unwrap_or_else(|_| ".".to_string());
        let state_path = format!("{}/maintenance_state.json", base_dir);
        let service_copy = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                let now = Utc::now();
                
                // 1. Load State
                let mut state: serde_json::Value = fs::read_to_string(&state_path)
                    .ok().and_then(|c| serde_json::from_str(&c).ok()).unwrap_or(serde_json::json!({}));

                let mut changed = false;

                // 2. Daily Check (22:00)
                if now.hour() >= 22 {
                    let last_daily = state["last_daily"].as_str().and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok());
                    if last_daily.map(|dt| dt.date_naive() < now.date_naive()).unwrap_or(true) {
                        if dispatch_heal(&producer, &config, 24).await.is_ok() {
                            state["last_daily"] = serde_json::json!(now.to_rfc3339());
                            changed = true;
                        }
                    }
                }

                // 3. Weekend Check
                if now.weekday() == chrono::Weekday::Sat || now.weekday() == chrono::Weekday::Sun {
                    let last_week = state["last_weekend"].as_str().and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok());
                    if last_week.map(|dt| (now - dt).num_days() >= 6).unwrap_or(true) {
                        if dispatch_heal(&producer, &config, 168).await.is_ok() {
                            state["last_weekend"] = serde_json::json!(now.to_rfc3339());
                            changed = true;
                        }
                    }
                }

                if changed {
                    let _ = fs::write(&state_path, serde_json::to_string_pretty(&state).unwrap());
                }
            }
        });

        Ok(())
    }
}

// Helper for Maintenance Loop
async fn dispatch_heal(producer: &FutureProducer, config: &IngestionCoordinatorConfig, hours: i64) -> Result<()> {
    let now = Utc::now();
    let start_ms = (now - chrono::Duration::hours(hours)).timestamp_millis();
    for asset in &config.assets {
        if !asset.enabled { continue; }
        let cmd = SyncCommand {
            command: "HEAL".to_string(),
            system_symbol: asset.system_symbol.clone(),
            mt5_symbol: asset.mt5_symbol.clone(),
            start_timestamp_ms: start_ms,
            end_timestamp_ms: Some(now.timestamp_millis()),
            timeframe: asset.timeframe.clone(),
        };
        let payload = serde_json::from_slice::<SyncCommand>(payload); // simplified for example
        // (Kafka send logic here...)
    }
    Ok(())
}}

// -----------

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

        let mut topics = Vec::new();
        let mut asset_id_map = HashMap::new();
        for asset in config.assets.iter().filter(|a| a.enabled) {
            let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
            let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), config.data_source_id);
            topics.push(topic.clone());
            asset_id_map.insert(topic, asset_id);
        }

        let topic_refs: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();
        consumer.subscribe(&topic_refs).context("Failed to subscribe to topics")?;

        info!("🚀 DataService Ingestion Extension Online. Monitoring: {:?}", topics);

        const BATCH_SIZE: usize = 100;
        let mut batch = Vec::with_capacity(BATCH_SIZE);
        let mut last_message: Option<OwnedMessage> = None;
        let mut is_caught_up = false;
        let mut check_interval = tokio::time::interval(Duration::from_secs(5));
        
        use futures::StreamExt;
        let mut message_stream = consumer.stream();

        loop {
            tokio::select! {
                res = message_stream.next() => {
                    let msg = match res { 
                        Some(Ok(m)) => m, 
                        Some(Err(e)) => { error!("Kafka error: {:?}", e); continue; } 
                        None => break 
                    };

                    if let Some(payload) = msg.payload() {
                        let topic_name = msg.topic();
                        let asset_id = asset_id_map.get(topic_name).cloned().unwrap_or_default();

                        match serde_json::from_slice::<MarketData>(payload) {
                            Ok(mut market_data) => {
                                market_data.asset_id = asset_id;

                                // --- [ZERO LATENCY TAP] ---
                                // We send to the orchestrator IMMEDIATELY before batching for DB
                                orchestrator.on_price_update(&market_data).await;

                                // --- [DATABASE PERSISTENCE BUFFER] ---
                                batch.push(market_data);
                                last_message = Some(msg.detach());
                                
                                if batch.len() >= BATCH_SIZE {
                                    self.execute_ingestion_batch(&mut batch, &last_message, &watchdog_state, &consumer).await?;
                                }
                            },
                            Err(e) => error!("Deserialization error on {}: {:?}", topic_name, e),
                        }
                    }
                }
                _ = check_interval.tick() => {
                    if !is_caught_up {
                        if self.check_actual_lag(&consumer, &topics).await {
                            info!("🏁 KAFKA SYNC COMPLETE: Signaling SPG Orchestrator");
                            let _ = sync_tx.send(true);
                            is_caught_up = true;
                        }
                    }

                    if !batch.is_empty() {
                        debug!("Interval reached. Flushing {} records to Postgres.", batch.len());
                        self.execute_ingestion_batch(&mut batch, &last_message, &watchdog_state, &consumer).await?;
                    }
                }
            }
        }

        // Final shutdown flush
        if !batch.is_empty() {
            warn!("💾 SHUTDOWN: Final flush of {} records.", batch.len());
            self.execute_ingestion_batch(&mut batch, &last_message, &watchdog_state, &consumer).await?;
        }
        
        Ok(())
    }

    async fn execute_ingestion_batch(
        &self,
        batch: &mut Vec<MarketData>,
        last_msg: &Option<OwnedMessage>,
        watchdog_state: &WatchdogState,
        consumer: &StreamConsumer,
    ) -> Result<()> {
        if batch.is_empty() { return Ok(()); }

        // 1. In-Memory Deduplication (Essential for Sync/Heal overlaps)
        let mut dedup_map = HashMap::new();
        for item in batch.drain(..) {
            if let Ok(ts) = time_utils::ts_to_utc_datetime(item.ts) {
                dedup_map.insert((ts, item.asset_id.clone()), item);
            }
        }
        let unique_batch: Vec<MarketData> = dedup_map.into_values().collect();

        // 2. Batch UPSERT into Postgres
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

        query_builder.build().execute(&self.pool).await
            .context("Failed to execute batch insert in DataService")?;

        // 3. Commit Kafka Offsets & Heartbeat Watchdog
        if let Some(msg) = last_msg {
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(msg.topic(), msg.partition(), rdkafka::Offset::Offset(msg.offset() + 1))?;
            consumer.commit(&tpl, CommitMode::Async)?;
        }
        
        watchdog_state.update();
        Ok(())
    }

    async fn check_actual_lag(&self, consumer: &StreamConsumer, topics: &[String]) -> bool {
        for topic in topics {
            match consumer.fetch_watermarks(topic, 0, Duration::from_secs(1)) {
                Ok((_, high)) => {
                    if high == 0 { continue; }
                    let current = consumer.position().ok().and_then(|tpl| {
                        tpl.find_partition(topic, 0).and_then(|p| match p.offset() {
                            rdkafka::Offset::Offset(o) => Some(o),
                            _ => None,
                        })
                    }).unwrap_or(-1);
                    if current != -1 && current < high { return false; }
                }
                Err(e) => { error!("Lag check error for {}: {:?}", topic, e); return false; }
            }
        }
        true
    }
}