
// use serde::{Serialize, Deserialize};
// use std::fs;
// use std::path::Path;
// use std::sync::Arc;
// use std::time::Duration;
// use chrono::{DateTime, Utc, Datelike, Timelike};
// use rdkafka::producer::FutureProducer;
// use anyhow::Result;
// use log::{info, error, debug};

// use crate::producer_config::{IngestionCoordinatorConfig, SyncCommand};
// use rdkafka::producer::FutureRecord;

// #[derive(Serialize, Deserialize, Debug, Default)]
// struct MaintenanceState {
//     last_weekend_heal: Option<DateTime<Utc>>,
//     last_daily_heal: Option<DateTime<Utc>>,
// }

// pub struct MaintenanceService {
//     producer: Arc<FutureProducer>,
//     config: Arc<IngestionCoordinatorConfig>,
//     state_path: String,
// }

// impl MaintenanceService {
//     pub fn new(producer: Arc<FutureProducer>, config: Arc<IngestionCoordinatorConfig>) -> Self {
//         Self {
//             producer,
//             config,
//             state_path: "maintenance_state.json".to_string(),
//         }
//     }

//     pub async fn run(&self) {
//         info!("🛠️ Maintenance Service started (File-based state).");
//         let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Check every hour

//         loop {
//             interval.tick().await;
//             let now = Utc::now();
//             let mut state = self.load_state();
//             let mut changed = false;

//             // --- 1. DAILY HEAL CHECK (Target: After 22:00 UTC) ---
//             if now.hour() >= 22 {
//                 let needs_daily = state.last_daily_heal
//                     .map(|dt| dt.date_naive() < now.date_naive())
//                     .unwrap_or(true);

//                 if needs_daily {
//                     info!("🩹 Daily Heal Triggered: Missing or outdated for today.");
//                     if self.trigger_heal(24).await.is_ok() {
//                         state.last_daily_heal = Some(now);
//                         changed = true;
//                     }
//                 }
//             }

//             // --- 2. WEEKEND DEEP HEAL CHECK ---
//             if now.weekday() == chrono::Weekday::Sat || now.weekday() == chrono::Weekday::Sun {
//                 let needs_weekend = state.last_weekend_heal
//                     .map(|dt| (now - dt).num_days() >= 6)
//                     .unwrap_or(true);

//                 if needs_weekend {
//                     info!("🩹 Weekend Deep-Heal Triggered: Gap filling for the week.");
//                     if self.trigger_heal(168).await.is_ok() {
//                         state.last_weekend_heal = Some(now);
//                         changed = true;
//                     }
//                 }
//             }

//             if changed {
//                 self.save_state(&state);
//             }
//         }
//     }

//     async fn trigger_heal(&self, lookback_hours: i64) -> Result<()> {
//         let now = Utc::now();
//         let start_ms = (now - chrono::Duration::hours(lookback_hours)).timestamp_millis();
//         let end_ms = now.timestamp_millis();

//         for asset in &self.config.assets {
//             if !asset.enabled { continue; }

//             let cmd = SyncCommand {
//                 command: "HEAL".to_string(),
//                 system_symbol: asset.system_symbol.clone(),
//                 mt5_symbol: asset.mt5_symbol.clone(),
//                 start_timestamp_ms: start_ms,
//                 end_timestamp_ms: Some(end_ms),
//                 timeframe: asset.timeframe.clone(),
//             };

//             let payload = serde_json::to_vec(&cmd)?;
//             self.producer.send(
//                 FutureRecord::to("market_control")
//                     .payload(&payload)
//                     .key(&asset.system_symbol),
//                 Duration::from_secs(5)
//             ).await.map_err(|(e, _)| e)?;
//         }
//         info!("✅ Heal command successfully dispatched for all assets.");
//         Ok(())
//     }

//     fn load_state(&self) -> MaintenanceState {
//         if !Path::new(&self.state_path).exists() { return MaintenanceState::default(); }
//         fs::read_to_string(&self.state_path)
//             .ok()
//             .and_then(|c| serde_json::from_str(&c).ok())
//             .unwrap_or_default()
//     }

