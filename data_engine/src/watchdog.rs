// use tokio::process::{Command, Child};
// use std::time::{Duration, SystemTime, UNIX_EPOCH};
// use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
// use anyhow::{Result, Context};
// use log::{info, warn, error};
// use chrono::{Datelike, Utc, Timelike};
// use serde::Deserialize;
// use std::fs;
// use dotenvy::dotenv;
// use std::env;
// use std::path::PathBuf;
// use dashmap::DashMap;
// use bytemuck::{Pod, Zeroable};
// use shared_models::data_model::MARKET_SESSIONS;

// // lazy_static::lazy_static! {
// //     pub static ref MARKET_SESSIONS: DashMap<String, SessionPacket> = DashMap::new();
// // }

// #[derive(Deserialize)]
// struct HolidayConfig {
//     holidays: Vec<Holiday>,
// }

// #[derive(Deserialize)]
// struct Holiday {
//     date: String, // Format: YYYY-MM-DD
// }

// #[derive(Clone)]
// pub struct WatchdogState {
//     pub last_data_received: Arc<AtomicU64>,
// }

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

// impl WatchdogState {
//     pub fn new() -> Self {
//         let now = SystemTime::now()
//             .duration_since(UNIX_EPOCH)
//             .unwrap()
//             .as_millis() as u64;
//         Self {
//             last_data_received: Arc::new(AtomicU64::new(now)),
//         }
//     }

//     pub fn update(&self) {
//         let now = SystemTime::now()
//             .duration_since(UNIX_EPOCH)
//             .unwrap()
//             .as_millis() as u64;
//         self.last_data_received.store(now, Ordering::SeqCst);
//     }
// }

// pub struct Mt5Watchdog {
//     // python_script: String,
//     state: WatchdogState,
// }

// impl Mt5Watchdog {
//     pub fn new( state: WatchdogState) -> Self {
//         Self {
//             // python_script: script_path.to_string(),
//             state,
//         }
//     }


//     fn get_path(&self, var_name: &str) -> PathBuf {
//         dotenv().ok(); // Load .env
//         let base = env::var("MARKET_DATA_DIR").unwrap_or_else(|_| ".".to_string());
//         let file = env::var(var_name).unwrap_or_else(|_| "default.json".to_string());
        
//         let mut path = PathBuf::from(base);
//         path.push(file);
//         path
//     }

//     pub async fn run(&mut self) -> Result<()> {
//         let mut child = self.spawn_worker()?;
//         let mut interval = tokio::time::interval(Duration::from_secs(30));

//         loop {
//             interval.tick().await;

//             // 1. Check if the process itself died (OS level)
//             if let Ok(Some(status)) = child.try_wait() {
//                 error!("Watchdog: Python process exited (Status: {}). Restarting...", status);
//                 self.restart_worker(&mut child).await?;
//                 continue;
//             }

//             // 2. Freshness Check during market hours
//             if self.is_market_open() {
//                 let now_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
//                 let last_ms = self.state.last_data_received.load(Ordering::SeqCst);
                
//                 // Threshold: 10 minutes (600,000 ms)
//                 if (now_ms - last_ms) > 600_000 {
//                     // Check if MT5 is actually disconnected vs just a quiet holiday
//                     if !self.check_mt5_connection_status().await {
//                         warn!("🚨 MT5 Disconnected or Frozen. Restarting...");
//                         self.restart_worker(&mut child).await?;
//                     } else {
//                         info!("Market is quiet but MT5 is connected. Likely a holiday or low liquidity.");
//                     }
//                 }
//             }
//         }
//     }

//     // fn is_market_open(&self) -> bool {
//     //     let now = Utc::now();
        
//     //     // Weekend check
//     //     let day = now.weekday();
//     //     if day == chrono::Weekday::Sat { return false; }
//     //     if day == chrono::Weekday::Fri && now.hour() >= 22 { return false; }
//     //     if day == chrono::Weekday::Sun && now.hour() < 22 { return false; }
        
//     //     // Maintenance hour
//     //     if now.hour() == 21 { return false; }

