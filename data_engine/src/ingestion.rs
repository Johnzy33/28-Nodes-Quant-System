
use chrono::{Utc};

use std::sync::Arc;
use std::time::Duration;
use anyhow::{ Result};
use log::{info, error, debug, warn};

use shared_models::data_model::DataService;
use shared_models::data_model as dm;
use shared_models::market_data::{MarketData};

use crate::traits::{DataIngestionExt, DataMaintenanceExt};
use shared_models::time_utils;
use tokio::net::{TcpStream};
use tokio_util::sync::CancellationToken;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use chrono::{TimeZone};
use async_trait::async_trait; 
use shared_models::data_model::{ LAST_LIVE_TS, MARKET_SESSIONS, BACKFILL_STATE, SYNCED_ASSETS};

use crate::watchdog::{Mt5Watchdog,WatchdogState};
use database_engine::runtime::{register_assets};
use shared_models::db_models as db;

use shared_models::data_model::{IngestionContext};

use shared_models::db_models::{AppDatabases, MonitorUpdates, SurrealIdExt};
use surrealdb::types::{RecordId, Datetime};
use surrealdb::types::SurrealValue;
use surrealdb::Connection; 
use dashmap::DashMap;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool};



lazy_static::lazy_static! {

    static ref FLUSH_TRACKER: DashMap<RecordId, i64> = DashMap::new();
    static ref MONITOR_LOCKS: DashMap<RecordId, Arc<AtomicBool>> = DashMap::new();

}



#[async_trait]

impl DataIngestionExt for DataService {
    

    async fn broker_ingestion(
        &self,
        mut socket: TcpStream,
        dbs: Arc<db::AppDatabases>,
        shutdown: CancellationToken,
        state: WatchdogState,
    ) -> db::AppResult<()> {
        info!("MT5 Connected. Waiting for Calibration before Handshake...");

        let context = self.load_ingestion_context().map_err(|e| e.to_string())?;
        
        // Track initialization state
        let mut is_initialized = false;
        

        // Timer Setup (Standard 5m window)
        let get_next_window_instant = || {
            let now = Utc::now().timestamp();
            let next_window_ts = ((now / 300) + 1) * 300 + 1;
            let sleep_secs = next_window_ts - now;
            tokio::time::Instant::now() + Duration::from_secs(sleep_secs as u64)
        };

        let mirror_timer = tokio::time::sleep(Duration::from_secs(301 - (Utc::now().timestamp() % 300) as u64));
        tokio::pin!(mirror_timer);

        let mut header = [0u8; 1];

        // --- MAIN LOOP ---
        loop {
            let loop_status: Result<bool, Box<dyn std::error::Error + Send + Sync>> = tokio::select! {
                // 1. SHUTDOWN SIGNAL
                _ = shutdown.cancelled() => {
                    info!("Shutdown signal received. Initiating graceful exit...");
                    Ok(false)
                }

                // 2. MIRROR TIMER
                _ = &mut mirror_timer => {
                    
                    self.perform_mirror_cycle(&dbs, &state, false).await;
                    
                    mirror_timer.as_mut().reset(get_next_window_instant());
                    Ok(true)
                }

                // 3. SOCKET TRAFFIC
                read_res = socket.read_exact(&mut header) => {
                    match read_res {
                        Ok(_) => {
                            match header[0] {
                                // --- CALIBRATION (THE UNLOCKER) ---
                                254 => {
                                    self.calibration_header(&mut socket).await?;
                                    
                                    // First time calibration triggers the Handshake
                                    if !is_initialized {
                                        let offset = self.get_broker_offset();
                                        info!("Calibration Success: Broker is GMT+{}. Dispatching Handshake...", offset / 3600);
                                        
                                        // Step A: Send Handshake (Now with correct Offset!)
                                        self.send_startup_handshake(&dbs, &mut socket, &context.0).await?;
                                        
                                        self.prime_ingestion_state(&context);
                                        // Step B: Run Gap Detection
                                        self.run_startup_gap_detection(&mut socket, &state).await?;
                                        
                                        is_initialized = true;
                                    }
                                    Ok(true)
                                }

                                // --- DATA HEADERS ---
                                0 => self.process_tick_header(&mut socket, &context, &state, &dbs).await.map(|_| true).map_err(|e| e.into()),
                               
                                1 => {
                                    
                                    self.process_tick_backfill_headerbatch(&mut socket, &context, &dbs).await.map(|_| true).map_err(|e| e.into())
                                       
                                },
                                
                                2 => {
                                    // info!("Received sync complete header for asset {}.", &context.0.assets[0].mt5_symbol);
                                    // tokio::time::sleep(Duration::from_millis(500)).await;
                                    self.handle_sync_complete(&mut socket, &dbs, &state).await.map(|_| true).map_err(|e| e.into())
                                    // let mut sym_buf = [0u8; 16];
                                    // socket.read_exact(&mut sym_buf).await?;
                                    // info!("Sync Complete Signal received for asset. Buffers already flushed by aggregator.");
                                    // Ok(true)
                                },


                                // --- SYSTEM HEADERS ---
                                3 => self.process_session_header(&mut socket, &context).await.map(|_| true).map_err(|e| e.into()),
                                4 => self.process_dna_header(&mut socket, dbs.clone()).await.map(|_| true).map_err(|e| e.into()),
                                5 => self.process_negotiation_header(&mut socket, &context).await.map(|_| true).map_err(|e| e.into()),
                                
                                _ => {
                                    warn!("Desync: Unknown Header {}", header[0]);
                                    Err("Socket Desync".into())
                                }
                            }
                        }

                        // Inside tokio::select! -> read_res match
                        Err(e) => {
                            let kind = e.kind();
                            self.handle_socket_error(e); 

                            if kind == std::io::ErrorKind::UnexpectedEof || kind == std::io::ErrorKind::ConnectionAborted {
                                // Signal the loop to stop WITHOUT triggering an "Emergency Mirror" error
                                Ok(false) 
                            } else {
                                // This is a real error (Broken Pipe, Reset, etc.)
                                Err("Socket Disconnected".into())
                            }
                        }
                   
                    }
                }
            };

            // --- EXIT HANDLER ---
            match loop_status {
                Ok(true) => continue, // Keep looping
                Ok(false) => {
                    // This was a GRACEFUL disconnect (Worker finished)
                    info!("Connection closed normally. Thread shutting down.");
                    break; // Exit the loop, function returns Ok at the bottom
                }
                Err(e) => {
                    // This was a CRASH (Broken pipe, etc)
                    warn!("Loop exiting due to error: {}. Performing emergency mirror...", e);
                    self.perform_mirror_cycle(&dbs, &state, true).await;
                    break; 
                }
            }
        }
        Ok(())
    }

