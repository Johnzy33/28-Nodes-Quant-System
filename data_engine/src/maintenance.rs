


use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc, Datelike, Timelike};
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use anyhow::{Result, Context};
use log::{info, error, debug};
use tokio::sync::Mutex;
use std::collections::HashSet;
use async_trait::async_trait; 
use lazy_static::lazy_static;
use tokio::net::{TcpListener, TcpStream};
use std::error::Error;
use tokio::io::AsyncWriteExt;

use crate::producer_config::{ SyncCommand};
use shared_models::data_model::IngestionCoordinatorConfig;
use shared_models::data_model::DataService;
use crate::traits::{DataServiceBase, DataMaintenanceExt};
use std::sync::atomic::{AtomicI64, Ordering};
use shared_models::data_model::BROKER_OFFSET;

// static BROKER_OFFSET: AtomicI64 = AtomicI64::new(0);

lazy_static! {
    static ref SYNCED_ASSETS: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
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
            
            // 1. Get High Water Mark
            let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), config.data_source_id);
            let hwm = self.get_market_data_high_watermark(&asset_id).await?.unwrap_or(0);

            // 2. Build the exact command Python expects
            let cmd = serde_json::json!({
                "command": "SYNC",
                "system_symbol": asset.system_symbol.to_uppercase(),
                "mt5_symbol": asset.mt5_symbol,
                "start_timestamp_ms": hwm,
                "timeframe": asset.timeframe
            });
            

            let payload = serde_json::to_vec(&cmd)?;
            
            // 3. Send to market_control
            producer.send(
                FutureRecord::to("market_control")
                    .payload(&payload)
                    .key(&asset.system_symbol),
                Duration::from_secs(0) // Use 0 for async-queue
            ).await.map_err(|(e, _)| e)?;
            
            info!("📡 Startup SYNC dispatched for {} from HWM: {}", asset.system_symbol, hwm);
        }

        // --- CRITICAL FIX: FORCE KAFKA TO SEND ---
        // Without this, messages might sit in the buffer while Task B starts the consumer
        let _ = producer.flush(Duration::from_secs(5)); 
        info!("📤 Kafka Producer buffer flushed. Commands are on the wire.");
        
        Ok(())
    }

    async fn send_startup_handshake(
        &self,
        socket: &mut TcpStream,
        //data_service: &DataService,
        config:  &IngestionCoordinatorConfig,
    ) -> Result<(), Box<dyn Error>> {

        let offset = BROKER_OFFSET.load(Ordering::SeqCst);
        let mut sub_list = Vec::new();

        for asset in config.assets.iter().filter(|a| a.enabled) {
            // 1. Calculate Asset ID (Namespace:Symbol:DataSource)
            let asset_id = format!(
                "assets:{}:{}", 
                asset.system_symbol.to_uppercase(), 
                config.data_source_id
            );

            // 2. Fetch High Watermark from SurrealDB
            let hwm = self
                .get_market_data_high_watermark(&asset_id)
                .await?
                .unwrap_or(0); // Default to 0 if no data exists

            // Convert UTC HWM to Broker Time HWM for MT5
           let final_hwm = if hwm > 0 {
            let broker_hwm = hwm  + offset;
            broker_hwm - 1800 // Subtract 30 mins (2 bars of M15) to ensure no gaps
        } else {
            (Utc::now().timestamp() + offset) - 86400 // Default 24h ago
        };

            // 3. Construct the JSON command MT5 expects
            sub_list.push(serde_json::json!({
                "mt5": asset.mt5_symbol,
                "tf": asset.timeframe,
                "hwm": final_hwm, //Utc::now().timestamp() - (1 * 60 * 60), //, hwm_broker,
                "system_symbol": asset.system_symbol.to_uppercase()
            }));

            info!("📡 Prepared SYNC for {} from HWM: {}", asset.system_symbol, hwm);
            // Add this print to your loop to see the "Gap"
            info!("DEBUG: Asset: {}, DB_UTC_HWM: {}, Calc_Broker_HWM: {}, Current_Broker_Time: {}", 
                asset.system_symbol, 
                hwm, 
                final_hwm, 
                Utc::now().timestamp() + offset
            );
        }

        // 4. Serialize and Prepare Binary Packet
        let handshake_json = serde_json::to_string(&sub_list)?;
        let json_bytes = handshake_json.as_bytes();
        let json_len = json_bytes.len() as u32;

        // 5. Send Packet: [Type 255 (Handshake)][Length 4 (u32 LE)][Payload (JSON)]
        let mut header_buf = [0u8; 5];
        header_buf[0] = 255;
        header_buf[1..5].copy_from_slice(&json_len.to_le_bytes());

        socket.write_all(&header_buf).await?;
        socket.write_all(json_bytes).await?;
        socket.flush().await?;

        info!("📤 Handshake dispatched. {} bytes sent to MT5.", json_len);
        Ok(())
    }

    async fn dispatch_heal_command(&self, producer: &FutureProducer, config: &IngestionCoordinatorConfig, hours: i64) -> Result<()> {
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

            let payload = serde_json::to_vec(&cmd)?;
            producer.send(
                FutureRecord::to("market_control").payload(&payload).key(&asset.system_symbol),
                Duration::from_secs(5)
            ).await.map_err(|(e, _)| e)?;
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
        let tmp_path = format!("{}.tmp", state_path);
        
        let ds = self.clone();

        tokio::spawn(async move {
            info!("🛠️ Maintenance Loop active. Using state: {}", state_path);
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); 

            loop {
                interval.tick().await;
                let now = Utc::now();
                
                // 1. Load current state
                let mut state: serde_json::Value = fs::read_to_string(&state_path)
                    .ok()
                    .and_then(|c| serde_json::from_str(&c).ok())
                    .unwrap_or(serde_json::json!({}));

                let mut changed = false;

                // 2. Daily Heal (Post 22:00 UTC)
                if now.hour() >= 22 {
                    let last = state["last_daily"].as_str().and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok());
                    if last.map(|dt| dt.date_naive() < now.date_naive()).unwrap_or(true) {
                        if ds.dispatch_heal_command(&producer, &config, 24).await.is_ok() {
                            state["last_daily"] = serde_json::json!(now.to_rfc3339());
                            changed = true;
                        }
                    }
                }

                // 3. Weekend Deep Heal (Sat/Sun)
                if now.weekday() == chrono::Weekday::Sat || now.weekday() == chrono::Weekday::Sun {
                    let last = state["last_weekend"].as_str().and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok());
                    if last.map(|dt| (now - dt).num_days() >= 6).unwrap_or(true) {
                        if ds.dispatch_heal_command(&producer, &config, 168).await.is_ok() {
                            state["last_weekend"] = serde_json::json!(now.to_rfc3339());
                            changed = true;
                        }
                    }
                }

                // 4. ATOMIC SAVE
                if changed {
                    if let Ok(content) = serde_json::to_string_pretty(&state) {
                        // Write to .tmp first
                        if fs::write(&tmp_path, content).is_ok() {
                            // Atomic rename (Standard on Unix/Linux)
                            if let Err(e) = fs::rename(&tmp_path, &state_path) {
                                error!("Failed to rename maintenance state file: {}", e);
                            } else {
                                info!("✅ Maintenance state updated atomically.");
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}