




use chrono::{Utc};
// use surrealdb_types::Value;

// use rdkafka::consumer::{Consumer, StreamConsumer, CommitMode};
// use rdkafka::config::ClientConfig;
// use rdkafka::message::{Message, OwnedMessage};
// use rdkafka::TopicPartitionList;
// use rdkafka::producer::FutureProducer;
// use surrealdb_types::datetime;
// use std::any::Any;
// use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
// use tokio::sync::watch;
use anyhow::{ Result};
use log::{info, error, debug, warn};
// use sqlx::{QueryBuilder, Postgres};


use shared_models::data_model::DataService;
use shared_models::data_model as dm;
use shared_models::market_data::{MarketData};
// use shared_models::traits::MarketDataHandler;
use crate::traits::{DataIngestionExt, DataMaintenanceExt};
use shared_models::time_utils;

use tokio::net::{TcpStream};
use tokio_util::sync::CancellationToken;
// use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
// use dotenvy;
// use std::env;
// use std::sync::atomic::{AtomicI64, Ordering};
// use tokio::time::{interval};
use dashmap::DashSet;
use chrono::{TimeZone};

// use dashmap::DashMap;

use async_trait::async_trait; 
use shared_models::data_model::{ LAST_LIVE_TS, MARKET_SESSIONS};
// use once_cell::sync::Lazy;


// use crate::traits::{DataIngestionExt};
// use futures::StreamExt;
// use smol_str::SmolStr;
// use rustc_hash::FxHashMap;
// use shared_models::data_model::MARKET_SESSIONS;
use crate::watchdog::{Mt5Watchdog,WatchdogState};
use database_engine::runtime::{register_assets};
// use shared_models::db_models::{make_composite_id}; 
use shared_models::db_models as db;
// use surrealdb::{Surreal, engine::local::Mem, engine::local::SurrealKv, opt::auth::Root};
use shared_models::data_model::{IngestionContext};

// use surrealdb::Value; 
// use std::collections::BTreeMap;
use shared_models::db_models::{AppDatabases, MonitorUpdates, SurrealIdExt};
use surrealdb::types::{RecordId, Datetime};
use surrealdb::types::SurrealValue;
use surrealdb::Connection; // Add this import
use dashmap::DashMap;
use std::collections::BTreeMap;
// use surrealdb::sql::RecordId;
// use surrealdb::Error as SurrealError;

//  use serde_json::json;


// use surrealdb::types::{Array, Value,};

// static BROKER_OFFSET: AtomicI64 = AtomicI64::new(0);

// static LIVE_AGGREGATOR: Lazy<DashMap<RecordId, MarketData>> = Lazy::new(DashMap::new);
// static LAST_LIVE_TS: Lazy<DashMap<SmolStr, i64>> = Lazy::new(DashMap::new);
// static TICK_MONITOR: Lazy<DashMap<SmolStr, TickTracker>> = Lazy::new(DashMap::new);

lazy_static::lazy_static! {
    static ref SYNCED_ASSETS: DashSet<RecordId> = DashSet::new();
    // static ref BACKFILL_STATE: DashMap<RecordId, (db::MarketData, u64, u64)> = DashMap::new();
    // static ref BACKFILL_STATE: DashMap<RecordId, (db::MarketData, u64, u64, Vec<db::MarketData>)> = DashMap::new();

    // Tuple: (BarData, BuyCount, SellCount, DiskBatch, PreviousTick)
    static ref BACKFILL_STATE: DashMap<RecordId, (db::MarketData, u64, u64, Vec<db::MarketData>, dm::MultiplexedTick)> = DashMap::new();

}


#[async_trait]

impl DataIngestionExt for DataService {
    