//     fn save_state(&self, state: &MaintenanceState) {
//         if let Ok(content) = serde_json::to_string_pretty(state) {
//             let tmp = format!("{}.tmp", self.state_path);
//             if fs::write(&tmp, content).is_ok() {
//                 let _ = fs::rename(tmp, &self.state_path);
//             }
//         }
//     }
// }


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

use crate::producer_config::{IngestionCoordinatorConfig, SyncCommand};
use shared_models::data_model::DataService;
use crate::traits::{DataServiceBase, DataMaintenanceExt};

lazy_static! {
    static ref SYNCED_ASSETS: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
}

// #[derive(Serialize, Deserialize, Debug, Default)]
// struct MaintenanceState {
//     last_weekend_heal: Option<DateTime<Utc>>,
//     last_daily_heal: Option<DateTime<Utc>>,
// }

// pub struct MaintenanceService {
//     producer: Arc<FutureProducer>,
//     config: Arc<IngestionCoordinatorConfig>,
//     data_service: DataService,
//     state_file_path: String,
// }

// impl MaintenanceService {
//     pub fn new(producer: Arc<FutureProducer>, config: Arc<IngestionCoordinatorConfig>, data_service: DataService) -> Self {
//         // Pull directory from .env or default to current dir
//         let base_dir = std::env::var("MARKET_DATA_DIR").unwrap_or_else(|_| ".".to_string());
//         let state_file_path = format!("{}/maintenance_state.json", base_dir);

//         Self {
//             producer,
//             config,
//             data_service,
//             state_file_path,
//         }
//     }

//     /// The main loop for Daily and Weekend healing
//     pub async fn run(&self) {
//         info!("🛠️ Maintenance Service active. State file: {}", self.state_file_path);
//         let mut interval = tokio::time::interval(Duration::from_secs(3600)); 

//         loop {
//             interval.tick().await;
//             let now = Utc::now();
//             let mut state = self.load_state();
//             let mut changed = false;

//             // --- 1. DAILY HEAL (Post-Market Close) ---
//             if now.hour() >= 22 {
//                 let needs_daily = state.last_daily_heal
//                     .map(|dt| dt.date_naive() < now.date_naive())
//                     .unwrap_or(true);

//                 if needs_daily {
//                     if self.trigger_heal("HEAL", 24, Some(now.timestamp_millis())).await.is_ok() {
//                         state.last_daily_heal = Some(now);
//                         changed = true;
//                     }
//                 }
//             }

//             // --- 2. WEEKEND DEEP HEAL ---
//             if now.weekday() == chrono::Weekday::Sat || now.weekday() == chrono::Weekday::Sun {
//                 let needs_weekend = state.last_weekend_heal
//                     .map(|dt| (now - dt).num_days() >= 6)
//                     .unwrap_or(true);

//                 if needs_weekend {
//                     if self.trigger_heal("HEAL", 168, Some(now.timestamp_millis())).await.is_ok() {
//                         state.last_weekend_heal = Some(now);
//                         changed = true;
//                     }
//                 }
//             }

//             if changed {
//                 self.save_state(&state);
//             }
//         }
//     }

//     /// Migrated: Handles the initial startup SYNC based on High Water Mark
//     pub async fn publish_sync_requests(&self) -> Result<()> {
//         let mut tracker = SYNCED_ASSETS.lock().await;

//         for asset in &self.config.assets {
//             if !asset.enabled { continue; }
//             let asset_id = format!("assets:{}:{}", asset.system_symbol, self.config.data_source_id);

//             if tracker.contains(&asset_id) { continue; }

//             let hwm = self.data_service.get_market_data_high_watermark(&asset_id).await?
//                 .unwrap_or(0); 

//             self.send_kafka_cmd("SYNC", &asset.system_symbol, &asset.mt5_symbol, hwm, None, &asset.timeframe).await?;

