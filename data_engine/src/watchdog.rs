
use std::sync::{Arc, atomic::{AtomicU64, Ordering}, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use log::{info, warn, error};
use chrono::{Utc, Timelike, Datelike};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use anyhow::{Result};
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
        info!("Watchdog active (Env-Aware): Monitoring mt5.service health...");

        loop {
            interval.tick().await;

            if self.is_market_open() {
                let now_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
                let last_ms = self.state.last_data_received.load(Ordering::SeqCst);
                
                // Threshold: 10 minutes of no data (600,000 ms) //24 hrs = 86,400,000 ms
                if (now_ms - last_ms) > 864_000_000 {
                    warn!("MT5 Stale Data Detected. Checking Circuit Breaker...");
                    
                    if !self.state.can_restart() {
                        error!("CIRCUIT BREAKER: 3 restarts in 1hr. Manual intervention required.");
                        continue;
                    }

                    self.restart_mt5_service().await?;
                }
            }
        }
    }

    async fn restart_mt5_service(&self) -> Result<()> {
        info!("Executing: sudo systemctl restart mt5.service");
        
        let status = tokio::process::Command::new("sudo")
            .args(["systemctl", "restart", "mt5.service"])
            .status()
            .await?;

        if status.success() {
            self.state.record_restart();
            info!(" mt5.service restarted. Data timer reset.");
            self.state.update(); 
            tokio::time::sleep(Duration::from_secs(60)).await;
        } else {
            error!(" Failed to restart mt5.service. Check sudoers and systemctl.");
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

        if hour == 22 { return false; }

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