    async fn broker_ingestion(
        &self,
        mut socket: TcpStream,
        dbs: Arc<db::AppDatabases>,
        shutdown: CancellationToken,
        state: WatchdogState,
    )-> db::AppResult<()> {
        info!("MT5 Connected. Initializing Startup Sync...");

        let context = self.load_ingestion_context().map_err(|e| e.to_string())?;

        // 1. Startup Handshake
        self.send_startup_handshake(&dbs, &mut socket, &context.0).await?;

        // 2. Initial Gap Detection (Manual Sync)
        for entry in LAST_LIVE_TS.iter() {
            let (asset_id, last_ts) = entry.pair();
            let now_ms = Utc::now().timestamp_millis();
            let offset = self.get_broker_offset() * 1000;
            let broker_now_ms = now_ms + offset;

            let gap = broker_now_ms - *last_ts;
            if !self.check_session_status(&asset_id) || !Mt5Watchdog::new(state.clone()).is_market_open() {
                return Ok(());
            }
            if gap > 120_000 {
                let asset_name = asset_id.key.to_raw_string();
                warn!("Detection: {} was offline for {}s. Requesting Backfill...", asset_name, gap / 1000);
                let _ = self.request_manual_sync(&mut socket, asset_id, *last_ts).await;
            }
        }

        // 3. Timer Setup
        let get_sleep_duration = || {
            let now = Utc::now().timestamp();
            let seconds_until_next_window = 301 - (now % 300);
            Duration::from_secs(seconds_until_next_window as u64)
        };

        let get_next_window_instant = || {
            let now = Utc::now().timestamp();
            let next_window_ts = ((now / 300) + 1) * 300 + 1;
            let sleep_secs = next_window_ts - now;
            tokio::time::Instant::now() + Duration::from_secs(sleep_secs as u64)
        };

        let mut surreal_batch = Vec::with_capacity(100);
        let mirror_timer = tokio::time::sleep(get_sleep_duration());
        tokio::pin!(mirror_timer);

        let mut header = [0u8; 1];

        // --- MAIN LOOP ---
        loop {
            // Use a Result to capture if any arm fails
            let loop_status: Result<bool, Box<dyn std::error::Error + Send + Sync>> = tokio::select! {
                // SHUTDOWN SIGNAL
                _ = shutdown.cancelled() => {
                    info!("Shutdown signal received. Initiating graceful exit...");
                    Ok(false) // false means "stop the loop"
                }

                // MIRROR TIMER
                _ = &mut mirror_timer => {
                    self.perform_mirror_cycle(&dbs, &state, false).await;
                    let next_instant = get_next_window_instant();
                    mirror_timer.as_mut().reset(next_instant);
                    Ok(true) // true means "keep going"
                }

                // SOCKET TRAFFIC
                read_res = socket.read_exact(&mut header) => {
                    match read_res {
                        Ok(_) => {
                            match header[0] {
                                0 => self.process_tick_header(&mut socket, &context, &state, &dbs).await?,
                                // 1 => self.process_bar_header(&mut socket, &context, &mut surreal_batch, &dbs).await?,
                                1 => self.process_tick_backfill_header(&mut socket, &context, &dbs).await?,
                                
                                3 => self.process_session_header(&mut socket, &context).await?,
                                4 => self.process_dna_header(&mut socket, dbs.clone()).await?,
                                5 => self.process_negotiation_header(&mut socket, &context).await?,
                                254 => self.calibration_header(&mut socket).await?,
                                _ => {
                                    warn!("Desync: Unknown Header {}", header[0]);
                                    return Err("Socket Desync".into());
                                }
                            }
                            Ok(true)
                        }
                        Err(e) => {
                            self.handle_socket_error(e);
                            Err("Socket Disconnected".into()) // Triggers loop exit
                        }
                    }
                }
            };

            // --- THE UNIFIED EXIT HANDLER ---
            match loop_status {
                Ok(true) => continue, // No issues, keep looping
                Ok(false) | Err(_) => {
                    // If it was a clean exit or an error, we MUST flush the mirror before returning
                    if loop_status.is_err() {
                        warn!("Loop exiting due to error. Performing emergency mirror...");
                    }

                    // Final Flush of any pending batches
                    if !surreal_batch.is_empty() {
                        let batch = self.transform_to_standard_market_data(surreal_batch.clone());
                        if let Err(e) = self.ingest_market_data(&dbs.disk, batch).await {
                            error!("Final Ingestion Failed: {}", e);
                        }
                    }

                    // Final Mirror Cycle
                    self.flush_backfill_state(&dbs).await?;
                    self.perform_mirror_cycle(&dbs, &state, true).await;

                    info!("Cleanup complete. System exiting.");
                    break; // Break the loop
                }
            }
        }

        Ok(())
    }


    async fn request_manual_sync(&self, socket: &mut TcpStream, asset_id: &RecordId, start_ts_ms: i64) -> Result<()> {
        let mut packet = Vec::with_capacity(25);
        packet.push(100u8); 

        // Extract the symbol from the first element of the RecordIdKey Array
        let symbol_only = match &asset_id.key {
            surrealdb_types::RecordIdKey::Array(arr) => {
                // arr[0] is the Symbol (e.g., "US2000")
                // arr[1] is the Source (e.g., "FundedNext")
                arr.get(0)
                    .map(|v|{
                        v.to_raw_string().trim_matches(|c| c == '"' || c == '\'').to_string()
                    })
                    .unwrap_or_else(|| "UNKNOWN".to_string())
            },
            _ => {
                // Fallback for flat IDs if any still exist
                let full = asset_id.key.to_raw_string();
                full.split(':').next().unwrap_or(&full).to_string()
            }
        };

        //  Pack the 16-byte buffer with the symbol
        let mut name_buf = [0u8; 16];
        let name_bytes = symbol_only.as_bytes();
        let len = name_bytes.len().min(16);
        name_buf[..len].copy_from_slice(&name_bytes[..len]);
        packet.extend_from_slice(&name_buf);

        //  Time conversion
        let start_ts_secs = start_ts_ms / 1000;
        packet.extend_from_slice(&start_ts_secs.to_le_bytes());

        socket.write_all(&packet).await?;
        socket.flush().await?;
        
    
        info!("Gap Recovery: Requesting [{}] from MT5 (Window Start: {})", symbol_only, start_ts_secs);
        Ok(())
    }


    async fn process_tick_header(
        &self, 
        socket: &mut TcpStream, 
        context: &IngestionContext, 
        state: &WatchdogState,
        // batch: &mut Vec<MarketData>,
        dbs: &Arc<AppDatabases> // 1. Pass your DB handles here
    )  -> db::AppResult<()> {
        let mut buf = [0u8; 64]; 
        socket.read_exact(&mut buf).await?;

        if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {
            // let asset_id = self.resolve_asset_id(&tick.asset, context);
            let source_id = context.0.data_source_id.clone();
            let symbol = String::from_utf8_lossy(&tick.asset).trim_matches(char::from(0)).to_string();
            let record_id = self.get_asset_record_id(&tick.asset, &source_id);
            let broker_offset = self.get_broker_offset();
         

            if !self.check_session_status(&record_id) || !Mt5Watchdog::new(state.clone()).is_market_open() {
                return Ok(());
            }

            LAST_LIVE_TS.insert(record_id.clone(), tick.time_msc);
            // let utc_ts_ms = tick.time_msc - (self.get_broker_offset() * 1000);

            let update = MonitorUpdates {
                asset_id: record_id,
                time_msc: tick.time_msc,
                last_bid: tick.bid,
                last_ask: tick.ask,
                volume: tick.volume as f64,
                time:None,
            };
            state.update();

            if let Err(e) = self.ingest_tick(dbs.clone(), update, &symbol, &source_id,broker_offset).await {
                error!("SurrealDB Ingest Error: {}", e);
            }
        }
        Ok(())
    }

