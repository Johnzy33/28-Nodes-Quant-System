use tokio::process::{Command, Child};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use anyhow::{Result, Context};
use log::{info, warn, error};
use chrono::{Datelike, Utc, Timelike};
use serde::Deserialize;
use std::fs;
use dotenvy::dotenv;
use std::env;
use std::path::PathBuf;

#[derive(Deserialize)]
struct HolidayConfig {
    holidays: Vec<Holiday>,
}

#[derive(Deserialize)]
struct Holiday {
    date: String, // Format: YYYY-MM-DD
}

#[derive(Clone)]
pub struct WatchdogState {
    pub last_data_received: Arc<AtomicU64>,
}

impl WatchdogState {
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        Self {
            last_data_received: Arc::new(AtomicU64::new(now)),
        }
    }

    pub fn update(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.last_data_received.store(now, Ordering::SeqCst);
    }
}

pub struct Mt5Watchdog {
    python_script: String,
    state: WatchdogState,
}

impl Mt5Watchdog {
    pub fn new(script_path: &str, state: WatchdogState) -> Self {
        Self {
            python_script: script_path.to_string(),
            state,
        }
    }


    fn get_path(&self, var_name: &str) -> PathBuf {
        dotenv().ok(); // Load .env
        let base = env::var("MARKET_DATA_DIR").unwrap_or_else(|_| ".".to_string());
        let file = env::var(var_name).unwrap_or_else(|_| "default.json".to_string());
        
        let mut path = PathBuf::from(base);
        path.push(file);
        path
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut child = self.spawn_worker()?;
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            // 1. Check if the process itself died (OS level)
            if let Ok(Some(status)) = child.try_wait() {
                error!("Watchdog: Python process exited (Status: {}). Restarting...", status);
                self.restart_worker(&mut child).await?;
                continue;
            }

            // 2. Freshness Check during market hours
            if self.is_market_open() {
                let now_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
                let last_ms = self.state.last_data_received.load(Ordering::SeqCst);
                
                // Threshold: 10 minutes (600,000 ms)
                if (now_ms - last_ms) > 600_000 {
                    // Check if MT5 is actually disconnected vs just a quiet holiday
                    if !self.check_mt5_connection_status().await {
                        warn!("🚨 MT5 Disconnected or Frozen. Restarting...");
                        self.restart_worker(&mut child).await?;
                    } else {
                        info!("Market is quiet but MT5 is connected. Likely a holiday or low liquidity.");
                    }
                }
            }
        }
    }

    fn is_market_open(&self) -> bool {
        let now = Utc::now();
        
        // Weekend check
        let day = now.weekday();
        if day == chrono::Weekday::Sat { return false; }
        if day == chrono::Weekday::Fri && now.hour() >= 22 { return false; }
        if day == chrono::Weekday::Sun && now.hour() < 22 { return false; }
        
        // Maintenance hour
        if now.hour() == 21 { return false; }

        // Holiday check
        if self.is_holiday(&now) {
            info!("Watchdog: Market is closed for holiday.");
            return false;
        }

        true
    }

    fn is_holiday(&self, now: &chrono::DateTime<Utc>) -> bool {
        let path = self.get_path("HOLIDAYS_FILE_NAME");
        let today_str = now.format("%Y-%m-%d").to_string();
        
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<HolidayConfig>(&content) {
                return config.holidays.iter().any(|h| h.date == today_str);
            }
        }
        false
    }

    async fn check_mt5_connection_status(&self) -> bool {
        let path = self.get_path("HEALTH_FILE_NAME");
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let connected = val["mt5_connected"].as_bool().unwrap_or(false);
                let last_ping = val["last_ping"].as_u64().unwrap_or(0);
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

                if connected && (now - last_ping) < 120_000 {
                    return true;
                }
            }
        }
        false
    }

    fn spawn_worker(&self) -> Result<Child> {
        info!("Watchdog: Spawning Wine MT5 Worker...");
        Command::new("wine")
            .arg("python.exe")
            .arg(&self.python_script)
            .spawn()
            .context("Failed to start Wine Python process")
    }

    async fn restart_worker(&mut self, child: &mut Child) -> Result<()> {
        let _ = child.kill().await;
        let _ = Command::new("wineserver").arg("-k").output().await;
        
        *child = self.spawn_worker()?;
        self.state.update(); 
        Ok(())
    }
}