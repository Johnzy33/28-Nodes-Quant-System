
use chrono::{ Utc,  Offset, TimeZone}; 
// use sqlx::{PgPool};
// use sqlx::{FromRow};
use surrealdb_types::{RecordId, Datetime};
// use crate::market_classification::MarketType;
use anyhow::{ Ok, Result};
//  use rdkafka::{
//     producer::{FutureProducer},
//     config::ClientConfig,
// };
// use std::env;
use bytemuck::{Pod, Zeroable};
// use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize};
// use std::sync::OnceLock;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use once_cell::sync::OnceCell;
use log::{info, warn};
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::net::{TcpStream};
use tokio::io::{AsyncReadExt};
use dashmap::DashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use crate::db_models::{self as db,AppDatabases, SurrealIdExt};

use dashmap::DashSet;



use chrono_tz::Europe::Athens;

pub type IngestionContext = (Arc<IngestionCoordinatorConfig>, FxHashMap<SmolStr, SmolStr>);


// 2. Ensure the Static Cell matches the Alias
pub static SYMBOL_MAP: OnceCell<IngestionContext> = OnceCell::new();
pub static LAST_LIVE_TS: Lazy<DashMap<RecordId, i64>> = Lazy::new(DashMap::new);

pub static BROKER_OFFSET: AtomicI64 = AtomicI64::new(0);
pub static  SYNCED_ASSETS: Lazy<DashSet<RecordId>> = Lazy::new(DashSet::new);
pub static  BACKFILL_STATE: Lazy<DashMap<RecordId, (db::MarketData, u64, u64, Vec<db::MarketData>, MultiplexedTick)>> = Lazy::new(DashMap::new);

/// Tracks calibration state to detect frozen clocks
pub struct CalibrationState {
    pub last_valid_offset: i64,        // Last known-good offset (in seconds)
    pub last_broker_time: i64,         // Last received broker timestamp
    pub calibration_count: u32,        // Number of successful calibrations
    pub frozen_clock_detected: bool,   // Is the broker clock currently frozen?
}

impl Default for CalibrationState {
    fn default() -> Self {
        Self {
            last_valid_offset: 0,
            last_broker_time: 0,
            calibration_count: 0,
            frozen_clock_detected: false,
        }
    }
}

lazy_static::lazy_static! {
    pub static ref MARKET_SESSIONS: DashMap<RecordId, Vec<SessionPacket>> = DashMap::new();
    pub static ref CALIBRATION_STATE: Mutex<CalibrationState> = Mutex::new(CalibrationState::default());
}

// static MARKET_SESSIONS: Lazy<DashMap<String, Vec<MarketSession>>> = Lazy::new(DashMap::new);


#[derive(Clone)]
pub struct DataService {
    pub dbs: Arc<AppDatabases>,
    // pub pool: PgPool,
}

// pub struct DataService {
//     // Replace your old sqlx::PgPool with this
//     pub dbs: Arc<AppDatabases>, 
// }


#[derive(Debug, Deserialize)]
pub struct IngestionCoordinatorConfig {
    // pub kafka_brokers: String,
    pub data_source_id: String,
    pub assets: Vec<AssetIngestJob>,
}

impl IngestionCoordinatorConfig {
    /// Loads the configuration from a file (requires `serde_yaml` or similar).
    pub fn load_from_file(path: &str) -> Result<Self> {
        let file_content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&file_content)?;
        Ok(config)
    }
}


#[derive(Debug, Deserialize, Clone)]
pub struct AssetIngestJob {
    pub system_symbol: String, // Formerly 'symbol'
    pub mt5_symbol: String,    // The new mapping field
    //pub file_path: String,
    // pub topic_base: String,
    pub timeframe: String,
    pub enabled: bool,
}

impl DataService{
    // pub fn new(pool: PgPool) -> Self {
    //     DataService { pool }
    // }  