    async fn process_dna_header(
        &self, 
        socket: &mut TcpStream, 
        dbs: Arc<db::AppDatabases>,
    )  -> db::AppResult<()> {
        let mut buf = vec![0u8; std::mem::size_of::<db::AssetInfoPacket>()];
        socket.read_exact(&mut buf).await?;

        if let Ok(packet) = bytemuck::try_from_bytes::<db::AssetInfoPacket>(&buf) {
            if let Err(e) = register_assets(dbs, *packet).await {
                error!("DNA Sync Error: {}", e);
            }
        }
        Ok(())
    }



    async fn process_bar_header(
        &self,
        socket: &mut TcpStream,
        context: &IngestionContext,
        surreal_batch: &mut Vec<MarketData>, 
        dbs: &Arc<AppDatabases>,
    ) -> db::AppResult<()> {
        let mut buf = [0u8; 64];
        socket.read_exact(&mut buf).await?;

        if let Ok(bar) = bytemuck::try_from_bytes::<dm::SyncBar>(&buf) {
            let asset_id = self.get_asset_record_id(&bar.asset, &context.0.data_source_id);
            
            let bar_time_ms = if bar.time < 10_000_000_000 { bar.time * 1000 } else { bar.time };
            let offset_ms = self.get_broker_offset() * 1000;
            let utc_ts_ms = bar_time_ms - offset_ms;

            let surreal_data = MarketData {
                asset_id: asset_id.clone(),
                ts: utc_ts_ms,
                open: bar.open,
                high: bar.high,
                low: bar.low,
                close: bar.close,
                volume: bar.volume as f64,
            };

            // --- 1. Real-time Routing (Memory DB) ---
            // If the candle is from the last 5 minutes, it belongs in RAM for the dashboard/aggregator
            let now_utc = Utc::now().timestamp_millis();
            if (now_utc - utc_ts_ms).abs() < (5 * 60 * 1000) {
                let dbs_clone = Arc::clone(dbs);
                let s_data = vec![surreal_data.clone()]; // Put in a vec for the ingester  
                let service_handle = self.clone(); 
                tokio::spawn(async move {
                    // Transform and Ingest to MEM
                    let mem_batch = service_handle.transform_to_standard_market_data(s_data);
                    if let Err(e) = service_handle.ingest_market_data(&dbs_clone.mem, mem_batch).await {
                        error!("Mem Ingestion Failed: {}", e);
                    }
                });
            }

            // --- 2. Batch Management (Disk DB) ---
            surreal_batch.push(surreal_data);

           if surreal_batch.len() >= 100 {
                // DRAIN the batch so the main thread can keep pushing new data to surreal_batch immediately
                let batch_to_process = surreal_batch.drain(..).collect::<Vec<_>>();
                let disk_service = self.clone();
                let disk_dbs = Arc::clone(dbs);

                // SPAWN Disk Ingestion: This prevents "Broken Pipe" by not blocking the TCP read
                tokio::spawn(async move {
                    let standard_batch = disk_service.transform_to_standard_market_data(batch_to_process);
                    if let Err(e) = disk_service.ingest_market_data(&disk_dbs.disk, standard_batch).await {
                        error!("Disk Batch Ingestion Failed: {}", e);
                    }
                });
            }  
            
        }
        Ok(())
    }

    // async fn process_tick_backfill_header(
    //     &self,
    //     socket: &mut TcpStream,
    //     context: &IngestionContext,
    //     dbs: &Arc<AppDatabases>,
    // ) -> db::AppResult<()> {
    //     let mut buf = [0u8; 64];
    //     socket.read_exact(&mut buf).await?;

    //     if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {
    //         let asset_id = self.get_asset_record_id(&tick.asset, &context.0.data_source_id);
            
    //         // 1. Price Selection (Forex fallback to Bid)
    //         let price = if tick.last > 0.0 { tick.last } else { tick.bid };
    //         let price_key = format!("{:.5}", price);

    //         // 2. Normalize Time to UTC using Broker Offset
    //         let broker_offset_ms = self.get_broker_offset() * 1000;
    //         let utc_time_msc = tick.time_msc - broker_offset_ms;
    //         let bucket_ms = 300_000; // 5 Minute Window
    //         let bar_start_ts = (utc_time_msc / bucket_ms) * bucket_ms;

    //         // 3. Retrieve or Init Aggregation State
    //         let mut entry = BACKFILL_STATE.entry(asset_id.clone()).or_insert_with(|| {
    //             let dt = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
    //             (
    //                 db::MarketData {
    //                     asset_id: asset_id.clone(),
    //                     time: dt,
    //                     open: price, high: price, low: price, close: price,
    //                     volume: tick.volume as f64,
    //                     buy_volume: 0.0, sell_volume: 0.0,
    //                     levels: BTreeMap::new(),
    //                 },
    //                 0, 0 // Initial Tick Counts
    //             )
    //         });

    //         let (ref mut bar, ref mut total_buy_count, ref mut total_sell_count) = *entry;
    //         let current_bar_ts = bar.time.timestamp_millis();

    //         // 4. Check for Window Change (Finalize the finished bar)
    //         if bar_start_ts > current_bar_ts {
    //             let mut finished_bar = bar.clone();
    //             let total_ticks = (*total_buy_count + *total_sell_count) as f64;
                
    //             if total_ticks > 0.0 {
    //                 // Determine how much "Official Volume" each tick represents
    //                 let scale_factor = finished_bar.volume / total_ticks;
                    
    //                 // Finalize Global Buy/Sell Volumes
    //                 finished_bar.buy_volume = *total_buy_count as f64 * scale_factor;
    //                 finished_bar.sell_volume = *total_sell_count as f64 * scale_factor;

    //                 // Scale every Price Level from Counts to Actual Volume
    //                 for (_price, volumes) in finished_bar.levels.iter_mut() {
    //                     // volumes.0 = buy_count, volumes.1 = sell_count (currently)
    //                     volumes.0 *= scale_factor; // Now buy_volume
    //                     volumes.1 *= scale_factor; // Now sell_volume
    //                 }
    //             }

