




use chrono::Utc;

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


use shared_models::data_model::DataService;
use shared_models::data_model as dm;
use shared_models::market_data::{MarketData, TickTracker};
use shared_models::traits::MarketDataHandler;
use shared_models::time_utils;

use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use dotenvy;
use std::env;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::time::{interval};
use dashmap::DashSet;

use dashmap::DashMap;

use async_trait::async_trait; 
use shared_models::data_model::{BROKER_OFFSET, LAST_LIVE_TS, MARKET_SESSIONS};
use once_cell::sync::Lazy;


use crate::traits::{DataIngestionExt, DataMaintenanceExt};
use futures::StreamExt;
use smol_str::SmolStr;
use rustc_hash::FxHashMap;
// use shared_models::data_model::MARKET_SESSIONS;
use crate::watchdog::{Mt5Watchdog,WatchdogState};
use database_engine::runtime::{ setup_database,register_assets, map_packet_to_asset};
use shared_models::db_models as db; 
use surrealdb::{Surreal, engine::local::Mem, engine::local::SurrealKv, opt::auth::Root};
use shared_models::data_model::{IngestionContext, IngestionCoordinatorConfig, SYMBOL_MAP};


// static BROKER_OFFSET: AtomicI64 = AtomicI64::new(0);

static LIVE_AGGREGATOR: Lazy<DashMap<SmolStr, MarketData>> = Lazy::new(DashMap::new);
// static LAST_LIVE_TS: Lazy<DashMap<SmolStr, i64>> = Lazy::new(DashMap::new);
static TICK_MONITOR: Lazy<DashMap<SmolStr, TickTracker>> = Lazy::new(DashMap::new);