    pub fn new(dbs: Arc<AppDatabases>) -> Self {
        Self { dbs }
    }


    pub fn load_ingestion_context(&self) -> anyhow::Result<&IngestionContext> {

        SYMBOL_MAP.get_or_try_init(|| {
            info!(" First connection detected: Loading config.json into memory...");
            
            let config_path = std::env::var("INGESTION_CONFIG_PATH")
                .unwrap_or_else(|_| "config.json".into());
                
            let config_data = IngestionCoordinatorConfig::load_from_file(&config_path)
                .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
            
            let arc_config = Arc::new(config_data);

            
            // Note: FxHashMap is an alias for HashMap<K, V, FxBuildHasher>
            let mut symbol_map = FxHashMap::default();
            
            for asset in &arc_config.assets {
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

            // --- THE CRITICAL RETURN ---
            // This MUST be an Ok() containing the TUPLE
            Ok((arc_config, symbol_map))
        })
    }


    pub async fn calibration_header(&self, socket: &mut TcpStream) -> anyhow::Result<()> {
        let mut buf = [0u8; 16];
        socket.read_exact(&mut buf).await?;
        
        let broker_time = i64::from_le_bytes(buf[0..8].try_into()?);
        let gmt_time = i64::from_le_bytes(buf[8..16].try_into()?);
        
        let raw_offset = broker_time - gmt_time;

        // 1. Calculate the calculated offset
        let mut normalized_offset = raw_offset.rem_euclid(86400);
        if normalized_offset > 43200 {
            normalized_offset -= 86400;
        }
        let rounded_offset = (normalized_offset as f64 / 3600.0).round() as i64 * 3600;
        let final_hours = rounded_offset / 3600;

        // 2. THE SUNDAY GUARD
        // If the offset is suspicious (> 5h), we pull the REAL offset for Athens right now.
        if final_hours.abs() > 3 {
            warn!(" Weekend Drift Detected (GMT{:+}). Market is closed.", final_hours);
            
            // Get the actual offset for Athens based on the current UTC time
            let now_utc = Utc::now();
            let athens_offset = Athens.offset_from_utc_datetime(&now_utc.naive_utc());
            let total_offset_secs = athens_offset.fix().local_minus_utc() as i64;
            
            info!("Falling back to Europe/Athens current offset: GMT{:+}", total_offset_secs / 3600);
            BROKER_OFFSET.store(total_offset_secs, Ordering::SeqCst);
        } else {
            BROKER_OFFSET.store(rounded_offset, Ordering::SeqCst);
            info!("Calibration Success: Broker is GMT{:+}", final_hours);
        }

        Ok(())
    }

    pub fn prime_ingestion_state(
            &self, 
            context: &(Arc<IngestionCoordinatorConfig>, FxHashMap<SmolStr, SmolStr>)) {
        let config = &context.0;
        
        for asset in &config.assets {
            let record_id = db::make_composite_id("assets", &[&asset.mt5_symbol, &config.data_source_id]);

            // Ensure the global state has a home for this asset
            BACKFILL_STATE.entry(record_id.clone()).or_insert_with(|| {
                (
                    db::MarketData {
                        asset_id: record_id.clone(),
                        time: Datetime::from_timestamp(1, 0).unwrap_or_default(),
                        ..Default::default()
                    },
                    0, 0, Vec::with_capacity(50), MultiplexedTick::default()
                )
            });

            // Ensure the set knows we are tracking this asset
            SYNCED_ASSETS.insert(record_id);
        }
        info!("Primed {} assets in global state", SYNCED_ASSETS.len());
    }

    pub  fn ensure_asset_primed(&self, mt5_symbol: &str, data_source_id: &str) -> RecordId {
        let record_id = db::make_composite_id("assets", &[&mt5_symbol, &data_source_id]);
        
        // Check SYNCED_ASSETS first (it's fast)
        if !SYNCED_ASSETS.contains(&record_id) {
            // Double-check/Insert into BACKFILL_STATE
            // BACKFILL_STATE.entry(record_id.clone()).or_insert_with(|| {
            //     (
            //         db::MarketData {
            //             asset_id: record_id.clone(),
            //             time: Datetime::from_timestamp(1, 0).unwrap_or_default(),
            //             ..Default::default()
            //         },
            //         0, 0, Vec::with_capacity(50), MultiplexedTick::default()
            //     )
            // });
            
            SYNCED_ASSETS.insert(record_id.clone());
            info!("Just-in-time priming successful for: {} Primed assets in global state: {}", record_id.to_raw_string(), SYNCED_ASSETS.len());
        }
        
        record_id
    }


    pub fn get_broker_offset(&self) -> i64 {
        BROKER_OFFSET.load(Ordering::SeqCst)
    }

    


    pub fn check_session_status(&self, asset_id: &RecordId) -> bool {
        // 1. If we have a very recent tick (last 10 seconds), 
        // the market is DEFINITELY open, ignore the schedule.
        if self.has_recent_data(asset_id, 10) {
            return true; 
        }

        if let Some(sessions) = MARKET_SESSIONS.get(asset_id) {
            // 2. Get current Broker Time (Minutes from start of day)
            // Use the clock offset we calculated in SendCalibration
            let broker_now_ms = Utc::now().timestamp_millis() + (self.get_broker_offset() * 1000);
            let current_min_of_day = ((broker_now_ms / 60000) % 1440) as u32;

            // 3. Check if current time falls into ANY of the sessions
            for s in sessions.iter() {
            if s.is_active == 0 { continue; }

            // Convert Packet hours/mins into a single number for comparison
            let start_min = (s.open_hour * 60) + s.open_min;
            let end_min = (s.close_hour * 60) + s.close_min;

            // Check if we are inside this specific session window
            if current_min_of_day >= start_min && current_min_of_day < end_min {
                return true;
            }
        }
            false
        } else {
            true // Unknown? Keep open to be safe.
        }
    }

    pub fn has_recent_data(&self, asset_id: &RecordId, seconds: i64) -> bool {
        if let Some(last_ts) = LAST_LIVE_TS.get(asset_id) {
            let broker_now_ms = Utc::now().timestamp_millis() + (BROKER_OFFSET.load(Ordering::SeqCst) * 1000);
            
            // If the last tick arrived less than X seconds ago, return true
            (broker_now_ms - *last_ts) < (seconds * 1000)
        } else {
            false
        }
    }

}

#[repr(C, packed)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct SyncBar {
    pub asset: [u8; 10], 
    pub _pad: [u8; 6],
    pub open: f64, 
    pub high: f64, 
    pub low: f64, 
    pub close: f64,
    pub volume: i64, 
    pub time: i64,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct MultiplexedTick {
    pub asset: [u8; 10], 
    pub _dummy: [u8; 6],
    pub bid: f64, 
    pub ask: f64, 
    pub last: f64,
    pub volume: i64, 
    pub time_msc: i64,
    pub flags: u32, 
    pub _end_pad: u32,
}

impl Default for MultiplexedTick {
    fn default() -> Self {
        Self {
            asset: [0u8; 10],
            _dummy: [0u8; 6],
            bid: 0.0,
            ask: 0.0,
            last: 0.0,
            volume: 0,
            time_msc: 0,
            flags: 0,
            _end_pad: 0,
        }
    }
}


pub struct MarketSession {
    pub open_min_of_day: u32,
    pub close_min_of_day: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SessionPacket {
    pub asset: [u8; 16],
    pub open_hour: u32,
    pub open_min: u32,
    pub close_hour: u32,
    pub close_min: u32,
    pub session_index: i32,
    pub is_active: i32,
}
#[repr(C, packed)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct CalibrationPacket {
    pub broker_time: i64,
    pub gmt_time: i64,
}