    //             // Ingest finished bar to Disk
    //             let dbs_disk = Arc::clone(&dbs);
    //             let service_handle = self.clone();
    //             tokio::spawn(async move {
    //                 let _ = service_handle.ingest_market_data(&dbs_disk.disk, vec![finished_bar]).await;
    //             });

    //             // 5. Reset Workspace for the New Window
    //             bar.time = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
    //             bar.open = price; bar.high = price; bar.low = price; bar.close = price;
    //             bar.volume = tick.volume as f64;
    //             bar.levels.clear();
    //             *total_buy_count = 0;
    //             *total_sell_count = 0;

    //         } else {
    //             // 6. Update Ongoing Bar
    //             bar.high = bar.high.max(price);
    //             bar.low = bar.low.min(price);
    //             bar.close = price;
    //             bar.volume = tick.volume as f64; // Keep updated to latest official total

    //             // Temporarily store tick counts in the levels map
    //             let level = bar.levels.entry(price_key).or_insert((0.0, 0.0));
    //             if tick.flags & 32 != 0 { 
    //                 level.0 += 1.0; 
    //                 *total_buy_count += 1; 
    //             } else if tick.flags & 64 != 0 { 
    //                 level.1 += 1.0; 
    //                 *total_sell_count += 1; 
    //             }
    //         }
    //     }
    //     Ok(())
    // }



    // async fn process_tick_backfill_header(
    //     &self,
    //     socket: &mut TcpStream,
    //     context: &IngestionContext,
    //     dbs: &Arc<AppDatabases>,
    // ) -> db::AppResult<()> {
    //     let mut buf = [0u8; 64];
    //     socket.read_exact(&mut buf).await?;

    //     if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {
    //         let asset_id = self.get_asset_record_id(&tick.asset, &context.0.data_source_id);
    //         let price = if tick.last > 0.0 { tick.last } else { tick.bid };
    //         let price_key = format!("{:.5}", price);

    //         let broker_offset_ms = self.get_broker_offset() * 1000;
    //         let utc_time_msc = tick.time_msc - broker_offset_ms;
    //         let bucket_ms = 300_000; 
    //         let bar_start_ts = (utc_time_msc / bucket_ms) * bucket_ms;

    //         let mut entry = BACKFILL_STATE.entry(asset_id.clone()).or_insert_with(|| {
    //             let dt = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
    //             (
    //                 db::MarketData {
    //                     asset_id: asset_id.clone(),
    //                     time: dt,
    //                     open: price, high: price, low: price, close: price,
    //                     volume: tick.volume as f64,
    //                     buy_volume: 0.0, sell_volume: 0.0,
    //                     levels: BTreeMap::new(),
    //                 },
    //                 0, 0,
    //                 Vec::with_capacity(10)
    //             )
    //         });

    //         let (ref mut bar, ref mut total_buy_count, ref mut total_sell_count, ref mut batch) = *entry;
    //         let current_bar_ts = bar.time.timestamp_millis();

    //         // --- WINDOW CHANGE: BAR IS FINISHED ---
    //         if bar_start_ts > current_bar_ts {
    //             info!("Bar Closed: pushing to batch");
    //             let mut finished_bar = bar.clone();
                
    //             // 1. Calculate final volumes/deltas
    //             let total_ticks = (*total_buy_count + *total_sell_count) as f64;
    //             if total_ticks > 0.0 {
    //                 let scale_factor = finished_bar.volume / total_ticks;
    //                 finished_bar.buy_volume = *total_buy_count as f64 * scale_factor;
    //                 finished_bar.sell_volume = *total_sell_count as f64 * scale_factor;
    //                 for v in finished_bar.levels.values_mut() { v.0 *= scale_factor; v.1 *= scale_factor; }
    //             }

    //             // 2. Routing Logic (Like the old way)
    //             let now_utc = Utc::now().timestamp_millis();
    //             let is_recent = (now_utc - current_bar_ts).abs() < (5 * 60 * 1000); // Within 5 mins


    //             if is_recent {
    //                 let dbs_mem = Arc::clone(dbs);
    //                 let monitor_data = finished_bar.clone();
                    
    //                 // Cleanly extract the symbol string from the tick bytes
    //                 let symbol = String::from_utf8_lossy(&tick.asset)
    //                     .trim_matches(char::from(0))
    //                     .to_string();
                        
    //                 let monitor_id = db::make_composite_id("monitor", &[&symbol, &context.0.data_source_id]);
                    
    //                 // Capture the current tick's ask to represent the 'current' market state
    //                 let current_ask = tick.ask;

    //                 tokio::spawn(async move {
    //                     // We include asset_id, last_bid, last_ask, and volume to prime the monitor perfectly
    //                     let _ = dbs_mem.mem.query("
    //                         UPSERT $id SET 
    //                             asset_id = $asset, 
    //                             last_bid = $bid, 
    //                             last_ask = $ask, 
    //                             volume = $vol, 
    //                             time = $time
    //                     ")
    //                     .bind(("id", monitor_id))
    //                     .bind(("asset", monitor_data.asset_id))
    //                     .bind(("bid", monitor_data.close)) // The bar close is the last bid
    //                     .bind(("ask", current_ask))        // The ask from the current tick
    //                     .bind(("vol", monitor_data.volume))
    //                     .bind(("time", monitor_data.time))
    //                     .await;
    //                 });
    //             }

    //             // 3. Disk Batch Management
    //             batch.push(finished_bar);
    //             if batch.len() >= 10 {
    //                 let to_send = std::mem::replace(batch, Vec::with_capacity(10));
    //                 let dbs_disk = Arc::clone(&dbs);
    //                 let service_handle = self.clone();
    //                 tokio::spawn(async move {
    //                     let _ = service_handle.ingest_market_data(&dbs_disk.disk, to_send).await;
    //                 });
    //             }