    async fn run_startup_gap_detection(
        &self, 
        socket: &mut TcpStream, 
        state: &WatchdogState
    ) -> db::AppResult<()> {
        let offset_ms = self.get_broker_offset() as i64 * 1000;
        let broker_now_ms = Utc::now().timestamp_millis() + offset_ms;

        for entry in LAST_LIVE_TS.iter() {
            let (asset_id, last_ts) = entry.pair();
            let gap = broker_now_ms - *last_ts;

            if gap > 120_000 && self.check_session_status(&asset_id) {
                warn!("Gap Detected for {}: {}s. Requesting Sync...", asset_id.to_raw_string(), gap / 1000);
                self.request_manual_sync(socket, asset_id, *last_ts).await?;
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
        dbs: &Arc<AppDatabases> 
    ) -> db::AppResult<()> {
        let mut buf = [0u8; 64]; 
        socket.read_exact(&mut buf).await?;

        if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {
            let record_id = self.get_asset_record_id(&tick.asset, &context.0.data_source_id);
            let symbol = String::from_utf8_lossy(&tick.asset).split('\0').next().unwrap_or("").trim().to_string();
            let broker_offset = self.get_broker_offset();
            let tick_window_ms = ((tick.time_msc - (broker_offset * 1000)) / 300_000) * 300_000;
            
            
            if SYNCED_ASSETS.contains(&record_id) {
                let mut entry = match BACKFILL_STATE.get_mut(&record_id) {
                    Some(e) => e,
                    None => return Ok(()), 
                };

                // let (ref mut bar, _, _, _, ref mut prev_m_tick) = *entry;
                let (ref mut bar, ref mut buy_count, ref mut sell_count, ref mut batch, ref mut prev_m_tick) = *entry;
                let bar_ts = bar.time.timestamp_millis();

                // 1. Priming
                if bar_ts == 1000 {
                    info!("Priming initial buffer for SYNCED asset {}, Window: {}", symbol, tick_window_ms);
                    bar.time = Datetime::from_timestamp(tick_window_ms / 1000, 0).unwrap_or_default();
                    bar.open = tick.bid; bar.high = tick.bid; bar.low = tick.bid; bar.close = tick.bid;
                    *prev_m_tick = tick.clone();
                    return Ok(());
                }

                // 2. THE REFINED GATE: Target-Based Exit
                let current_live_window = (Utc::now().timestamp_millis() / 300_000) * 300_000;
                let previous_live_window = current_live_window - 300_000;

                // Condition A: The tick we just received is part of the LIVE window (or future)
                let tick_is_live = tick_window_ms >= current_live_window;

                // Condition B: The bar we have in RAM is also at the LIVE edge
                let bar_is_live = bar_ts >= previous_live_window;

                // Condition C: We just saw a window transition (e.g., 01:20 -> 01:25)
                let is_transition = tick_window_ms > bar_ts;

                // Condition D: Backfill is NOT currently busy with historical bars (This prevents handoff during a large historical flush)
                let backfill_is_busy = !batch.is_empty(); 



                // 2. Handoff Guard: Only handoff if tick is NEWER than bar AND bar is already "LIVE"
                if tick_is_live && bar_is_live && is_transition && !backfill_is_busy  {
                    drop(entry); // Release lock before removal
                    if let Some((_, entry_to_flush)) = BACKFILL_STATE.remove(&record_id) {
                        let mut completed_bar = entry_to_flush.0;
                        let buy_count = entry_to_flush.1 as f64;
                        let sell_count = entry_to_flush.2 as f64;
                        let total_ticks = (buy_count + sell_count) as f64;
                        let dbs_disk = Arc::clone(dbs);
                        let service_handle = self.clone();
                        

                         // SCALE VOLUME: Ensures buy_vol + sell_vol = total volume
                        if total_ticks > 0.0 {
                            let scale = completed_bar.volume / total_ticks;
                            completed_bar.buy_volume = (buy_count * scale).round();
                            completed_bar.sell_volume = (completed_bar.volume - completed_bar.buy_volume).max(0.0);
                            
                            // Also scale the price levels for a consistent footprint
                            for v in completed_bar.levels.values_mut() {
                                v.0 = (v.0 * scale).round();
                                v.1 = (v.1 * scale).round();
                            }
                        }

                        tokio::spawn(async move {
                            let _ = service_handle.ingest_market_data(&dbs_disk.disk, vec![completed_bar]).await;
                        });
                    }
                    SYNCED_ASSETS.remove(&record_id.clone());
                    info!("Handoff Complete for {}", symbol);
                } else {
                    // 3. Live Update: Belongs to the current bar in BACKFILL_STATE
                    // (This handles the case where backfill and live are currently in the same window)
                    let price = tick.bid;
                    bar.high = bar.high.max(price);
                    bar.low = bar.low.min(price);
                    bar.close = price;
                    
                    // Volume logic...
                    let is_buy = if tick.ask > prev_m_tick.ask { true } 
                                else if tick.bid < prev_m_tick.bid { false } 
                                else { tick.bid >= tick.ask };
                    let vol_delta = (tick.volume as f64 - prev_m_tick.volume as f64).max(0.0);
                    
                    let level = bar.levels.entry(format!("{:.5}", price)).or_insert((0.0, 0.0));
                    if is_buy { bar.buy_volume += vol_delta; level.0 += 1.0; *buy_count += 1; } 
                    else { bar.sell_volume += vol_delta; level.1 += 1.0; *sell_count += 1; }

                    bar.volume = tick.volume as f64;
                    *prev_m_tick = tick.clone();
                    return Ok(());
                }
            }

            // Standard Monitor Path
            let update = MonitorUpdates {
                asset_id: record_id.clone(),
                time_msc: tick.time_msc,
                last_bid: tick.bid, last_ask: tick.ask,
                volume: tick.volume as f64,
                time: None, flags: Some(tick.flags),
            };
            state.update();
            LAST_LIVE_TS.insert(record_id, tick.time_msc);
            let _ = self.ingest_tick(dbs.clone(), update, &symbol, &context.0.data_source_id, "monitor", broker_offset).await;
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


    async fn process_tick_backfill_headerbatch(
        &self,
        socket: &mut TcpStream,
        context: &IngestionContext,
        dbs: &Arc<AppDatabases>,
    ) -> db::AppResult<()> {
        // 1. Read the 4-byte count (How many ticks are coming?)
        let mut count_buf = [0u8; 4];
        socket.read_exact(&mut count_buf).await?;
        let count = u32::from_le_bytes(count_buf) as usize;

        if count == 0 { 
            info!("Backfill batch count is 0, no ticks to process");
            return Ok(()); 
        }

        // 2. Calculate total bytes (count * 64)
        let total_bytes = count * 64;
        let mut data_buf = vec![0u8; total_bytes];

        
        // 3. FORCE read the entire batch. This is the "Desync Killer"
        socket.read_exact(&mut data_buf).await?;

        // 4. Zero-copy cast and process
        let ticks: &[dm::MultiplexedTick] = bytemuck::cast_slice(&data_buf);
        let asset_id = self.get_asset_record_id(&ticks[0].asset, &context.0.data_source_id);

        // let symbol = String::from_utf8_lossy(&ticks[0].asset).trim_matches(char::from(0)).to_string();
        // Use this logic everywhere you convert tick.asset to a String
        let symbol = String::from_utf8_lossy(&ticks[0].asset)
            .split('\0')             // Stop at the first null byte
            .next()                  // Take the first part
            .unwrap_or("")
            .trim()                  // Remove any whitespace
            .to_string();

        info!("Expecting backfill batch of {} ticks ({} bytes, for asset {})", count, total_bytes, symbol);
 
        // PROCESS IN CHUNKS to prevent blocking the async runtime
        for (i, tick) in ticks.iter().enumerate() {

            let finished_history: bool = self.handle_backfill_aggregation(tick, context, dbs).await?;

            if finished_history {
                // BACKFILL COMPLETE: Flush any remaining bars in the batch
                info!("Backfill complete for {}. Performing final cleanup.", symbol);
                self.flush_backfill_asset(&asset_id, dbs).await;

                return Ok(()); 
            }
            
            // Every 1000 ticks, yield to allow the heartbeat/other tasks to run
            if i % 1000 == 0 {
                tokio::task::yield_now().await;
            }
        }

        Ok(())
    }


    async fn handle_backfill_aggregation(
        &self,
        tick: &dm::MultiplexedTick,
        context: &IngestionContext,
        dbs: &Arc<AppDatabases>,
    ) -> db::AppResult<bool> { // Returns true if we reached the live edge
        let asset_id = self.get_asset_record_id(&tick.asset, &context.0.data_source_id);
        let symbol = String::from_utf8_lossy(&tick.asset)
            .split('\0').next().unwrap_or("").trim().to_string();
        let is_special = matches!(symbol.as_str(), "US2000" | "XAUUSD" | "XAGUSD" | "JP225" | "SILVER" | "GOLD");

        let price = if tick.last > 0.0 { tick.last } else { tick.bid };
        let broker_offset_ms = self.get_broker_offset() * 1000;
        let bar_start_ts = ((tick.time_msc - broker_offset_ms) / 300_000) * 300_000;
        let current_live_window = (Utc::now().timestamp_millis() / 300_000) * 300_000;

        // --- LIVE EDGE SIGNAL ---
        // If this tick belongs to the current live window, we signal completion of backfill
        let is_live_tick = bar_start_ts >= current_live_window;

        let entry_opt = BACKFILL_STATE.get_mut(&asset_id);

        if entry_opt.is_none() {
            let new_bar = db::MarketData {
                time: Datetime::from_timestamp(bar_start_ts / 1000, 0).unwrap_or_default(),
                open: price, high: price, low: price, close: price,
                volume: tick.volume as f64,
                asset_id: asset_id.clone(),
                ..Default::default()
            };
            BACKFILL_STATE.insert(asset_id.clone(), (new_bar, 0u64, 0u64, Vec::new(), tick.clone()));
            return Ok(is_live_tick);
        }

        let mut entry = entry_opt.unwrap();
        let (ref mut bar, ref mut buy_count, ref mut sell_count, ref mut batch, ref mut prev_tick) = *entry;

        // --- WINDOW PIVOT LOGIC ---
        if bar_start_ts != bar.time.timestamp_millis() {
            let is_dummy = bar.time.timestamp_millis() == 1000;

            if !is_dummy && (*buy_count + *sell_count) > 0 {
                let mut finished_bar = bar.clone();
                let total_ticks = (*buy_count + *sell_count) as f64;

                if total_ticks > 0.0 {
                    if is_special {
                        let scale = finished_bar.volume / total_ticks;
                        finished_bar.buy_volume = (*buy_count as f64 * scale).round();
                        finished_bar.sell_volume = (finished_bar.volume - finished_bar.buy_volume).max(0.0);
                    } else {
                        finished_bar.buy_volume = *buy_count as f64;
                        finished_bar.sell_volume = (finished_bar.volume - finished_bar.buy_volume).max(0.0);
                    }
                }

                // Only archive if it's a historical window.
                if finished_bar.time.timestamp_millis() < current_live_window {
                    batch.push(finished_bar);
                }
            }

            // Standard batch flush logic
            if batch.len() >= 20 {
                let to_send = std::mem::replace(batch, Vec::with_capacity(20));
                let dbs_disk = Arc::clone(&dbs);
                let service_handle = self.clone();
                tokio::spawn(async move {
                    let _ = service_handle.ingest_market_data(&dbs_disk.disk, to_send).await;
                });
            }

            // RESET bar to the new window
            bar.time = Datetime::from_timestamp(bar_start_ts / 1000, 0).unwrap_or_default();
            bar.open = price; bar.high = price; bar.low = price; bar.close = price;
            bar.volume = tick.volume as f64;
            bar.levels.clear();
            *buy_count = 0; *sell_count = 0;
        }

        // --- AGGREGATION ---
        bar.high = bar.high.max(price);
        bar.low = bar.low.min(price);
        bar.close = price;
        bar.volume = tick.volume as f64;

        let bid_diff = tick.bid - prev_tick.bid;
        let ask_diff = tick.ask - prev_tick.ask;
        let is_buy = if ask_diff > 0.0 { true } else if bid_diff < 0.0 { false } else { tick.bid >= tick.ask };

        let level = bar.levels.entry(format!("{:.5}", price)).or_insert((0.0, 0.0));
        if is_buy { level.0 += 1.0; *buy_count += 1; } 
        else { level.1 += 1.0; *sell_count += 1; }

        *prev_tick = tick.clone();

        // If we are in the live window now, return true so the caller can finalize the backfill
        Ok(is_live_tick)
    }

   

    async fn handle_sync_complete(
        &self,
        socket: &mut TcpStream,
        dbs: &Arc<db::AppDatabases>,
        _state: &WatchdogState,
    ) -> db::AppResult<()> {
        let mut sym_buf = [0u8; 16];
        socket.read_exact(&mut sym_buf).await?;
        
        let symbol = String::from_utf8_lossy(&sym_buf)
            .split('\0').next().unwrap_or("").trim().to_string();
        
        let context = self.load_ingestion_context().map_err(|e| e.to_string())?;
        let asset_id = self.get_asset_record_id(symbol.as_bytes(), &context.0.data_source_id);

        info!("Service: Sync Complete for {}. Finalizing Ingestion...", symbol);

        // CRITICAL: We do NOT spawn here. We await.
        // This ensures the aggregation from the previous header is 100% done
        // because this code only runs AFTER process_tick_backfill_headerbatch finished.
        // self.flush_backfill_asset(&asset_id, dbs).await;

        Ok(())
    }


    async fn flush_backfill_asset(&self, asset_id: &RecordId, dbs: &Arc<AppDatabases>) {
        let mut batch_to_save = Vec::new();
        let aid_log = asset_id.to_raw_string();

        if let Some(mut entry) = BACKFILL_STATE.get_mut(asset_id) {
            let (ref mut bar, _, _, ref mut batch, _) = *entry;
            
            // If batch is empty, it means all ticks stayed in the 'active bar'
            if batch.is_empty() && bar.volume > 0.0 {
                info!("Handoff: No historical bars found for {}, data is live in RAM (Vol: {})", aid_log, bar.volume);
            }

            if !batch.is_empty() {
                batch_to_save = std::mem::take(batch);
            }
        }

        if !batch_to_save.is_empty() {
            let count = batch_to_save.len();
            if let Err(e) = self.ingest_market_data(&dbs.disk, batch_to_save).await {
                error!("Error flushing {} bars for {}: {}", count, aid_log, e);
            } else {
                info!("Successfully flushed {} historical bars for {}", count, aid_log);
            }
        }
    }

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
        mut update: MonitorUpdates,
        symbol: &str,
        source: &str,
        table: &str,
        broker_offset_seconds: i64
    ) -> surrealdb::Result<()> {
        
        let adjusted_millis = update.time_msc - (broker_offset_seconds * 1000);
        let datetime = Utc.timestamp_millis_opt(adjusted_millis)
            .single()
            .map(Datetime::from)
            .unwrap_or_else(|| Datetime::default());

        update.time = Some(datetime);
        let monitor_id = db::make_composite_id(table, &[&symbol, &source]);

        // --- RETRY LOGIC START ---
        let mut attempts = 0;
        let max_attempts = 15;

        loop {
            let result = dbs.mem
                .query("UPSERT $id SET 
                    asset_id = $asset, 
                    last_bid = $bid, 
                    last_ask = $ask, 
                    volume = $vol, 
                    flags = $flags, 
                    time = $time")
                .bind(("id", monitor_id.clone()))
                .bind(("asset", update.asset_id.clone()))
                .bind(("bid", update.last_bid))
                .bind(("ask", update.last_ask))
                .bind(("vol", update.volume))
                .bind(("flags", update.flags))
                .bind(("time", datetime))
                .await;

            match result {
                Ok(response) => {
                    // If query worked, check if the response itself has an error
                    if let Err(e) = response.check() {
                        let err_string = e.to_string();
                        // If it's a write conflict, we retry
                        if err_string.contains("Write conflict") && attempts < max_attempts {
                            attempts += 1;
                            let delay = 5 + (attempts * 2); 
                            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                            continue;
                        }
                        return Err(e);
                    }
                    return Ok(()); // Success!
                }
                Err(e) => {
                    let err_string = e.to_string();
                    // If the network call failed due to conflict, we retry
                    if err_string.contains("Write conflict") && attempts < max_attempts {
                        attempts += 1;
                        let delay = 5 + (attempts * 2); 
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }


   

    // fn get_asset_record_id(&self, asset_bytes: &[u8], source: &str) -> RecordId {

    //     let name = std::str::from_utf8(asset_bytes).unwrap_or("").trim_matches(char::from(0));
       
    //     let asset_id = db::make_composite_id("assets",&[&name, &source]);
       
    //     asset_id
    // }

    fn get_asset_record_id(&self, raw_asset: &[u8], source_id: &str) -> RecordId {
        let clean_symbol = String::from_utf8_lossy(raw_asset)
            .split('\0')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        
        db::make_composite_id("assets", &[&clean_symbol, &source_id])
    }


    async fn perform_mirror_cycle(
        &self,
        dbs: &Arc<db::AppDatabases>,
        state: &WatchdogState,
        is_shutdown: bool
    ) {
        // 1. Guard: If market is closed and it's not a shutdown, save CPU and exit
        if !is_shutdown && !Mt5Watchdog::new(state.clone()).is_market_open() {
            info!("[Mirror] Market is closed, skipping mirror cycle.");
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
                        // 4. Use the Universal Ingester
                        if let Err(e) = self.ingest_market_data(&dbs.disk, data).await {
                            error!("[Mirror] Failed to flush {} records: {}", count, e);
                        } else {
                            // info!("[Mirror] Successfully flushed {} candles for window {} to Disk", count, target_ts);
                            info!("[Mirror] Synced {} NEW bars for {} {} to Disk.", count, asset_id.key.to_raw_string(), target_ts);
                        }
                    }
                },
                
                Err(e) => error!("[Mirror] Query failed for {}: {}", asset_id.key.to_raw_string(), e),
            }
        }

        if !is_shutdown {
            // 1 Hour Pruning for Ticks, 24 Hour for MarketData
            let candle_cutoff = Datetime::from(Utc::now() - Duration::from_hours(24));

            let _ = dbs.mem.query("DELETE tick WHERE time < time::now() - 5m; DELETE market_data WHERE time < $c_cutoff;")
                // .bind(("t_cutoff", tick_cutoff))
                .bind(("c_cutoff", candle_cutoff))
                .await;
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

        info!("Market Data Ingested: {} unique records processed for asset {}.", unique_count, all_rows.first().map(|r| r.data.asset_id.key.to_raw_string()).unwrap_or_else(|| "UNKNOWN".to_string()), );
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


}