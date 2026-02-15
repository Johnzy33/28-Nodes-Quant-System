
use chrono::{DateTime, Utc, NaiveDate, Offset, TimeZone}; 
use sqlx::{PgPool, Result as SqlxResult};
use sqlx::{FromRow};
use crate::market_classification::MarketType;
use anyhow::{Context, Ok, Result};
 use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    config::ClientConfig,
};
use std::env;
use bytemuck::{Pod, Zeroable};
// use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use once_cell::sync::OnceCell;
use log::{info, warn, debug};
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use dashmap::DashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use crate::db_models::AppDatabases;

use chrono_tz::Europe::Athens;

pub type IngestionContext = (Arc<IngestionCoordinatorConfig>, FxHashMap<SmolStr, SmolStr>);


// 2. Ensure the Static Cell matches the Alias
pub static SYMBOL_MAP: OnceCell<IngestionContext> = OnceCell::new();
pub static LAST_LIVE_TS: Lazy<DashMap<SmolStr, i64>> = Lazy::new(DashMap::new);

pub static BROKER_OFFSET: AtomicI64 = AtomicI64::new(0);

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
    pub static ref MARKET_SESSIONS: DashMap<SmolStr, Vec<SessionPacket>> = DashMap::new();
    pub static ref CALIBRATION_STATE: Mutex<CalibrationState> = Mutex::new(CalibrationState::default());
}

// static MARKET_SESSIONS: Lazy<DashMap<String, Vec<MarketSession>>> = Lazy::new(DashMap::new);


#[derive(Clone)]
pub struct DataService {
pub pool: PgPool,

}

// pub struct DataService {
//     // Replace your old sqlx::PgPool with this
//     pub dbs: Arc<AppDatabases>, 
// }

impl DataService {
    
}

#[derive(Debug, Deserialize)]
pub struct IngestionCoordinatorConfig {
    pub kafka_brokers: String,
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
    pub topic_base: String,
    pub timeframe: String,
    pub enabled: bool,
}

impl DataService{
    pub fn new(pool: PgPool) -> Self {
        DataService { pool }
    }  

    // pub fn new(dbs: Arc<AppDatabases>) -> Self {
    //     Self { dbs }
    // }

    pub async fn initialize_producer() -> Result<FutureProducer> {
    let brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .create()
        .context("Producer creation error")?;

    Ok(producer)
    
    }