    //             // Reset for New Window
    //             bar.time = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
    //             bar.open = price; bar.high = price; bar.low = price; bar.close = price;
    //             bar.volume = tick.volume as f64;
    //             bar.levels.clear();
    //             *total_buy_count = 0; *total_sell_count = 0;

    //         } else {
    //             // --- UPDATE ONGOING BAR ---
    //             bar.high = bar.high.max(price);
    //             bar.low = bar.low.min(price);
    //             bar.close = price;
    //             bar.volume = tick.volume as f64;

    //             // // Manual Count Logic
    //             // let is_buy = if tick.last > 0.0 { tick.last >= tick.ask } else { true };
    //             // let level = bar.levels.entry(price_key).or_insert((0.0, 0.0));
    //             // if is_buy { level.0 += 1.0; *total_buy_count += 1; } 
    //             // else { level.1 += 1.0; *total_sell_count += 1; }

              

    //             // 1. Determine the "Active Price" for side calculation
    //             // let active_price = if tick.last > 0.0 { tick.last } else { tick.bid };

    //             // // 2. Deterministic Side Logic
    //             // // If price is >= Ask, it's a Buy. 
    //             // // If price is <= Bid, it's a Sell.
    //             // // If it's in between, we check which one it's closer to.
    //             // let is_buy = if active_price >= tick.ask {
    //             //     true
    //             // } else if active_price <= tick.bid {
    //             //     false
    //             // } else {
    //             //     // Price is inside the spread (rare in backfill, but possible)
    //             //     // Compare distance to Bid vs distance to Ask
    //             //     (active_price - tick.bid) > (tick.ask - active_price)
    //             // };

    //             // // 3. Update the Level and Global Counts
    //             // let level = bar.levels.entry(price_key).or_insert((0.0, 0.0));
    //             // if is_buy { 
    //             //     level.0 += 1.0; 
    //             //     *total_buy_count += 1; 
    //             // } else { 
    //             //     level.1 += 1.0; 
    //             //     *total_sell_count += 1; 
    //             // }

    //             // 1. Get the previous price from the bar's current 'close' (which was the last tick's price)
    //             let prev_price = bar.close; 

    //             // 2. Logic: If price went up, it's a buy. If down, it's a sell.
    //             let is_buy = if price > prev_price {
    //                 true
    //             } else if price < prev_price {
    //                 false
    //             } else {
    //                 // If price didn't change (Flat Tick), fall back to Quote Comparison
    //                 // This prevents the "All Zero" problem
    //                 price >= tick.ask 
    //             };

    //             // 3. Update the counts
    //             let level = bar.levels.entry(price_key).or_insert((0.0, 0.0));
    //             if is_buy { 
    //                 level.0 += 1.0; 
    //                 *total_buy_count += 1; 
    //             } else { 
    //                 level.1 += 1.0; 
    //                 *total_sell_count += 1; 
    //             }

    //             // 4. Update close for the NEXT tick's comparison
    //             bar.close = price;
    //         }
    //     }
    //     Ok(())
    // }