lazy_static::lazy_static! {
    static ref SYNCED_ASSETS: DashSet<SmolStr> = DashSet::new();
}


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
       // last_msg: &Option<OwnedMessage>,
        // watchdog_state: &WatchdogState,
        // consumer: &StreamConsumer,
    ) -> Result<()> {
        if batch.is_empty() { return Ok(()); }

        let mut dedup_map = FxHashMap::default(); // Faster hashing for the (ts, asset_id) key
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
            let ts = time_utils::ts_to_utc_datetime(data.ts).unwrap_or_else(|_| chrono::Utc::now());
            b.push_bind(ts)
             .push_bind(data.asset_id.as_str())
             .push_bind(data.open)
             .push_bind(data.high)
             .push_bind(data.low)
             .push_bind(data.close)
             .push_bind(data.volume);
        });

        query_builder.push(r#" 
            ON CONFLICT ("time", asset_id) DO UPDATE SET
                open = EXCLUDED.open,
                high = GREATEST(market_data.high, EXCLUDED.high),
                low = LEAST(market_data.low, EXCLUDED.low),
                close = EXCLUDED.close,
                volume = EXCLUDED.volume -- Overwrite with the most complete version
        "#);

        let rows_affected = query_builder.build()
            .execute(&self.pool).await
            .context("Failed to execute batch insert in DataService")?
            .rows_affected();
        info!("Batch processed {} record(s) in Postgres", rows_affected);

        // 3. Commit Kafka Offsets & Heartbeat Watchdog
        // if let Some(msg) = last_msg {
        //     let mut tpl = TopicPartitionList::new();
        //     tpl.add_partition_offset(msg.topic(), msg.partition(), rdkafka::Offset::Offset(msg.offset() + 1))?;
        //     consumer.commit(&tpl, CommitMode::Async)?;
        // }
        
        // watchdog_state.update();
        Ok(())
    }


    // async fn handle_mt5_ingestion(
    //     &self,
    //     mut socket: TcpStream,
    //     shutdown: CancellationToken,
    //     state: WatchdogState
    // ) -> Result<(), Box<dyn Error>> {
    //     info!("✅ MT5 Connected. Initializing Startup Sync...");

    //     let mut header = [0u8; 1];

    //     // 1. Wait for Calibration Packet (Header 254)
    //     socket.read_exact(&mut header).await?;
    //     if header[0] == 254 {
    //         self.handle_calibration(&mut socket).await.map_err(|e| e.to_string())?;
    //     }

    //     let (config, symbol_map) = self.load_ingestion_context()
    //         .map_err(|e| e.to_string())?;

    //     // 2. Call startup handshake
    //     self.send_startup_handshake(&mut socket, &config).await?;

    //     let mut batch = Vec::with_capacity(100);
    //     let mut flush_interval = interval(Duration::from_secs(5));
    //     let mut health_check_timer = interval(Duration::from_secs(60));
    //     let source_id = SmolStr::from(config.data_source_id.clone());

    //     loop {
    //         tokio::select! {
    //             _ = shutdown.cancelled() => {
    //                 info!("🛑 Shutdown signal received. Cleaning up...");
    //                 if let Err(e) = self.flush_live_aggregator().await {
    //                     error!("Error during final aggregator flush: {}", e);
    //                 }
    //                 if !batch.is_empty() {
    //                     self.execute_ingestion_batch(&mut batch).await?;
    //                 }
    //                 break;
    //             }

    //             _ = health_check_timer.tick() => {
    //                 self.log_aggregator_status();
    //             }

    //             _ = flush_interval.tick() => {
    //                 // 1. Run Heartbeat to close any stale bars (e.g. UK100 after hours)
    //                 let expired_bars = self.check_heartbeat();
    //                 for bar in expired_bars {
    //                     batch.push(bar);
    //                 }

    //                 // 2. Flush batch to DB
    //                 if !batch.is_empty() {
    //                     info!("⏱️ Timer triggered: Flushing {} bars", batch.len());
    //                     self.execute_ingestion_batch(&mut batch).await?;
    //                 }
    //             }

    //             read_res = socket.read_exact(&mut header) => {
    //                 if read_res.is_err() { break; } 
                    
    //                 let offset = BROKER_OFFSET.load(Ordering::SeqCst);
    //                 let worker_state = state.clone();
    //                 let whatdog = Mt5Watchdog::new(worker_state);

    //                 match header[0] {
    //                     0 => { // TICK (64 bytes)
    //                         let mut buf = [0u8; 64];
    //                         socket.read_exact(&mut buf).await?;

    //                         if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {

                                
    //                             let raw_name = SmolStr::new(String::from_utf8_lossy(&tick.asset));
    //                             let asset_name = raw_name.trim_matches(char::from(0));
    //                             let asset_id = symbol_map.get(asset_name)
    //                                 .cloned()
    //                                 .unwrap_or_else(|| SmolStr::from(format!("assets:{}:{}", asset_name, config.data_source_id)));


    //                             // --- HIBERNATION GATE ---
    //                             if let Some(session) = MARKET_SESSIONS.get(&asset_id.clone()) {
    //                                 if session.is_active == 0 {
    //                                     // If the market is closed, don't even touch the aggregator.
    //                                     // This prevents the "1 Ticks" health check entry.
    //                                     continue; 
    //                                 }
    //                             }

    //                             if !Mt5Watchdog::is_market_open(&whatdog){
    //                                 continue;
    //                             }

    //                             let utc_ts_ms = if tick.time_msc > 10_000_000_000 {
    //                                 tick.time_msc - (offset * 1000)
    //                             } else {
    //                                 (tick.time_msc - offset) * 1000
    //                             };

    //                             let tick_market_data = MarketData {
    //                                 ts: utc_ts_ms,
    //                                 asset_id: asset_id.clone(),
    //                                 open: tick.bid, high: tick.bid, low: tick.bid, close: tick.bid,
    //                                 volume: tick.volume as f64,
    //                                 source: Some(source_id.clone()),
    //                                 seq: None,
    //                             };

    //                             state.update();

    //                             if let Some(completed_bar) = self.handle_tick(tick_market_data).await {
    //                                 batch.push(completed_bar);
    //                             }

    //                             // let bid = tick.bid;
    //                             // if bid != tick.last{
                                
    //                             //  println!("📈 [LIVE] {} | Bid: {:.2}", asset_name, bid)
    //                             // };
    //                         }
    //                     }

    //                     1 => { // BAR (64 bytes)
    //                         let mut buf = [0u8; 64];
    //                         socket.read_exact(&mut buf).await?;

    //                         if let Ok(bar) = bytemuck::try_from_bytes::<dm::SyncBar>(&buf) {
    //                             let raw_name = SmolStr::new(String::from_utf8_lossy(&bar.asset));
    //                             let asset_name = raw_name.trim_matches(char::from(0));
    //                             let asset_id = symbol_map.get(asset_name)
    //                                 .cloned()
    //                                 .unwrap_or_else(|| SmolStr::from(format!("assets:{}:{}", asset_name, config.data_source_id)));

    //                             if !Mt5Watchdog::is_market_open(&whatdog){
    //                                 continue;
    //                             }

    //                             let utc_seconds = bar.time - offset;
    //                             let corrected_time_ms = if utc_seconds < 10_000_000_000 { utc_seconds * 1000 } else { utc_seconds };

    //                             batch.push(MarketData {
    //                                 ts: corrected_time_ms,
    //                                 asset_id: asset_id.clone(),
    //                                 open: bar.open, high: bar.high, low: bar.low, close: bar.close,
    //                                 volume: bar.volume as f64,
    //                                 source: Some(source_id.clone()),
    //                                 seq: None,
    //                             });

    //                             if batch.len() >= 50 {
    //                                 self.execute_ingestion_batch(&mut batch).await?;
    //                             }

    //                             SYNCED_ASSETS.insert(asset_id.clone());
    //                         }
    //                     }

    //                     3 => { // SESSION PACKET (36 bytes)
    //                         let mut buf = [0u8; 36];
    //                         socket.read_exact(&mut buf).await?;

    //                         if let Ok(packet) = bytemuck::try_from_bytes::<dm::SessionPacket>(&buf) {
    //                             let raw_name = SmolStr::new(String::from_utf8_lossy(&packet.asset));
    //                             let asset_name = raw_name.trim_matches(char::from(0));
    //                             let asset_id = symbol_map.get(asset_name)
    //                                 .cloned()
    //                                 .unwrap_or_else(|| SmolStr::from(format!("assets:{}:{}", asset_name, config.data_source_id)));
                                
    //                              // Update the global session map
    //                             // MARKET_SESSIONS.insert(asset_id.clone(), packet.clone());

                         
                                
    //                             // Store in global session map
    //                             MARKET_SESSIONS.insert(asset_id.clone(), *packet);
    //                             if packet.is_active == 0 {
    //                                 // If MQL5 says inactive, remove from Live Aggregator immediately
    //                                 if let Some((id, agg)) = LIVE_AGGREGATOR.remove(&asset_id) {
    //                                     info!("💤 Immediate Hibernation: {} (Market Closed via MQL5)", id);
                                        
    //                                     // Optional: Force a flush of the final tick before killing
    //                                     // if agg.volume > 0.0 {
    //                                     //     self.execute_ingestion_batch(&agg).await?;
    //                                     // }
    //                                 }
    //                             } else {
    //                                 info!("📅 Session for {}: {:02}:{:02}-{:02}:{:02} (Active: {})", 
    //                                     asset_name, packet.open_hour, packet.open_min, 
    //                                     packet.close_hour, packet.close_min, packet.is_active);
    //                             }
    //                         }
    //                     }
       
    //                     5 => {
    //                         // 1. Read the 64-byte payload
    //                         let mut buf = [0u8; 64];
    //                         socket.read_exact(&mut buf).await?;

    //                         let raw_name = SmolStr::new(String::from_utf8_lossy(&buf[0..10]));
    //                         let asset_name = raw_name.trim_matches(char::from(0));
    //                         let asset_id = symbol_map.get(asset_name)
    //                             .cloned()
    //                             .unwrap_or_else(|| SmolStr::from(format!("assets:{}:{}", asset_name, config.data_source_id)));

    //                         // 2. Check memory status
    //                         let is_live = LIVE_AGGREGATOR.contains_key(&asset_id) || SYNCED_ASSETS.contains(&asset_id);

    //                         // 3. Check session status (Updated)
    //                         let is_active = MARKET_SESSIONS.get(&asset_id)
    //                             .map(|s| s.is_active == 1)
    //                             .unwrap_or(false);

    //                         // 4. Decision Logic: ONLY skip sync if the asset is LIVE AND the market is ACTIVE
    //                         // If it's the weekend (is_active == 0), we return 0 to force a clean state check.
    //                         let sync_needed_byte = if is_live && is_active {
    //                             info!("🤝 Negotiation: {} is already LIVE & ACTIVE. Telling MT5 to skip sync.", asset_name);
    //                             1u8 // 1 = Found / Skip Sync
    //                         } else {
    //                             let reason = if !is_active { "Market is CLOSED" } else { "Asset is NEW" };
    //                             info!("📥 Negotiation: {} ({}). Requesting full history sync.", asset_name, reason);
    //                             0u8 // 0 = Not found or Closed / Sync required
    //                         };

    //                         // 5. Send the 1-byte decision back to MT5
    //                         socket.write_all(&[sync_needed_byte]).await?;
    //                     }
                        
    //                     _ => {
    //                         warn!("⚠️ Desync: Unknown Header {}", header[0]);
    //                         break;
    //                     }
    //                 }
    //             }
    //         }
    //     }
    //     Ok(())
    // }


    async fn handle_mt5_ingestion_new(
        &self,
        mut socket: TcpStream,
        dbs: Arc<db::AppDatabases>,
        shutdown: CancellationToken,
        state: WatchdogState
    ) -> Result<(), Box<dyn Error>> {
        info!("✅ MT5 Connected. Initializing Startup Sync...");

       // 1. Pull the static context once per connection
        // let context = SYMBOL_MAP.get().ok_or("CRITICAL: SYMBOL_MAP OnceCell is empty!")?;
        let context = self.load_ingestion_context().map_err(|e| e.to_string())?;

        // 2. Perform startup handshake using the config from context
        self.send_startup_handshake(&mut socket, &context.0).await?;

         // --- NEW: RECONNECTION GAP CHECK ---
        // Even if aggregator was flushed, LAST_LIVE_TS remembers the last real tick.
        for entry in LAST_LIVE_TS.iter() {
            let (asset_id, last_ts) = entry.pair();
            let now_ms = Utc::now().timestamp_millis();
            // let offset = BROKER_OFFSET.load(Ordering::SeqCst) * 1000;
            let offset = self.get_broker_offset () * 1000;
            let broker_now_ms = now_ms + offset;
            
            let gap = broker_now_ms - *last_ts;
            if gap > 120_000 { // 2 Minutes Gap
                warn!("⚠️ Detection: {} was offline for {}s. Requesting Backfill...", asset_id, gap/1000);
                let _ = self.request_manual_sync(&mut socket, asset_id, *last_ts).await;
            }
        }

        // let (config, symbol_map) = self.load_ingestion_context().map_err(|e| e.to_string())?;
        //  let context = SYMBOL_MAP.get().ok_or("Ingestion Context not initialized")?;
        // let dbs = setup_database().await.map_err(|e| e.to_string())?;
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
                                0 => self.process_tick_header(&mut socket, &context, &state, &mut batch).await?,
                                1 => self.process_bar_header(&mut socket, &context, &mut batch).await?,
                                3 => self.process_session_header(&mut socket, &context).await?,
                                4 => self.process_dna_header(&mut socket, dbs.clone()).await?,
                                5 => self.process_negotiation_header(&mut socket, &context).await?,
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

    async fn request_manual_sync(&self, socket: &mut TcpStream, asset_id: &str, start_ts_ms: i64) -> Result<()> {
        let mut packet = Vec::with_capacity(25);
        packet.push(100u8); // Header 100

        let raw_symbol = asset_id.split(':').nth(1).unwrap_or(asset_id);
        let mut name_buf = [0u8; 16];
        let name_bytes = raw_symbol.as_bytes();
        let len = name_bytes.len().min(16);
        name_buf[..len].copy_from_slice(&name_bytes[..len]);
        packet.extend_from_slice(&name_buf);

        let start_ts_secs = start_ts_ms / 1000;
        packet.extend_from_slice(&start_ts_secs.to_le_bytes());

        socket.write_all(&packet).await?;
        socket.flush().await?;
        
        info!("🕳️ Gap Recovery Triggered: Backfilling {} from {}", raw_symbol, start_ts_secs);
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

    async fn process_dna_header(
        &self, 
        socket: &mut TcpStream, 
        dbs: Arc<db::AppDatabases>,
    ) -> Result<(), Box<dyn Error>> {
        let mut buf = vec![0u8; std::mem::size_of::<db::AssetInfoPacket>()];
        socket.read_exact(&mut buf).await?;

        if let Ok(packet) = bytemuck::try_from_bytes::<db::AssetInfoPacket>(&buf) {
            if let Err(e) = register_assets(dbs, *packet).await {
                error!("❌ DNA Sync Error: {}", e);
            }
        }
        Ok(())
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

    async fn process_session_header(
        &self,
        socket: &mut TcpStream,
        context: &IngestionContext,
    ) -> Result<(), Box<dyn Error>> {
        let mut buf = [0u8; 40]; // Matches dm::SessionPacket size
        socket.read_exact(&mut buf).await?;

        if let Ok(packet) = bytemuck::try_from_bytes::<dm::SessionPacket>(&buf) {
            let asset_id = self.resolve_asset_id(&packet.asset, context);

            info!("📬 Session Update: {} | Index: {} | Active: {} | {:02}:{:02}-{:02}:{:02}", 
                asset_id, packet.session_index, packet.is_active,
                packet.open_hour, packet.open_min, packet.close_hour, packet.close_min
            );

            // Update Global Session Map
            if packet.session_index == 0 {
                // New day/First packet: Overwrite existing session list
                MARKET_SESSIONS.insert(asset_id.clone(), vec![*packet]);
            } else {
                // Multi-session day (e.g. lunch breaks): Append to existing
                if let Some(mut sessions) = MARKET_SESSIONS.get_mut(&asset_id) {
                    sessions.push(*packet);
                }
            }

            // Hibernation: If the market is marked inactive, stop aggregating live ticks
            if packet.is_active == 0 {
                if let Some((id, _)) = LIVE_AGGREGATOR.remove(&asset_id) {
                    info!("💤 Hibernating {}: Market is closed.", id);
                }
            }
        }
        Ok(())
    }

    async fn process_negotiation_header(
        &self, 
        socket: &mut TcpStream, 
        context: &IngestionContext,
    ) -> Result<(), Box<dyn Error>> {
        let mut buf = [0u8; 64];
        socket.read_exact(&mut buf).await?;
        let asset_name = std::str::from_utf8(&buf[0..10])?.trim_matches(char::from(0)).trim();
        let asset_id = self.resolve_asset_id_str(asset_name, context);

        let is_live = LIVE_AGGREGATOR.contains_key(&asset_id) || SYNCED_ASSETS.contains(&asset_id);
        let is_active = MARKET_SESSIONS.get(&asset_id)
            .and_then(|s| s.first().map(|p| p.is_active == 1))
            .unwrap_or(false);

        let sync_needed_byte = if is_live && is_active { 1u8 } else { 0u8 };
        socket.write_all(&[sync_needed_byte]).await?;
        Ok(())
    }

    fn handle_socket_error(&self, e: std::io::Error) {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            info!("🔌 MT5 disconnected gracefully (likely timeframe/symbol change).");
        } else {
            error!("❌ Socket error: {}", e);
        }
    }

    async fn flush_batch_if_needed(&self, batch: &mut Vec<MarketData>) -> Result<(), Box<dyn Error>> {
        let expired_bars = self.check_heartbeat(); // Your custom heartbeat logic
        for bar in expired_bars { batch.push(bar); }

        if !batch.is_empty() {
            debug!("⏱️ Flush Timer: Processing {} bars", batch.len());
            self.execute_ingestion_batch(batch).await?;
        }
        Ok(())
    }

    // fn resolve_asset_id(&self, asset_bytes: &[u8], symbol_map: &HashMap<SmolStr, SmolStr>, config: &IngestionContext) -> SmolStr {
    //     let name = std::str::from_utf8(asset_bytes).unwrap_or("").trim_matches(char::from(0));
    //     self.resolve_asset_id_str(name, symbol_map, config)
    // }

    // fn resolve_asset_id_str(&self, name: &str, symbol_map: &HashMap<SmolStr, SmolStr>, config: &IngestionContext) -> SmolStr {
    //     symbol_map.get(name)
    //         .cloned()
    //         .unwrap_or_else(|| SmolStr::from(format!("assets:{}:{}", name, config.0.data_source_id)))
    // }

    // Helper to resolve Asset ID from raw bytes (used in Tick/Bar packets)
    fn resolve_asset_id(&self, asset_bytes: &[u8], context: &IngestionContext) -> SmolStr {
        let name = std::str::from_utf8(asset_bytes).unwrap_or("").trim_matches(char::from(0));
        
        // context.1 is the FxHashMap<SmolStr, SmolStr>
        context.1.get(name)
            .cloned()
            .unwrap_or_else(|| SmolStr::from(format!("assets:{}:{}", name, context.0.data_source_id)))
    }

    // Helper to resolve Asset ID from a string slice
    fn resolve_asset_id_str(&self, name: &str, context: &IngestionContext) -> SmolStr {
        let (config, symbol_map) = context;
        
        // Check the FxHashMap first
        if let Some(id) = symbol_map.get(name) {
            return id.clone();
        }
        
        // Fallback: Generate the standard URN if not in the map
        SmolStr::from(format!("assets:{}:{}", name, config.data_source_id))
    }


    async fn handle_mt5_ingestion(
    &self,
    mut socket: TcpStream,
    shutdown: CancellationToken,
    state: WatchdogState
) -> Result<(), Box<dyn Error>> {
    info!("✅ MT5 Connected. Initializing Startup Sync...");

    // --- STEP 0: DRAIN STALE BYTES ---
    // If Rust restarted but MT5 was still talking, clear the pipe.
    // let mut drain_buf = [0u8; 1024];
    // loop {
    //     match tokio::time::timeout(Duration::from_millis(50), socket.read(&mut drain_buf)).await {
    //         Ok(Ok(n)) if n > 0 => {
    //             debug!("🧹 Drained {} stale bytes from socket", n);
    //             continue;
    //         },
    //         _ => break, // Socket is clear or timed out (which is what we want)
    //     }
    // }

    let mut header = [0u8; 1];

    // 1. Peek at the first byte without consuming it
    let mut peek_buf = [0u8; 1];
    let peek_res = tokio::time::timeout(Duration::from_secs(1), socket.peek(&mut peek_buf)).await;

    if let Ok(Ok(n)) = peek_res {
        if n > 0 && peek_buf[0] == 254 {
            // It IS a calibration packet, consume the header and handle it
            socket.read_exact(&mut header).await?;
            self.handle_calibration(&mut socket).await.map_err(|e| e.to_string())?;
        } else {
            // It's a normal data header (0, 3, 5), don't consume it here!
            debug!("Existing connection detected (No Calibration needed).");
        }
    }

    let (config, symbol_map) = self.load_ingestion_context().map_err(|e| e.to_string())?;

    // 2. Startup Handshake
    self.send_startup_handshake(&mut socket, &config).await?;

    // --- NEW: RECONNECTION GAP CHECK ---
        // Even if aggregator was flushed, LAST_LIVE_TS remembers the last real tick.
    for entry in LAST_LIVE_TS.iter() {
        let (asset_id, last_ts) = entry.pair();
        let now_ms = Utc::now().timestamp_millis();
        // let offset = BROKER_OFFSET.load(Ordering::SeqCst) * 1000;
        let offset = self.get_broker_offset () * 1000;
        let broker_now_ms = now_ms + offset;
        
        let gap = broker_now_ms - *last_ts;
        if gap > 120_000 { // 2 Minutes Gap
            warn!("⚠️ Detection: {} was offline for {}s. Requesting Backfill...", asset_id, gap/1000);
            let _ = self.request_manual_sync(&mut socket, asset_id, *last_ts).await;
        }
    }

    let mut batch = Vec::with_capacity(100);
    let mut flush_interval = interval(Duration::from_secs(2));
    let mut health_check_timer = interval(Duration::from_secs(60));
    let source_id = SmolStr::from(config.data_source_id.clone());
    let dbs = setup_database().await.map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                info!("🛑 Shutdown signal received. Cleaning up...");
                let _ = self.flush_live_aggregator().await;
                if !batch.is_empty() {
                    let _ = self.execute_ingestion_batch(&mut batch).await;
                }
                break;
            }

            _ = health_check_timer.tick() => {
                self.log_aggregator_status();
            }

            _ = flush_interval.tick() => {
                let expired_bars = self.check_heartbeat();
                for bar in expired_bars { batch.push(bar); }
                if !batch.is_empty() {
                    info!("⏱️ Timer triggered: Flushing {} bars", batch.len());
                    self.execute_ingestion_batch(&mut batch).await?;
                }
            }

            // 3. Graceful Read with EOF handling
            read_res = socket.read_exact(&mut header) => {
                match read_res {
                    Ok(_) => {
                        // let offset = self.get_broker_offset();
                        // let offset = BROKER_OFFSET.load(Ordering::SeqCst);
                        let offset_ms = self.get_broker_offset() * 1000;
                        
                        let watchdog = Mt5Watchdog::new(state.clone());

                        match header[0] {
                            0 => { // TICK (64 bytes)
                                let mut buf = [0u8; 64];
                                socket.read_exact(&mut buf).await?;

                                if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {
                                    let asset_name = std::str::from_utf8(&tick.asset)?.trim_matches(char::from(0));
                                    let asset_id = symbol_map.get(asset_name)
                                        .cloned()
                                        .unwrap_or_else(|| SmolStr::from(format!("assets:{}:{}", asset_name, config.data_source_id)));

                                    if let Some(session) = MARKET_SESSIONS.get(&asset_id) {
                                        if let Some(first_session) = session.first() {
                                            if first_session.is_active == 0 { continue; }
                                        }
                                    }
                                    let market_is_open = self.check_session_status(&asset_id);

                                    if  !Mt5Watchdog::is_market_open(&watchdog) { continue; }
                                    if !market_is_open {
                                        // If market is closed, skip processing but still update the session status
                                        if let Some((id, _)) = LIVE_AGGREGATOR.remove(&asset_id) {
                                            info!("💤 Hibernation: {} (Market Closed)", id);
                                        }
                                        continue;
                                    }

                                    // Update the Last Known Real Tick TS
                                    LAST_LIVE_TS.insert(asset_id.clone(), tick.time_msc);

                                    // let utc_ts_ms = if tick.time_msc > 10_000_000_000 {
                                    //     tick.time_msc - (offset * 1000)
                                    // } else {
                                    //     (tick.time_msc - offset) * 1000
                                    // };

                                    let broker_ms = tick.time_msc;
                                   
                                    let utc_ts_ms = broker_ms - offset_ms;

                                    let tick_market_data = MarketData {
                                        ts: utc_ts_ms,
                                        asset_id: asset_id.clone(),
                                        open: tick.bid,
                                        high: tick.bid, 
                                        low: tick.bid, 
                                        close: tick.bid,
                                        volume: tick.volume as f64,
                                        source: Some(source_id.clone()),
                                        seq: None,
                                    };

                                    state.update();
                                    if let Some(completed_bar) = self.handle_tick(tick_market_data, tick.bid, tick.ask,tick.volume).await {
                                        batch.push(completed_bar);
                                    }
                                }
                            }



                            1 => { // BAR (64 bytes)
                                let mut buf = [0u8; 64];
                                socket.read_exact(&mut buf).await?;

                                if let Ok(bar) = bytemuck::try_from_bytes::<dm::SyncBar>(&buf) {
                                    let asset_name = std::str::from_utf8(&bar.asset)?.trim_matches(char::from(0));
                                    let asset_id = symbol_map.get(asset_name)
                                        .cloned()
                                        .unwrap_or_else(|| SmolStr::from(format!("assets:{}:{}", asset_name, config.data_source_id)));

                                    // let offset = self.get_broker_offset() * 1000;
                                    let bar_time_ms = if bar.time < 10_000_000_000 { bar.time * 1000 } else { bar.time };
                                    let utc_ts_ms = bar_time_ms - offset_ms;

                                    // --- NEW LOGIC: SEED THE AGGREGATOR ---
                                    // If this bar is "Current" (within the last 15 mins), put it in the Aggregator
                                    let now_utc = Utc::now().timestamp_millis();
                                    let is_very_recent = (now_utc - utc_ts_ms).abs() < (5 * 60 * 1000); 

                                    let market_data = MarketData {
                                        ts: utc_ts_ms,
                                        asset_id: asset_id.clone(),
                                        open: bar.open, 
                                        high: bar.high, 
                                        low: bar.low, 
                                        close: bar.close,
                                        volume: bar.volume as f64,
                                        source: Some(source_id.clone()),
                                        seq: None,
                                    };

                                    if is_very_recent {
                                        // Put it in the aggregator so live ticks can "continue" this bar
                                        LIVE_AGGREGATOR.insert(asset_id.clone(), market_data.clone());
                                        info!("🌱 Seeded Aggregator for {} with sync bar at {}", asset_name, utc_ts_ms);
                                    }
                                    
                                    // Always push to batch as well (UPSERT will handle the overlap in DB)
                                    batch.push(market_data);

                                    if batch.len() >= 50 {
                                        self.execute_ingestion_batch(&mut batch).await?;
                                    }
                                    SYNCED_ASSETS.insert(asset_id);
                                }
                            }

                 

                            3 => { // SESSION (36 bytes)
                                let mut buf = [0u8; 40];
                                socket.read_exact(&mut buf).await?;

                                if let Ok(packet) = bytemuck::try_from_bytes::<dm::SessionPacket>(&buf) {
                                    let asset_name = std::str::from_utf8(&packet.asset)?.trim_matches(char::from(0));
                                    let asset_id = symbol_map.get(asset_name)
                                        .cloned()
                                        .unwrap_or_else(|| SmolStr::from(format!("assets:{}:{}", asset_name, config.data_source_id)));
                                    
                                    // --- PRINT THE PACKET DATA ---
                                    info!("📬 Received Session: {} | Index: {} | Active: {} | Window: {:02}:{:02} - {:02}:{:02}", 
                                        asset_id, 
                                        packet.session_index, 
                                        packet.is_active,
                                        packet.open_hour, packet.open_min,
                                        packet.close_hour, packet.close_min
                                    );

                                    // 🚨 FIXED: Handle multiple session packets
                                    if packet.session_index == 0 {
                                        // Index 0 means a fresh daily update: Replace the list
                                        MARKET_SESSIONS.insert(asset_id.clone(), vec![*packet]);
                                    } else {
                                        // Subsequent indices: Append to the list
                                        if let Some(mut sessions) = MARKET_SESSIONS.get_mut(&asset_id) {
                                            sessions.push(*packet);
                                        }
                                    }

                                    // Hibernation Logic: Only hibernate if the WHOLE day is marked inactive
                                    if packet.is_active == 0 {
                                        if let Some((id, _)) = LIVE_AGGREGATOR.remove(&asset_id) {
                                            info!("💤 Hibernation: {} (Market Inactive for the day)", id);
                                        }
                                    }
                                }
                            }
                            
                            4 => {// --- HEADER 4: ASSET DNA ---
                                let mut buf = vec![0u8; std::mem::size_of::<db::AssetInfoPacket>()];
                                
                                if socket.read_exact(&mut buf).await.is_ok() {
                                    let packet: &db::AssetInfoPacket = bytemuck::from_bytes(&buf);
                                    if let Err(e) = register_assets(dbs.clone(), *packet).await {
                                        eprintln!("❌ DB Error: {}", e);
                                    }
                                }
                            },

                            5 => { // NEGOTIATION (64 bytes)
                                let mut buf = [0u8; 64];
                                socket.read_exact(&mut buf).await?;
                                let asset_name = std::str::from_utf8(&buf[0..10])?.trim_matches(char::from(0)).trim();
                                
                                let asset_id = symbol_map.get(asset_name)
                                    .cloned()
                                    .unwrap_or_else(|| SmolStr::from(format!("assets:{}:{}", asset_name, config.data_source_id)));

                                let is_live = LIVE_AGGREGATOR.contains_key(&asset_id) || SYNCED_ASSETS.contains(&asset_id);
                                let is_active = MARKET_SESSIONS.get(&asset_id).and_then(|s| s.first().map(|p| p.is_active == 1)).unwrap_or(false);

                                // If weekend (is_active=false), force sync check (0) to be safe
                                let sync_needed_byte = if is_live && is_active { 1u8 } else { 0u8 };
                                socket.write_all(&[sync_needed_byte]).await?;
                            }

                            // Inside your ingestion loop
                           254 => { 
                                // 🕒 Periodic Calibration / New Day Sync
                                // Use your existing function to keep logic DRY
                                self.handle_calibration(&mut socket).await.map_err(|e| e.to_string())?;
                            },
                            
                            _ => {
                                warn!("⚠️ Desync: Unknown Header {}", header[0]);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::UnexpectedEof {
                            info!("🔌 MT5 disconnected gracefully (Timeframe change).");
                        } else {
                            error!("❌ Socket error: {}", e);
                        }
                        break;
                    }
                }
            }
        }
    }
    
    // Final cleanup before worker dies
    let _ = self.flush_live_aggregator().await;
    if !batch.is_empty() { let _ = self.execute_ingestion_batch(&mut batch).await; }
    
    Ok(())
}

//    async fn handle_tick(&self, tick_data: MarketData,bid: f64, 
//     ask: f64) -> Option<MarketData> {
//         let window_size = 3 * 60 * 1000; // 15 minutes in ms
//         let bar_start_ts = (tick_data.ts / window_size) * window_size;
//          // FIX: If tick volume is 0, we treat this tick as 1 unit of "Tick Volume"
//         let tick_increment = 1.0;

        
//         // Use the static map instead of self
//         let mut entry = LIVE_AGGREGATOR.entry(tick_data.asset_id.clone()).or_insert_with(|| {
//             MarketData {
//                 ts: bar_start_ts,
//                 asset_id: tick_data.asset_id.clone(),
//                 open: tick_data.close,
//                 high: tick_data.close,
//                 low: tick_data.close,
//                 close: tick_data.close,
//                 volume: tick_increment,
//                 source: tick_data.source.clone(),
//                 seq: None,
//             }
//         });

//         // 2. State Tracker (Bid/Ask Memory)
//         let mut monitor = TICK_MONITOR.entry(tick_data.asset_id.clone()).or_insert_with(|| {
//             TickTracker {
//                 last_bid: bid,
//                 last_ask: ask,
//                 last_ts_msc: tick_data.ts,
//             }
//         });

        
//         // Mid-stream Gap Detection
//         if bar_start_ts > entry.ts + window_size {
//             warn!("🕳️ Mid-stream Gap for {}. Window jumped!", tick_data.asset_id);
//             // We let the reconnection logic or a manual sync handle this gap.
//         }

//         if bar_start_ts > entry.ts {
//             let finished_bar = entry.clone();
            
//             entry.ts = bar_start_ts;
//             entry.open = tick_data.close;
//             entry.high = tick_data.close;
//             entry.low = tick_data.close;
//             entry.close = tick_data.close;
//             entry.volume = tick_increment;

//             // Update monitor for the new bar
//             monitor.last_ts_msc = tick_data.ts;
//             monitor.last_bid = bid;
//             monitor.last_ask = ask;
            
//             return Some(finished_bar);
//         }

        
//         // 4. THE FILTER & DELTA LOGIC
//         // Check if this is a "New Information" tick (Matches Broker Volume Logic)
//         // 4. Volume Filtering using the passed Bid/Ask
//         let is_new_tick = bid != monitor.last_bid && ask != monitor.last_ask;
        
//         // let price_moved = (bid - monitor.last_bid).abs() > 0.0000001 || 
//         //           (ask - monitor.last_ask).abs() > 0.0000001;

//         if is_new_tick {
            
        
//         // Increment volume only for real updates
//             entry.volume +=  tick_increment ;

        

//         // Update the monitor state
//             monitor.last_ts_msc = tick_data.ts;
//             monitor.last_bid = bid;
//             monitor.last_ask = ask;
        

//          // This prevents "Multiplexed Bundles" from artificially inflating volume
//         // if tick_data.close != entry.close || entry.volume == 0.0 
        
//             // entry.volume += 1.0;

//             // Update OHLC values
//             entry.close = tick_data.close;
//             if tick_data.close > entry.high { entry.high = tick_data.close; }
//             if tick_data.close < entry.low { entry.low = tick_data.close; }
//         }

//         None
//     }

    async fn handle_tick(
        &self, 
        tick_data: MarketData, 
        bid: f64, 
        ask: f64, 
        broker_vol: i64
    ) -> Option<MarketData> {
        let m1_window = 5 * 60 * 1000;       // 1 minute
        // // let m15_window = 6 * 60 * 1000; // 15 minutes
        
        let current_m1_ts = (tick_data.ts / m1_window) * m1_window;
        // let current_m15_ts = (tick_data.ts / m15_window) * m15_window;
        let window_size = 5 * 60 * 1000; // 15 minutes in ms
        let bar_start_ts = (tick_data.ts / window_size) * window_size;

        // --- 1. GET OR CREATE 15m AGGREGATOR ENTRY ---
        let mut entry = LIVE_AGGREGATOR.entry(tick_data.asset_id.clone()).or_insert_with(|| {
            MarketData {
                ts: bar_start_ts,
                asset_id: tick_data.asset_id.clone(),
                open: tick_data.close,
                high: tick_data.close,
                low: tick_data.close,
                close: tick_data.close,
                volume: broker_vol as f64, // Initial M1 volume
                source: tick_data.source.clone(),
                seq: None,
            }
        });

        // --- 2. GET OR CREATE TRACKER (For Delta & Volume Sync) ---
        let mut monitor = TICK_MONITOR.entry(tick_data.asset_id.clone()).or_insert_with(|| {
            TickTracker {
                last_bid: bid,
                last_ask: ask,
                last_ts_msc: tick_data.ts,
                last_m1_ts: current_m1_ts,
                vol_before_current_m1: 0.0,
            }
        });

        // --- 3. HANDLE 15m BAR ROLLOVER ---
        if bar_start_ts > entry.ts {
            let finished_bar = entry.clone();
            
            // Reset 15m Entry
            entry.ts = bar_start_ts;
            entry.open = tick_data.close;
            entry.high = tick_data.close;
            entry.low = tick_data.close;
            entry.close = tick_data.close;
            entry.volume = broker_vol as f64; 

            // Reset Monitor for new 15m context
            monitor.last_m1_ts = current_m1_ts;
            monitor.vol_before_current_m1 = 0.0;
            monitor.last_bid = bid;
            monitor.last_ask = ask;

            return Some(finished_bar);
        }

        if bar_start_ts == entry.ts {
            if (broker_vol as f64) > entry.volume {
                entry.volume = broker_vol as f64;
            }
        }

        // --- 4. DELTA STRATEGY HOOK ---
        // This runs on EVERY tick, regardless of volume changes.
        let bid_delta = bid - monitor.last_bid;
        let ask_delta = ask - monitor.last_ask;

        if bid_delta.abs() > f64::EPSILON || ask_delta.abs() > f64::EPSILON {
            // Example: Detect aggressive selling (Price hitting the Bid)
            if bid_delta < 0.0 {
                // self.apply_delta_strategy(tick_data.asset_id.clone(), "SELL", bid_delta.abs()).await;
            }
        }

        // --- 5. VOLUME RECONCILIATION ---
        // If we moved into a new 1-minute candle within the same 15-minute bar
        // if current_m1_ts > monitor.last_m1_ts {
        //     // Add the 'final' volume of the minute that just ended to our 15m cumulative total
        //     // Note: broker_m1_vol here would be the first tick of the NEW minute, 
        //     // but we assume the previous minute's last sent volume was the 'total'.
        //     // For simplicity, we just use the current entry.volume as the base for the next minute.
        //     monitor.vol_before_current_m1 = entry.volume;
        //     monitor.last_m1_ts = current_m1_ts;
        // }

        // Official 15m Volume = (Volume from closed M1 bars) + (Official Volume of current M1 bar)
        // entry.volume = monitor.vol_before_current_m1 + (broker_vol as f64);
        entry.volume = broker_vol as f64;
        // --- 6. UPDATE OHLC & MONITOR ---
        entry.close = tick_data.close;
        if tick_data.close > entry.high { entry.high = tick_data.close; }
        if tick_data.close < entry.low { entry.low = tick_data.close; }

        monitor.last_bid = bid;
        monitor.last_ask = ask;
        monitor.last_ts_msc = tick_data.ts;

        None
    }

    async fn flush_live_aggregator(&self) -> Result<()> {
        info!("📥 Flushing live aggregator buckets to database...");
        
        // 1. Collect all "in-progress" bars from the static map
        let mut final_bars: Vec<MarketData> = Vec::new();
        
        // Use a block to ensure the lock is dropped quickly
        {
            for entry in LIVE_AGGREGATOR.iter() {
                final_bars.push(entry.value().clone());
            }
        }

        // 2. Clear the map so we don't double-process if shutdown takes time
        LIVE_AGGREGATOR.clear();

        // 3. Reuse your existing batch function to save them
        if !final_bars.is_empty() {
            self.execute_ingestion_batch(&mut final_bars).await?;
        }

        info!("✅ Aggregator flush complete.");
        Ok(())
    }

    fn log_aggregator_status(&self) {
        if LIVE_AGGREGATOR.is_empty() {
            info!("📊 Aggregator Status: No active bars in memory.");
            return;
        }
        // Only print if there is at least one bar with actual volume or recent activity
        let active_count = LIVE_AGGREGATOR.iter().filter(|e| e.value().volume > 1.0).count();
        if active_count == 0 {
            return; 
        }

        let meaningful_activity = LIVE_AGGREGATOR.iter().any(|entry| {
            let data = entry.value();
            let is_open = self.check_session_status(entry.key());
            data.volume > 1.0 || is_open
        });

        if !meaningful_activity { return; }

        // let offset_secs = self.get_broker_offset() * 1000;
       


        info!("--- 🛰️  Live Aggregator Health Check ---");
        for entry in LIVE_AGGREGATOR.iter() {
            let (asset_id, data) = entry.pair();
           
            let utc_ts = data.ts; 
            if data.volume == 0.0 { continue; }
            // Convert timestamp for readability
            let time_str = time_utils::ts_to_utc_datetime(utc_ts)
                .map(|dt| dt.format("%H:%M:%S").to_string())
                .unwrap_or_else(|_| "Unknown".to_string());

            info!(
                "ID: {:<20} | Start: {} | Price: {:>10.2} | Ticks: {:>5}",
                asset_id, time_str, data.close, data.volume
            );
        }
        info!("---------------------------------------");
    }

    // fn check_heartbeat(&self) -> Vec<MarketData> {
    //     let mut completed_bars = Vec::new();
    //     let window_size = 15 * 60 * 1000; // 15 Minutes
        
    //     let now_ms = std::time::SystemTime::now()
    //         .duration_since(std::time::UNIX_EPOCH)
    //         .unwrap()
    //         .as_millis() as i64;
            
    //     let current_window_start = (now_ms / window_size) * window_size;

    //     // Use retain to safely remove "hibernating" assets while iterating
    //     LIVE_AGGREGATOR.retain(|asset_id, data| {

    //         let market_is_open = self.check_session_status(asset_id);
    
    //         // Check for "Zombie" bars: No volume and market is closed
    //         if !market_is_open && data.volume <= 1.0 {
    //             // If it's been sitting here for more than 2 minutes with no activity
    //             let idle_time = now_ms - data.ts; 
    //             if idle_time > 120_000 { 
    //                 info!("💤 Immediate Hibernation: {} (Market Closed)", asset_id);
    //                 return false; 
    //             }
    //         }


    //         // Only act if the bar in memory belongs to a past window
    //         if data.ts < current_window_start {
                
    //             // 1. Calculate if market is currently open
    //             let market_is_open = if let Some(session) = MARKET_SESSIONS.get(asset_id.as_str()) {
    //                 if session.is_active == 0 {
    //                     false
    //                 } else {
    //                     let current_min_of_day = ((now_ms / 1000 / 60) % 1440) as u32;
    //                     let start_min = (session.open_hour * 60) + session.open_min;
    //                     let end_min = (session.close_hour * 60) + session.close_min;
    //                     current_min_of_day >= start_min && current_min_of_day < end_min
    //                 }
    //             } else {
    //                 true // Default to true if no session data yet
    //             };

    //             // 2. Decide: Flush, Reset, or Hibernate?
    //             if market_is_open || data.volume > 0.0 {
    //                 // We have data to save OR the market is open and we need to start a new bar
    //                 info!("💓 Heartbeat triggering rollover for {}", asset_id);
    //                 completed_bars.push(data.clone());

    //                 // Update the bar for the NEW window
    //                 data.ts = current_window_start;
    //                 data.open = data.close;
    //                 data.high = data.close;
    //                 data.low = data.close;
    //                 data.volume = 0.0;
                    
    //                 return true; // Keep in map
    //             } else {
    //                 // Market is closed AND volume is 0.
    //                 info!("💤 Hibernating {}: Market closed with no volume.", asset_id);
    //                 return false; // This REMOVES the asset from the LIVE_AGGREGATOR map
    //             }
    //         }
    //         true // Keep assets that are already in the current window
    //     });

    //     completed_bars
    // }
    fn check_heartbeat(&self) -> Vec<MarketData> {
        let mut completed_bars = Vec::new();
        let window_size = 5 * 60 * 1000;
        
        
        // Get time synced with Broker
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
            
        let current_window_start = (now_ms / window_size) * window_size;

        LIVE_AGGREGATOR.retain(|asset_id, data| {
            // Use our new multi-session + recent data check
            let market_is_open = self.check_session_status(asset_id);

            // 1. ZOMBIE CHECK: Market is closed and nothing is happening
            if !market_is_open && data.volume <= 0.0 {
                let idle_time = now_ms - data.ts; 
                if idle_time > 120_000 { 
                    info!("💤 Immediate Hibernation: {} (Market Closed & Idle)", asset_id);
                    return false; // Remove from map
                }
            }

            // 2. WINDOW ROLLOVER CHECK
            if data.ts < current_window_start {
                // Decide: Flush or Hibernate?
                // If the market is open OR we actually have data (volume > 0)
                if market_is_open || data.volume > 0.0 {
                    info!("💓 Heartbeat triggering rollover for {}", asset_id);
                    completed_bars.push(data.clone());

                    // Reset the bar for the NEW window
                    data.ts = current_window_start;
                    // High/Low/Open/Close all stay at the last known price
                    data.open = data.close;
                    data.high = data.close;
                    data.low = data.close;
                    data.volume = 0.0;
                    
                    return true; // Keep in map for the next window
                } else {
                    // Market is closed AND we have no data for the past window.
                    info!("💤 Hibernating {}: Market closed with no volume.", asset_id);
                    return false; // Remove from map
                }
            }
            true // Keep assets in current window
        });

        completed_bars
    }
}