//     //     // Holiday check
//     //     if self.is_holiday(&now) {
//     //         info!("Watchdog: Market is closed for holiday.");
//     //         return false;
//     //     }

//     //     true
//     // }

//     pub fn is_market_open(&self) -> bool {
//         let now = Utc::now();
//         let hour = now.hour();
//         let day = now.weekday();

//         // 1. Hard Weekend Gate
//         if day == chrono::Weekday::Sat { return false; }
        
//         // 2. Standard Friday Close (22:00 UTC)
//         if day == chrono::Weekday::Fri && hour >= 22 { return false; }

//         // 3. Re-opening Logic (Sunday or Holiday)
//         // Most markets (MT5/Liquidity Providers) wake up at 22:00 UTC
//         let is_reopening_period = hour >= 22;

//         if day == chrono::Weekday::Sun {
//             if !is_reopening_period { return false; }
//             // If it's Sunday after 22:00, we are OPEN.
//         }

//         // 4. Holiday Gate with Re-opening Exception
//         if self.is_holiday(&now) {
//             if is_reopening_period {
//                 // Market is starting to breathe again
//                 return true;
//             }
//             info!("Watchdog: Market is closed for holiday ({})", now.format("%Y-%m-%d"));
//             return false;
//         }

//         // 5. Daily Maintenance Window (21:00 - 22:00 UTC)
//         // Very important for MT5/Indices to avoid "ghost" data or freeze-restarts
//         if hour == 21 { return false; }

//         true
//     }

//     fn is_holiday(&self, now: &chrono::DateTime<Utc>) -> bool {
//         let path = self.get_path("HOLIDAYS_FILE_NAME");
//         let today_str = now.format("%Y-%m-%d").to_string();
        
//         if let Ok(content) = fs::read_to_string(path) {
//             if let Ok(config) = serde_json::from_str::<HolidayConfig>(&content) {
//                 return config.holidays.iter().any(|h| h.date == today_str);
//             }
//         }
//         false
//     }

//     async fn check_mt5_connection_status(&self) -> bool {
//         let path = self.get_path("HEALTH_FILE_NAME");
//         if let Ok(content) = fs::read_to_string(path) {
//             if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
//                 let connected = val["mt5_connected"].as_bool().unwrap_or(false);
//                 let last_ping = val["last_ping"].as_u64().unwrap_or(0);
//                 let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

//                 if connected && (now - last_ping) < 120_000 {
//                     return true;
//                 }
//             }
//         }
//         false
//     }

//     fn spawn_worker(&self) -> Result<Child> {
//         info!("Watchdog: Spawning Wine MT5 Worker...");
//         Command::new("wine")
//             .arg("python.exe")
//             // .arg(&self.python_script)
//             .spawn()
//             .context("Failed to start Wine Python process")
//     }

//     async fn restart_worker(&mut self, child: &mut Child) -> Result<()> {
//         let _ = child.kill().await;
//         let _ = Command::new("wineserver").arg("-k").output().await;
        
//         *child = self.spawn_worker()?;
//         self.state.update(); 
//         Ok(())
//     }

//    pub  fn is_market_actually_open( asset_id: &str) -> bool {
//     let now = chrono::Utc::now();
//     let current_minutes = (now.hour() * 60) + now.minute();

//     if let Some(session) = MARKET_SESSIONS.get(asset_id) {
//         if session.is_active == 0 { return false; }

//         let start_total = (session.open_hour * 60) + session.open_min;
//         let end_total = (session.close_hour * 60) + session.close_min;

//         // If current time is within the broker's session window
//         return current_minutes >= start_total && current_minutes < end_total;
//     }

//     // Default to true if we haven't received a session packet yet
//     true 
// }
// }




use std::sync::{Arc, atomic::{AtomicU64, Ordering}, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use log::{info, warn, error};
use chrono::{Utc, Timelike, Datelike};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};
use dotenvy::dotenv;
use std::env;
use bytemuck::{Pod, Zeroable};