    async fn process_tick_backfill_header(
        &self,
        socket: &mut TcpStream,
        context: &IngestionContext,
        dbs: &Arc<AppDatabases>,
    ) -> db::AppResult<()> {
        let mut buf = [0u8; 64];
        socket.read_exact(&mut buf).await?;

        if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {
            let asset_id = self.get_asset_record_id(&tick.asset, &context.0.data_source_id);
            let price = if tick.last > 0.0 { tick.last } else { tick.bid };
            let price_key = format!("{:.5}", price);

            let broker_offset_ms = self.get_broker_offset() * 1000;
            let utc_time_msc = tick.time_msc - broker_offset_ms;
            let bucket_ms = 300_000; // 5m
            let bar_start_ts = (utc_time_msc / bucket_ms) * bucket_ms;

            let mut entry = BACKFILL_STATE.entry(asset_id.clone()).or_insert_with(|| {
                let dt = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
                (
                    db::MarketData {
                        asset_id: asset_id.clone(),
                        time: dt,
                        open: price, high: price, low: price, close: price,
                        volume: tick.volume as f64,
                        buy_volume: 0.0, sell_volume: 0.0,
                        levels: BTreeMap::new(),
                    },
                    0, 0,
                    Vec::with_capacity(10),
                    tick.clone() // Initialize prev_tick with the first tick
                )
            });

            let (ref mut bar, ref mut total_buy_count, ref mut total_sell_count, ref mut batch, ref mut prev_tick) = *entry;
            let current_bar_ts = bar.time.timestamp_millis();

            // --- WINDOW CHANGE: FINALIZE PREVIOUS BAR ---
            if bar_start_ts > current_bar_ts {
                let mut finished_bar = bar.clone();
                
                let total_ticks = (*total_buy_count + *total_sell_count) as f64;
                if total_ticks > 0.0 {
                    let scale_factor = finished_bar.volume / total_ticks;
                    finished_bar.buy_volume = *total_buy_count as f64 * scale_factor;
                    finished_bar.sell_volume = *total_sell_count as f64 * scale_factor;
                    for v in finished_bar.levels.values_mut() { 
                        v.0 *= scale_factor; 
                        v.1 *= scale_factor; 
                    }
                }

                // SEED MEMORY (Check if this bar is within the last 10 mins)
                let now_utc = Utc::now().timestamp_millis();
                if (now_utc - current_bar_ts).abs() < (5 * 60 * 1000) {
                    let dbs_mem = Arc::clone(dbs);
                    let monitor_data = finished_bar.clone();
                    let symbol = String::from_utf8_lossy(&tick.asset).trim_matches(char::from(0)).to_string();
                    let monitor_id = db::make_composite_id("monitor", &[&symbol, &context.0.data_source_id]);
                    let current_ask = tick.ask;

                    tokio::spawn(async move {
                        let _ = dbs_mem.mem.query("UPSERT $id SET 
                            asset_id = $asset, last_bid = $bid, last_ask = $ask, volume = $vol, time = $time")
                            .bind(("id", monitor_id))
                            .bind(("asset", monitor_data.asset_id))
                            .bind(("bid", monitor_data.close))
                            .bind(("ask", current_ask))
                            .bind(("vol", monitor_data.volume))
                            .bind(("time", monitor_data.time))
                            .await;
                    });
                }

                // PUSH TO DISK BATCH
                batch.push(finished_bar);
                if batch.len() >= 10 {
                    let to_send = std::mem::replace(batch, Vec::with_capacity(10));
                    let dbs_disk = Arc::clone(&dbs);
                    let service_handle = self.clone();
                    tokio::spawn(async move {
                        let _ = service_handle.ingest_market_data(&dbs_disk.disk, to_send).await;
                    });
                }

                // RESET FOR NEW WINDOW
                bar.time = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
                bar.open = price; bar.high = price; bar.low = price; bar.close = price;
                bar.volume = tick.volume as f64;
                bar.levels.clear();
                *total_buy_count = 0; *total_sell_count = 0;

            } else {
                // --- UPDATE ONGOING BAR ---
                bar.high = bar.high.max(price);
                bar.low = bar.low.min(price);
                
                // SIDE LOGIC: Mirroring SurrealDB (Quote Movement)
                let bid_diff = tick.bid - prev_tick.bid;
                let ask_diff = tick.ask - prev_tick.ask;

                let is_buy = if ask_diff > 0.0 {
                    true  // Ask moving up = Buying pressure
                } else if bid_diff < 0.0 {
                    false // Bid moving down = Selling pressure
                } else {
                    // Fallback: If quotes are static, check price relative to spread
                    price >= tick.ask 
                };

                let level = bar.levels.entry(price_key).or_insert((0.0, 0.0));
                if is_buy { 
                    level.0 += 1.0; 
                    *total_buy_count += 1; 
                } else { 
                    level.1 += 1.0; 
                    *total_sell_count += 1; 
                }

                // Update state for next tick
                bar.close = price;
                bar.volume = tick.volume as f64;
                *prev_tick = tick.clone(); 
            }
        }
        Ok(())
    }
    async fn flush_backfill_state(&self, dbs: &Arc<AppDatabases>) -> db::AppResult<()> {
        let mut bars_to_flush = Vec::new();

        // Take everything currently in the accumulator
        for mut entry in BACKFILL_STATE.iter_mut() {
            let (bar, buy_cnt, sell_cnt, batch,prev_tic,) = entry.value_mut();
            let mut final_bar = bar.clone();
            
            let total_ticks = (*buy_cnt + *sell_cnt) as f64;
            if total_ticks > 0.0 {
                let scale_factor = final_bar.volume / total_ticks;
                final_bar.buy_volume = *buy_cnt as f64 * scale_factor;
                final_bar.sell_volume = *sell_cnt as f64 * scale_factor;

                for (_price, volumes) in final_bar.levels.iter_mut() {
                    volumes.0 *= scale_factor;
                    volumes.1 *= scale_factor;
                }
            }
            batch.push(final_bar);
        }

        if !bars_to_flush.is_empty() {
            let count = bars_to_flush.len();
            self.ingest_market_data(&dbs.disk, bars_to_flush).await?;
            info!("Final Backfill Flush: {} bars pushed to disk.", count);
            BACKFILL_STATE.clear();
        }
        Ok(())
    }

//     async fn process_tick_backfill_header(
//         &self,
//         socket: &mut TcpStream,
//         context: &IngestionContext,
//         dbs: &Arc<AppDatabases>,
//     ) -> db::AppResult<()> {
//         let mut buf = [0u8; 64];
//         socket.read_exact(&mut buf).await?;

//         if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {
//             let asset_id = self.get_asset_record_id(&tick.asset, &context.0.data_source_id);
            
//             // Price Selection: Use 'last' for exchange data, fallback to 'bid' for Forex
//             let price = if tick.last > 0.0 { tick.last } else { tick.bid };

//             let broker_offset_ms = self.get_broker_offset() * 1000;
//             let utc_time_msc = tick.time_msc - broker_offset_ms;
//             let bucket_ms = 300_000; 
//             let bar_start_ts = (utc_time_msc / bucket_ms) * bucket_ms;

//             let mut entry = BACKFILL_STATE.entry(asset_id.clone()).or_insert_with(|| {
//                 let dt = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
//                 (
//                     db::MarketData {
//                         asset_id: asset_id.clone(),
//                         time: dt,
//                         open: price, high: price, low: price, close: price,
//                         volume: tick.volume as f64,
//                         buy_volume: 0.0, 
//                         sell_volume: 0.0,
//                     },
//                     0, 0 
//                 )
//             });

//             let (ref mut bar, ref mut buy_count, ref mut sell_count) = *entry;
//             let current_bar_ts = bar.time.timestamp_millis();

//             if bar_start_ts > current_bar_ts {
//                 // --- FINALIZE OLD BAR ---
//                 let mut finished_bar = bar.clone();
//                 let total_ticks = (*buy_count + *sell_count) as f64;
                
//                 if total_ticks > 0.0 {
//                     let ratio = *buy_count as f64 / total_ticks;
//                     finished_bar.buy_volume = finished_bar.volume * ratio;
//                     finished_bar.sell_volume = finished_bar.volume - finished_bar.buy_volume;
//                 } else {
//                     // If no flags were found, split 50/50 as a fallback
//                     finished_bar.buy_volume = finished_bar.volume / 2.0;
//                     finished_bar.sell_volume = finished_bar.volume / 2.0;
//                 }

//                 let dbs_disk = Arc::clone(&dbs);
//                 let service_handle = self.clone();
//                 tokio::spawn(async move {
//                     let _ = service_handle.ingest_market_data(&dbs_disk.disk, vec![finished_bar]).await;
//                 });

//                 // --- RESET FOR NEW BAR ---
//                 bar.time = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
//                 bar.open = price; bar.high = price; bar.low = price; bar.close = price;
//                 bar.volume = tick.volume as f64;
//                 *buy_count = 0; *sell_count = 0;
//             } else {
//                 // --- UPDATE EXISTING BAR ---
//                 bar.high = bar.high.max(price);
//                 bar.low = bar.low.min(price);
//                 bar.close = price;
//                 bar.volume = tick.volume as f64; // Anchor to official volume

//                 // Flag 32 = TICK_FLAG_BUY | Flag 64 = TICK_FLAG_SELL
//                 if tick.flags & 32 != 0 { *buy_count += 1; }
//                 else if tick.flags & 64 != 0 { *sell_count += 1; }
//             }
//         }
//         Ok(())
// }

    async fn process_session_header(
        &self,
        socket: &mut TcpStream,
        context: &IngestionContext,
    )  -> db::AppResult<()> {
        let mut buf = [0u8; 40]; // Matches dm::SessionPacket size
        socket.read_exact(&mut buf).await?;

        if let Ok(packet) = bytemuck::try_from_bytes::<dm::SessionPacket>(&buf) {
            // let asset_id = self.resolve_asset_id(&packet.asset, context);
             let asset_id = self.get_asset_record_id(&packet.asset, &context.0.data_source_id);

            debug!("Session Update: {} | Index: {} | Active: {} | {:02}:{:02}-{:02}:{:02}", 
                asset_id.to_raw_string(), packet.session_index, packet.is_active,
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
                if let Some((id, _)) = LAST_LIVE_TS.remove(&asset_id) {
                    info!("Hibernating {}: Market is closed.", id.to_raw_string());
                }
            }
        }
        Ok(())
    }

    async fn process_negotiation_header(
        &self, 
        socket: &mut TcpStream, 
        context: &IngestionContext,
    )  -> db::AppResult<()> {
        let mut buf = [0u8; 64];
        socket.read_exact(&mut buf).await?;
    

        let asset_record_id = self.get_asset_record_id(&buf[0..10], &context.0.data_source_id);

        
        let is_live = LAST_LIVE_TS.contains_key(&asset_record_id) || SYNCED_ASSETS.contains(&asset_record_id);
        let is_active = MARKET_SESSIONS.get(&asset_record_id)
            .and_then(|s| s.first().map(|p| p.is_active == 1))
            .unwrap_or(false);

        let sync_needed_byte = if is_live && is_active { 1u8 } else { 0u8 };
        socket.write_all(&[sync_needed_byte]).await?;
        Ok(())
    }


    fn handle_socket_error(&self, err: std::io::Error) {
        use std::io::ErrorKind;

        match err.kind() {
            // 1. The "Clean" Disconnect (Terminal Closed)
            ErrorKind::UnexpectedEof | ErrorKind::ConnectionAborted => {
                info!("MT5 Connection closed gracefully by peer (EOF).");
            }

            // 2. The "Dirty" Disconnect (Network Crash / Timeout)
            ErrorKind::ConnectionReset => {
                warn!("MT5 Connection reset by peer. The remote host may have crashed or restarted.");
            }

            // 3. The "Network Failure" (Cable unplugged / Firewall)
            ErrorKind::BrokenPipe => {
                error!("Broken Pipe: The connection was lost before data could be sent.");
            }

            // 4. The "Busy" Error (Shouldn't happen with read_exact, but good for safety)
            ErrorKind::WouldBlock | ErrorKind::TimedOut => {
                debug!("Socket operation timed out or would block. Retrying...");
            }

            // 5. Everything else
            _ => {
                error!("Unexpected Socket Error: [{:?}] {}", err.kind(), err);
            }
        }
    }


    async fn ingest_tick(
        &self,
        dbs: Arc<AppDatabases>, 
        mut update: MonitorUpdates, // We take ownership so we can modify it
        symbol: &str,
        source: &str,
        broker_offset_seconds: i64
    ) -> surrealdb::Result<()> {
        
        // 1. Convert MT5 millis to normalized Surreal Datetime
        let adjusted_millis = update.time_msc - (broker_offset_seconds * 1000);
        
        let datetime = Utc.timestamp_millis_opt(adjusted_millis)
            .single()
            .map(Datetime::from)
            .unwrap_or_else(|| Datetime::default());

        // 2. Attach the normalized time to the struct before sending
        update.time = Some(datetime);

        // 3. Generate the Composite RecordId
        let monitor_id = db::make_composite_id("monitor", &[&symbol, &source]);

        // 4. Perform the UPSERT with a direct struct bind
        // dbs.mem
        //     .query("UPSERT $id MERGE $data")
        //     .bind(("id", monitor_id))
        //     .bind(("data", update)) // This uses the #[serde] rules to rename fields automatically
        //     .await?
        //     .check()?;

        dbs.mem
            .query("UPSERT $id SET 
                asset_id = $asset, 
                last_bid = $bid, 
                last_ask = $ask, 
                volume = $vol, 
                time = $time")
            .bind(("id", monitor_id))
            .bind(("asset", update.asset_id))
            .bind(("bid", update.last_bid))
            .bind(("ask", update.last_ask))
            .bind(("vol", update.volume))
            .bind(("time", datetime))
            .await?
            .check()?;
            
        Ok(())
    }


   

    fn get_asset_record_id(&self, asset_bytes: &[u8], source: &str) -> RecordId {

        let name = std::str::from_utf8(asset_bytes).unwrap_or("").trim_matches(char::from(0));
       
        let asset_id = db::make_composite_id("assets",&[&name, &source]);
       
        asset_id
    }


    async fn perform_mirror_cycle(
        &self,
        dbs: &Arc<db::AppDatabases>,
        state: &WatchdogState,
        is_shutdown: bool
    ) {
        // 1. Guard: If market is closed and it's not a shutdown, save CPU and exit
        if !is_shutdown && !Mt5Watchdog::new(state.clone()).is_market_open() {
            return;
        }

        if let Err(_) = dbs.mem.health().await {
            warn!("[Mirror] DB connection closed, skipping final mirror.");
            return;
        }

        // 2. Calculate the target window (the 5m block that just closed)
        let now = Utc::now().timestamp();
        let target_raw = if is_shutdown { now - (now % 300) } else { now - (now % 300) - 300 };
        
        let target_ts = Datetime::from(
            chrono::DateTime::from_timestamp(target_raw, 0).unwrap_or_default()
        );

        for entry in LAST_LIVE_TS.iter() {
            let (asset_id, _) = entry.pair();
                
            // Skip if already flushed this window
            // if last_flushed.get(asset_id).map(|ts| *ts == target_ts).unwrap_or(false) {
            //     continue;
            // }
            // 3. Select ALL candles for this window from MEM node
            let query = "SELECT * FROM market_data WHERE asset_id = $asset AND time = $ts";

            match 
            dbs.mem
            .query(query)
            .bind(("asset",asset_id.clone()))
            .bind(("ts", target_ts.clone()))
            .await {
                Ok(mut res) => {
                    let data: Vec<db::MarketData> = res.take(0).unwrap_or_default();
                    
                    if !data.is_empty() {
                        let count = data.len();
                        // 4. Use the Universal Ingester we built
                        if let Err(e) = self.ingest_market_data(&dbs.disk, data).await {
                            error!("[Mirror] Failed to flush {} records: {}", count, e);
                        } else {
                            info!("[Mirror] Successfully flushed {} candles for window {} to Disk", count, target_ts);
                        }
                    }
                },
                Err(e) => error!("[Mirror] Mem query failed: {}", e),
            }
        }
    }

    async fn ingest_market_data<C: Connection>(
        &self,
        db: &surrealdb::Surreal<C>,
        data: Vec<db::MarketData>,
    ) -> surrealdb::Result<()> {
        if data.is_empty() { return Ok(()); }

        // 1. Deduplication based on Asset and Time
        // We use a HashMap to ensure only the latest version of a specific candle exists
        let mut dedup_map = rustc_hash::FxHashMap::default();
        for  candle in data {
            // Unique key: (Asset RecordId, Timestamp)
            let key = (candle.asset_id.clone(), candle.time.clone());
            dedup_map.insert(key, candle);
        }

        
        let unique_count = dedup_map.len();
        

        // 2. Map to Ingest Rows with Composite IDs
        let all_rows: Vec<db::CandleIngestRow> = dedup_map
            .into_values()
            .map(|c| {
                // let symbol = match c.asset_id.clone().key {
                //     surrealdb_types::RecordIdKey::Array(arr) => arr.first().unwrap().to_raw_string(),
                //     _ => "UNKNOWN".to_string(),
                // };
                let symbol = match c.asset_id.clone().key {
                    surrealdb_types::RecordIdKey::Array(ref arr) => {
                        arr.first()
                        .map(|v| v.to_raw_string())
                        .unwrap_or_else(|| "ARRAY_BUT_EMPTY".to_string())
                    }
                    // Change this to capture the key so you can see what it IS
                    other => {
                        warn!("Unexpected key format found: {:?}", other);
                        "NOT_ARRAY_TYPE".to_string()
                    }
                };
                let id = db::make_composite_id("market_data", &[&symbol, &c.time]);
                db::CandleIngestRow { id, data: c }
            })
            .collect();

        // let batch_value = all_rows.into_value();

        // 3. Chunked Upsert
        for chunk in all_rows.chunks(5000) {
            let query = "FOR $row in $batch { UPSERT $row.id CONTENT $row.data };";
            let batch_vec = chunk.to_vec();
            let batch_value = batch_vec.into_value();

            db.query(query)
                .bind(("batch", batch_value))
                .await?
                .check()?;
        }

        info!("Market Data Ingested: {} unique records processed.", unique_count);
        Ok(())
    }

    fn transform_to_standard_market_data(&self, batch: Vec<MarketData>) -> Vec<db::MarketData> {
        batch.into_iter()
            .map(|item| db::MarketData {
                asset_id: item.asset_id,
                // Convert i64 millis -> Chrono Utc -> Surreal Datetime
                time: time_utils::ts_to_utc_datetime(item.ts)
                    .map(Datetime::from)
                    .unwrap_or_else(|_| Datetime::default()),
                open: item.open,
                high: item.high,
                low: item.low,
                close: item.close,
                volume: item.volume,
                buy_volume:item.volume,
                sell_volume:item.volume,
                levels: BTreeMap::new(),
            })
            .collect()
    }

    // fn transform_to_standard_market_data(&self, batch: Vec<MarketData>) -> Vec<db::MarketData> {
    //     batch.into_iter()
    //         .map(|item| db::MarketData {
    //             asset_id: item.asset_id,
    //             time: time_utils::ts_to_utc_datetime(item.ts)
    //                 .map(Datetime::from)
    //                 .unwrap_or_else(|_| Datetime::default()),
    //             open: item.open,
    //             high: item.high,
    //             low: item.low,
    //             close: item.close,
    //             volume: item.volume,
    //             // Map the new fields from your internal MarketData to the DB struct
    //             buy_volume: item.buy_volume,
    //             sell_volume: item.sell_volume,
    //         })
    //         .collect()
    // }
}