    pub fn load_ingestion_context(&self) -> anyhow::Result<&IngestionContext> {

        SYMBOL_MAP.get_or_try_init(|| {
            info!("📂 First connection detected: Loading config.json into memory...");
            
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

    // pub async fn handle_calibration(&self, socket: &mut TcpStream) -> anyhow::Result<()> {
    //     let mut buf = [0u8; 16];
    //     socket.read_exact(&mut buf).await?;
        
    //     // let broker_time = i64::from_le_bytes(buf);
    //     // let utc_now = chrono::Utc::now().timestamp();

    //     let broker_time = i64::from_le_bytes(buf[0..8].try_into()?);
    //     let gmt_time = i64::from_le_bytes(buf[8..16].try_into()?);
        
    //     // Calculate offset rounded to the nearest hour
    //     let mut raw_offset = broker_time - gmt_time;

    //     if raw_offset.abs() > 43200 {
    //     debug!("🕵️ Weekend drift detected ({}s). Normalizing...", raw_offset);
    //     // This keeps the "leftover" hours after removing whole days
    //     let remainder = raw_offset % 86400; 
        
    //     // Handle negative modulo math in Rust to ensure we get the true TZ
    //     raw_offset = if remainder < -43200 { remainder + 86400 } 
    //                  else if remainder > 43200 { remainder - 86400 } 
    //                  else { remainder };
    //     }

    //     let rounded_offset = (raw_offset as f64 / 3600.0).round() as i64 * 3600;
        
    //     BROKER_OFFSET.store(rounded_offset, Ordering::SeqCst);
        
    //     info!("🔄 Calibration Success! Broker Offset: {} hours ({}s)", rounded_offset / 3600, rounded_offset);
    //     info!("DEBUG: MT5_Broker={} | MT5_GMT={} | Calculated_Offset={}", broker_time, gmt_time, raw_offset);
    //     info!("Broker Time: {}, GMT Time: {}, Raw Offset: {}s", broker_time, gmt_time, raw_offset);
    //     Ok(())
    // }

    // pub async fn handle_calibration(&self, socket: &mut TcpStream) -> anyhow::Result<()> {
    //     let mut buf = [0u8; 16];
    //     socket.read_exact(&mut buf).await?;
        
    //     let broker_time = i64::from_le_bytes(buf[0..8].try_into()?);
    //     let gmt_time = i64::from_le_bytes(buf[8..16].try_into()?);
        
    //     let raw_offset = broker_time - gmt_time;
        
    //     let mut cal_state = CALIBRATION_STATE.lock().unwrap();

    //     // === FROZEN CLOCK DETECTION: Raw offset too large ===
    //     // If raw_offset is > ±12 hours (43200s), broker clock is likely frozen (36+ hour drift)
    //     // This catches the freeze IMMEDIATELY, even on first calibration
    //     if raw_offset.abs() > 43200 {
    //         warn!("⚠️  FROZEN CLOCK DETECTED: Raw offset {}s ({}h) is too large!", 
    //               raw_offset, raw_offset / 3600);
            
    //         if cal_state.calibration_count > 0 {
    //             // We have a previous valid offset, use it as fallback
    //             warn!("🔒 Using fallback offset from last valid calibration: GMT{:+}", 
    //                   cal_state.last_valid_offset / 3600);
    //             BROKER_OFFSET.store(cal_state.last_valid_offset, Ordering::SeqCst);
    //         } else {
    //             // First calibration and it's bad - this shouldn't happen if MQL5 sends TimeGMT()
    //             // Try to normalize it anyway in case it's a transient value
    //             warn!("📡 First calibration with large offset - normalizing and storing");
    //             let mut normalized_offset = raw_offset.rem_euclid(86400);
    //             if normalized_offset > 43200 {
    //                 normalized_offset -= 86400;
    //             }
    //             let rounded = (normalized_offset as f64 / 3600.0).round() as i64 * 3600;
    //             warn!("   Normalized to: GMT{:+}", rounded / 3600);
    //             BROKER_OFFSET.store(rounded, Ordering::SeqCst);
    //             cal_state.last_valid_offset = rounded;
    //         }
            
    //         cal_state.frozen_clock_detected = true;
    //         drop(cal_state);
    //         return Ok(());
    //     }

    //     // === FROZEN CLOCK DETECTION: Broker time not advancing ===
    //     // If broker_time hasn't changed since last calibration, clock is frozen
    //     let is_frozen = cal_state.last_broker_time > 0 && broker_time == cal_state.last_broker_time;
        
    //     if is_frozen {
    //         warn!("⚠️  FROZEN CLOCK: Broker time stuck at {}. Using last valid offset: GMT{:+}", 
    //               broker_time, cal_state.last_valid_offset / 3600);
    //         cal_state.frozen_clock_detected = true;
    //         BROKER_OFFSET.store(cal_state.last_valid_offset, Ordering::SeqCst);
    //         drop(cal_state);
    //         return Ok(());
    //     }

    //     // === EXCESSIVE DRIFT DETECTION ===
    //     // If offset changed by > 6 hours since last calibration, something went wrong
    //     let offset_drift = (raw_offset - cal_state.last_valid_offset).abs();
    //     if offset_drift > 21600 && cal_state.calibration_count > 0 {
    //         warn!("⚠️  EXCESSIVE DRIFT: Offset jumped {}s (from {} to {})", 
    //               offset_drift, cal_state.last_valid_offset, raw_offset);
    //         warn!("🔒 Using fallback offset: GMT{:+}", cal_state.last_valid_offset / 3600);
    //         cal_state.frozen_clock_detected = true;
    //         BROKER_OFFSET.store(cal_state.last_valid_offset, Ordering::SeqCst);
    //         drop(cal_state);
    //         return Ok(());
    //     }

    //     // === NORMAL CALIBRATION FLOW ===
    //     // 1. Normalize the offset to a 24-hour window
    //     let mut normalized_offset = raw_offset.rem_euclid(86400);

    //     // 2. Adjust for brokers behind GMT (e.g., New York is -5)
    //     if normalized_offset > 43200 {
    //         normalized_offset -= 86400;
    //     }

    //     // 3. Round to the nearest hour for stability
    //     let rounded_offset = (normalized_offset as f64 / 3600.0).round() as i64 * 3600;
        
    //     // Update calibration state with this valid reading
    //     cal_state.last_broker_time = broker_time;
    //     cal_state.last_valid_offset = rounded_offset;
    //     cal_state.calibration_count += 1;
    //     cal_state.frozen_clock_detected = false;
        
    //     BROKER_OFFSET.store(rounded_offset, Ordering::SeqCst);
        
    //     info!("✅ Calibration #{}: Broker is GMT{:+}", cal_state.calibration_count, rounded_offset / 3600);
    //     info!("   Raw offset: {}s | Normalized: {}s", raw_offset, normalized_offset);
        
    //     drop(cal_state);
    //     Ok(())
    // }

//     pub async fn handle_calibration(&self, socket: &mut TcpStream) -> anyhow::Result<()> {
//     let mut buf = [0u8; 16];
//     socket.read_exact(&mut buf).await?;
    
//     let broker_time = i64::from_le_bytes(buf[0..8].try_into()?);
//     let gmt_time = i64::from_le_bytes(buf[8..16].try_into()?);
    
//     let raw_offset = broker_time - gmt_time;

//     // 1. Calculate the calculated offset
//     let mut normalized_offset = raw_offset.rem_euclid(86400);
//     if normalized_offset > 43200 {
//         normalized_offset -= 86400;
//     }
//     let rounded_offset = (normalized_offset as f64 / 3600.0).round() as i64 * 3600;
//     let final_hours = rounded_offset / 3600;

//     // 2. THE SUNDAY GUARD
//     // If the offset is suspicious (> 5h), we pull the REAL offset for Athens right now.
//     if final_hours.abs() > 5 {
//         warn!("⚠️ Weekend Drift Detected (GMT{:+}). Market is closed.", final_hours);
        
//         // Get the actual offset for Athens based on the current UTC time
//         let now_utc = Utc::now();
//         let athens_offset = Athens.offset_from_utc_datetime(&now_utc.naive_utc());
//         let total_offset_secs = athens_offset.fix().local_minus_utc() as i64;
        
//         info!("🏛️ Falling back to Europe/Athens current offset: GMT{:+}", total_offset_secs / 3600);
//         BROKER_OFFSET.store(total_offset_secs, Ordering::SeqCst);
//     } else {
//         BROKER_OFFSET.store(rounded_offset, Ordering::SeqCst);
//         info!("✅ Calibration Success: Broker is GMT{:+}", final_hours);
//     }

//     Ok(())
// }

    pub async fn handle_calibration(&self, socket: &mut TcpStream) -> anyhow::Result<()> {
    let mut buf = [0u8; 16];
    socket.read_exact(&mut buf).await?;
    
    let broker_time = i64::from_le_bytes(buf[0..8].try_into()?);
    let gmt_time = i64::from_le_bytes(buf[8..16].try_into()?);
    
    let raw_offset = broker_time - gmt_time;
    let mut cal_state = CALIBRATION_STATE.lock().unwrap();

    // 1. DETECT CLOCK FREEZE (Friday Close / Maintenance)
    // If the broker time is identical to last hour, or raw_offset is impossible
    let is_clock_stuck = cal_state.last_broker_time > 0 && broker_time <= cal_state.last_broker_time;
    let is_offset_insane = raw_offset.abs() > 43200; // > 12 hours is usually a freeze result

    if is_clock_stuck || is_offset_insane {
        if cal_state.calibration_count > 0 {
            warn!("⚠️ Broker Clock Frozen ({}). Locking Offset to GMT{:+}", 
                  broker_time, cal_state.last_valid_offset / 3600);

            let now_utc = Utc::now();
            let athens_offset = Athens.offset_from_utc_datetime(&now_utc.naive_utc());
            let total_offset_secs = athens_offset.fix().local_minus_utc() as i64;
        
            info!("🏛️ Falling back to Europe/Athens current offset: GMT{:+}", total_offset_secs / 3600);
            BROKER_OFFSET.store(total_offset_secs, Ordering::SeqCst);
            
            // Keep using what worked when the market was actually moving
            // BROKER_OFFSET.store(cal_state.last_valid_offset, Ordering::SeqCst);
            cal_state.frozen_clock_detected = true;
            return Ok(());
        }
    }


    // 2. NORMALIZE & ROUND
    // We only reach here if the clock seems to be moving or it's our first run
    let mut normalized = raw_offset.rem_euclid(86400);
    if normalized > 43200 { normalized -= 86400; }
    
    // Round to nearest hour (Brokers don't use 30-min offsets usually)
    let rounded_offset = (normalized as f64 / 3600.0).round() as i64 * 3600;
    let final_hours = rounded_offset / 3600;

    // 3. JUMP GUARD
    // If we have an offset and the new one changed by > 30 mins, 
    // it's likely a mid-maintenance calculation error.
    if cal_state.calibration_count > 0 {
        let drift = (rounded_offset - cal_state.last_valid_offset).abs();
        if drift > 3600 { // 1 hour
             warn!("🛑 Blocked suspicious offset jump: {} -> {}", 
                   cal_state.last_valid_offset / 3600, rounded_offset / 3600);
             return Ok(());
        }
    }

   

    // 4. COMMIT SUCCESS
    cal_state.last_broker_time = broker_time;
    cal_state.last_valid_offset = rounded_offset;
    cal_state.calibration_count += 1;
    cal_state.frozen_clock_detected = false;
    
    BROKER_OFFSET.store(rounded_offset, Ordering::SeqCst);
    info!("✅ Calibration #{}: Offset Locked at GMT{:+}", 
          cal_state.calibration_count, rounded_offset / 3600);

    Ok(())
}

    pub fn get_broker_offset(&self) -> i64 {
        BROKER_OFFSET.load(Ordering::SeqCst)
    }

    

    // pub fn check_session_status(&self, asset_id: &str) -> bool {
    //     // 1. Look up the session packet for this asset
    //     if let Some(session) = MARKET_SESSIONS.get(asset_id) {
    //         if session.is_active == 0 {
    //             return false; // Market is explicitly closed (Weekend/Holiday)
    //         }

    //         // 2. Calculate current time in minutes from the start of the day
    //         let now = std::time::SystemTime::now()
    //             .duration_since(std::time::UNIX_EPOCH)
    //             .unwrap()
    //             .as_secs();
            
    //         let current_min_of_day = ((now / 60) % 1440) as u32;
    //         let start_min = (session.open_hour * 60) + session.open_min;
    //         let end_min = (session.close_hour * 60) + session.close_min;

    //         // 3. Return true only if we are within the open window
    //         current_min_of_day >= start_min && current_min_of_day < end_min
    //     } else {
    //         // If we haven't received a session packet yet, default to true 
    //         // to avoid missing data during the first connection
    //         true 
    //     }
    // }

    pub fn check_session_status(&self, asset_id: &str) -> bool {
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

    pub fn has_recent_data(&self, asset_id: &str, seconds: i64) -> bool {
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

// #[repr(C)]
// #[derive(Copy, Clone, Debug, Pod, Zeroable)]
// pub struct SessionPacket {
//     pub asset: [u8; 16],
//     pub open_hour: u32,
//     pub open_min: u32,
//     pub close_hour: u32,
//     pub close_min: u32,
//     pub is_active: u32, // 1 = trading day, 0 = closed/holiday
// }

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
// ====================================================================
// 1. Raw Session Data (Input - mirrors the session_base table)
// ====================================================================

/// Struct to represent the raw data fetched from the session_base table
#[derive(Debug, FromRow, Clone)]
pub struct RawSessionData {
    pub trading_date: chrono::NaiveDate,
    pub asset_id: String,
    pub session_name: String,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub high_ts: DateTime<Utc>,
    pub low: f64,
    pub low_ts: DateTime<Utc>,
    pub close: f64,
    pub volume: f64, 
    pub bars: i64,
}


// ====================================================================
// 2. Classified Session (Output - mirrors the session_views table)
// ====================================================================

/// The Rust struct representation of the final classified session_views table (L1).
#[derive(Debug, Clone)]
pub struct ClassifiedSession {
    pub trading_date: chrono::NaiveDate,
    pub asset_id: String,
    pub session_name: String,
    pub session_type: MarketType,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub high_ts: DateTime<Utc>,
    pub low: f64,
    pub low_ts: DateTime<Utc>,
    pub close: f64,
    pub volume: f64,
    pub bars: i64,
    
}

impl Default for ClassifiedSession {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            trading_date: now.date_naive(),
            asset_id: String::default(),
            session_name: "Unknown".to_string(),
            session_type: MarketType::Other, // Or your default variant
            start_ts: now,
            end_ts: now,
            open: 0.0,
            high: f64::MIN, // Set to MIN so first tick always updates it
            high_ts: now,
            low: f64::MAX,  // Set to MAX so first tick always updates it
            low_ts: now,
            close: 0.0,
            volume: 0.0,
            bars: 0,
        }
    }
}


// A. Struct for data FETCHED from the 8hr_base SOURCE VIEW (Raw)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Raw8HrBlock {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub block_number: i32,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub bars: i64,
}

// B. Struct for data PERSISTED to the block_base TARGET TABLE (Classified)
#[derive(Debug, Clone)]
pub struct Classified8HrBlock {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub block_number: i32,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub bars: i64,
    pub block_type: MarketType, // Use String for persistence if MarketType is not directly serializable
    // pub consolidation_subtype: MarketSubtype, // Use String for persistence if MarketSubtype is not directly serializable
    
    pub body_ratio: f64,
    pub up_wick_ratio: f64,
    pub lo_wick_ratio: f64,
    // pub total_wick_ratio: f64,
    pub session_range: f64,
}


// ====================================================================
// 3. Raw Daily Data (Input - mirrors the daily_base table)
// ====================================================================

/// Input data structure for the daily views ETL (mirrors daily_base).
#[derive(Debug, sqlx::FromRow, Clone)]
pub struct RawDailyData {
    pub trading_date: chrono::NaiveDate,
    pub asset_id: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64, 
    pub bars: i64,   
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub high_ts: DateTime<Utc>,
    pub high_session: String,
    pub low_ts: DateTime<Utc>,
    pub low_session: String,
    pub high_bar: i32,
    pub low_bar: i32,
}


// ====================================================================
// 4. Classified Daily View (Output - mirrors daily_views table)
// ====================================================================

/// Output data structure for the daily_views table (the L2 feature set).
#[derive(Debug, Clone)]
pub struct ClassifiedDailyView {
    pub trading_date: chrono::NaiveDate,
    pub asset_id: String,
    pub dow: String, 
    pub day_type: MarketType,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub bars: i64,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub high_ts: DateTime<Utc>,
    pub high_session: String,
    pub low_ts: DateTime<Utc>,
    pub low_session: String,
    pub high_bar: i32,
    pub low_bar: i32,
    pub prior_day_high: Option<f64>,
    pub prior_day_low: Option<f64>,
    pub prior_week_high: Option<f64>,
    pub prior_week_low: Option<f64>,
    pub body_ratio: f64,
    pub up_wick_ratio: f64,
    pub lo_wick_ratio: f64,
    // pub total_wick_ratio: f64,
    pub day_range: f64,
}


// ====================================================================
// 5. Input for Weekly Views (mirrors daily_views for fetch)
// ====================================================================

/// Input data structure for the weekly views ETL (mirrors the necessary daily_views fields).
#[derive(Debug, sqlx::FromRow, Clone)]
pub struct RawDailyView {
    pub trading_date: chrono::NaiveDate,
    pub asset_id: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
    pub bars: i64,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub high_ts: DateTime<Utc>,
    pub high_session: String,
    pub low_ts: DateTime<Utc>,
    pub low_session: String,
    //pub high_bar: i32,
   // pub low_bar: i32,
}

// ====================================================================
// 6. Classified Weekly View (Output - mirrors weekly_views table)
// ====================================================================

/// The Rust struct representation of the final weekly_views table (L3).
#[derive(Debug, Clone)]
pub struct ClassifiedWeeklyView {
    pub week_start: chrono::NaiveDate,
    pub asset_id: String,
    pub month_of_year: i32,
    pub weekly_type: MarketType,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
    pub bars: i64,
    pub high_trading_date: chrono::NaiveDate,
    pub high_ts: DateTime<Utc>,
    pub high_session: String,
    pub low_trading_date: chrono::NaiveDate,
    pub low_ts: DateTime<Utc>,
    pub low_session: String,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub body_ratio: f64,
    pub up_wick_ratio: f64,
    pub lo_wick_ratio: f64,
    // pub total_wick_ratio: f64,
    pub week_range: f64,
}

// ====================================================================
// 7. Input for Monthly Views (mirrors weekly_views for fetch)
// ====================================================================

/// Input data structure for the monthly views ETL (mirrors the necessary weekly_views fields).
// #[derive(Debug, sqlx::FromRow, Clone)]
// pub struct RawWeeklyView {
//     pub week_start: chrono::NaiveDate,
//     pub asset_id: String,
//     pub open: f64,
//     pub high: f64,
//     pub low: f64,
//     pub close: f64,
//     pub volume: i64,
//     pub bars: i64,
//     pub high_trading_date: chrono::NaiveDate,
//     pub high_ts: DateTime<Utc>,
//     pub high_session: String,
//     pub low_trading_date: chrono::NaiveDate,
//     pub low_ts: DateTime<Utc>,
//     pub low_session: String,
// }

// ====================================================================
// 8. Classified Monthly View (Output - mirrors monthly_views table)
// ====================================================================

/// The Rust struct representation of the final monthly_views table (L4).
#[derive(Debug, Clone)]
pub struct ClassifiedMonthlyView {
    pub month_start: chrono::NaiveDate,
    pub asset_id: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
    pub bars: i64,
    pub monthly_type: MarketType,
    
    // Metadata fields
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub start_trading_date: chrono::NaiveDate, // The date of the first trade (first week's start)
    pub end_trading_date: chrono::NaiveDate,   // The date of the last trade (last week's start)
    pub high_trading_date: chrono::NaiveDate,
    pub high_ts: DateTime<Utc>,
    pub high_session: String,
    pub low_trading_date: chrono::NaiveDate,
    pub low_ts: DateTime<Utc>,
    pub low_session: String,

    pub body_ratio: f64,
    pub up_wick_ratio: f64,
    pub lo_wick_ratio: f64,
    //pub total_wick_ratio: f64,
    pub month_range: f64,
}

// ====================================================================
// 9. Input for Yearly Views (mirrors monthly_views for fetch)
// ====================================================================

/// Input data structure for the yearly views ETL (mirrors the necessary monthly_views fields).
#[derive(Debug, sqlx::FromRow, Clone)]
pub struct RawMonthlyView {
    pub month_start: chrono::NaiveDate,
    pub asset_id: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
    pub bars: i64,
    pub start_trading_date: chrono::NaiveDate,
    pub end_trading_date: chrono::NaiveDate,
    //pub start_ts: DateTime<Utc>,
   // pub end_ts: DateTime<Utc>,
    pub high_trading_date: chrono::NaiveDate,
    pub high_ts: DateTime<Utc>,
    pub high_session: String,
    pub low_trading_date: chrono::NaiveDate,
    pub low_ts: DateTime<Utc>,
    pub low_session: String,
}

// ====================================================================
// 10. Classified Yearly View (Output - mirrors yearly_views table)
// ====================================================================

/// The Rust struct representation of the final yearly_views table (L5).
#[derive(Debug, Clone)]
pub struct ClassifiedYearlyView {
    pub year_start: chrono::NaiveDate,
    pub asset_id: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
    pub bars: i64,
    pub yearly_type: MarketType,
    
    // Metadata fields
    pub high_trading_date: chrono::NaiveDate,
    pub high_ts: DateTime<Utc>,
    pub high_session: String,
    pub low_trading_date: chrono::NaiveDate,
    pub low_ts: DateTime<Utc>,
    pub low_session: String,
    // pub start_ts: DateTime<Utc>,
    // pub end_ts: DateTime<Utc>,

    pub body_ratio: f64,
    pub up_wick_ratio: f64,
    pub lo_wick_ratio: f64,
    //pub total_wick_ratio: f64,
    pub year_range: f64,
}