#[derive(Deserialize)]
struct HolidayConfig {
    holidays: Vec<Holiday>,
}

#[derive(Deserialize)]
struct Holiday {
    date: String,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SessionPacket {
    pub asset: [u8; 16],
    pub open_hour: u32,
    pub open_min: u32,
    pub close_hour: u32,
    pub close_min: u32,
    pub is_active: u32, // 1 = trading day, 0 = closed/holiday
}

#[derive(Clone)]
pub struct WatchdogState {
    pub last_data_received: Arc<AtomicU64>,
    pub restart_history: Arc<Mutex<Vec<u64>>>,
}

impl WatchdogState {
    pub fn new() -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        Self {
            last_data_received: Arc::new(AtomicU64::new(now)),
            restart_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn update(&self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        self.last_data_received.store(now, Ordering::SeqCst);
    }

    pub fn can_restart(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut history = self.restart_history.lock().unwrap();
        history.retain(|&ts| now - ts < 3600); // Rolling 1-hour window
        history.len() < 3
    }

    pub fn record_restart(&self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut history = self.restart_history.lock().unwrap();
        history.push(now);
    }
}

pub struct Mt5Watchdog {
    state: WatchdogState,
}

impl Mt5Watchdog {
    pub fn new(state: WatchdogState) -> Self {
        Self { state }
    }

    /// Dynamically resolves paths based on .env variables
    fn get_config_path(&self, var_name: &str) -> PathBuf {
        dotenv().ok(); 
        let base = env::var("MARKET_DATA_DIR").unwrap_or_else(|_| ".".to_string());
        let file = env::var(var_name).unwrap_or_else(|_| "default.json".to_string());
        
        let mut path = PathBuf::from(base);
        path.push(file);
        path
    }

    pub async fn run(&self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        info!("🐕 Watchdog active (Env-Aware): Monitoring mt5.service health...");

        loop {
            interval.tick().await;

            if self.is_market_open() {
                let now_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
                let last_ms = self.state.last_data_received.load(Ordering::SeqCst);
                
                // Threshold: 10 minutes of no data (600,000 ms)
                if (now_ms - last_ms) > 600_000 {
                    warn!("🚨 MT5 Stale Data Detected. Checking Circuit Breaker...");
                    
                    if !self.state.can_restart() {
                        error!("🛑 CIRCUIT BREAKER: 3 restarts in 1hr. Manual intervention required.");
                        continue;
                    }

                    self.restart_mt5_service().await?;
                }
            }
        }
    }

    async fn restart_mt5_service(&self) -> Result<()> {
        info!("🔄 Executing: sudo systemctl restart mt5.service");
        
        let status = tokio::process::Command::new("sudo")
            .args(["systemctl", "restart", "mt5.service"])
            .status()
            .await?;

        if status.success() {
            self.state.record_restart();
            info!("✅ mt5.service restarted. Data timer reset.");
            self.state.update(); 
            tokio::time::sleep(Duration::from_secs(60)).await;
        } else {
            error!("❌ Failed to restart mt5.service. Check sudoers and systemctl.");
        }
        Ok(())
    }

    pub fn is_market_open(&self) -> bool {
        let now = Utc::now();
        let hour = now.hour();
        let day = now.weekday();

        if day == chrono::Weekday::Sat { return false; }
        if day == chrono::Weekday::Fri && hour >= 22 { return false; }
        
        let is_reopening_period = hour >= 22;
        if day == chrono::Weekday::Sun {
            if !is_reopening_period { return false; }
        }

        if self.is_holiday(&now) {
            if is_reopening_period { return true; }
            return false;
        }

        if hour == 21 { return false; }

        true
    }

    fn is_holiday(&self, now: &chrono::DateTime<Utc>) -> bool {
        let path = self.get_config_path("HOLIDAYS_FILE_NAME");
        let today_str = now.format("%Y-%m-%d").to_string();
        
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<HolidayConfig>(&content) {
                return config.holidays.iter().any(|h| h.date == today_str);
            }
        }
        false
    }
}