//             info!("📡 Startup SYNC sent for {} from HWM: {}", asset.system_symbol, hwm);
//             tracker.insert(asset_id);
//         }
//         Ok(())
//     }

//     /// Internal helper to dispatch Kafka commands
//     async fn trigger_heal(&self, cmd_type: &str, lookback_hours: i64, end_ms: Option<i64>) -> Result<()> {
//         let start_ms = (Utc::now() - chrono::Duration::hours(lookback_hours)).timestamp_millis();
        
//         for asset in &self.config.assets {
//             if !asset.enabled { continue; }
//             self.send_kafka_cmd(cmd_type, &asset.system_symbol, &asset.mt5_symbol, start_ms, end_ms, &asset.timeframe).await?;
//         }
//         info!("✅ {} command dispatched for all assets.", cmd_type);
//         Ok(())
//     }

//     async fn send_kafka_cmd(&self, cmd: &str, sys_sym: &str, mt5_sym: &str, start: i64, end: Option<i64>, tf: &str) -> Result<()> {
//         let command = SyncCommand {
//             command: cmd.to_string(),
//             system_symbol: sys_sym.to_string(),
//             mt5_symbol: mt5_sym.to_string(),
//             start_timestamp_ms: start,
//             end_timestamp_ms: end,
//             timeframe: tf.to_string(),
//         };

//         let payload = serde_json::to_vec(&command)?;
//         self.producer.send(
//             FutureRecord::to("market_control")
//                 .payload(&payload)
//                 .key(sys_sym),
//             Duration::from_secs(5)
//         ).await.map_err(|(e, _)| e).context("Kafka send failed")?;
//         Ok(())
//     }

//     fn load_state(&self) -> MaintenanceState {
//         if !Path::new(&self.state_file_path).exists() { return MaintenanceState::default(); }
//         fs::read_to_string(&self.state_file_path)
//             .ok()
//             .and_then(|c| serde_json::from_str(&c).ok())
//             .unwrap_or_default()
//     }

//     fn save_state(&self, state: &MaintenanceState) {
//         if let Ok(content) = serde_json::to_string_pretty(state) {
//             let tmp = format!("{}.tmp", self.state_file_path);
//             if fs::write(&tmp, content).is_ok() {
//                 let _ = fs::rename(tmp, &self.state_file_path);
//             }
//         }
//     }
// }


// use rdkafka::producer::{FutureProducer, FutureRecord};
// use std::fs;
// use std::path::Path;
// use chrono::{Utc, Datelike, Timelike};
// use anyhow::{Context, Result};
// use log::{info, error};

#[async_trait]
impl DataMaintenanceExt for DataService {

    // async fn publish_startup_sync(
    //     &self,
    //     producer: Arc<FutureProducer>,
    //     config: Arc<IngestionCoordinatorConfig>,
    // ) -> Result<()> {
    //     for asset in &config.assets {
    //         if !asset.enabled { continue; }
    //         let asset_id = format!("assets:{}:{}", asset.system_symbol, config.data_source_id);
            
    //         // Accessing DB directly through DataService's existing methods
    //         let hwm = self.get_market_data_high_watermark(&asset_id).await?.unwrap_or(0);

    //         let cmd = SyncCommand {
    //             command: "SYNC".to_string(),
    //             system_symbol: asset.system_symbol.clone(),
    //             mt5_symbol: asset.mt5_symbol.clone(),
    //             start_timestamp_ms: hwm,
    //             end_timestamp_ms: None,
    //             timeframe: asset.timeframe.clone(),
    //         };

    //         let payload = serde_json::to_vec(&cmd)?;
    //         producer.send(
    //             FutureRecord::to("market_control").payload(&payload).key(&asset.system_symbol),
    //             Duration::from_secs(5)
    //         ).await.map_err(|(e, _)| e)?;
            
    //         info!("📡 Startup SYNC dispatched for {} from HWM: {}", asset.system_symbol, hwm);
    //     }
    //     Ok(())
    // }

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