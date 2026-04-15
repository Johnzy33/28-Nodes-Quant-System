
// session_utils.rs in share_model 
use anyhow::{anyhow, Result};
use chrono::{DateTime, Timelike};
use chrono_tz::Tz;
use chrono_tz::America::New_York;


#[derive(Debug, PartialEq, Clone)]
pub enum TradingSession {
    AS,   
    LN,    
    NYAM,  
    NYL,   
    NYPM,  
    Unknown,
}

// The Trading Timezone is America/New_York
const TRADING_TIMEZONE: Tz = New_York;


pub fn get_trading_session(ts_ms: i64) -> Result<TradingSession> {
    
    let secs = ts_ms / 1000;
    let nsecs = (ts_ms % 1000) as u32 * 1_000_000;
    
    let utc_dt = match DateTime::from_timestamp(secs, nsecs) {
        Some(dt) => dt,
        None => return Err(anyhow!("Invalid timestamp: {}", ts_ms)),
    };
    
    
    let ny_dt = utc_dt.with_timezone(&TRADING_TIMEZONE);
    let hour = ny_dt.hour(); 
    

    let session = match hour {
        
        2..=7 => TradingSession::LN,
        8..=11 => TradingSession::NYAM,
        12..=13 => TradingSession::NYL,
        14..=17 => TradingSession::NYPM,
        18..=23 | 0..=1 => TradingSession::AS,
        _ => TradingSession::Unknown,
    };
    
    Ok(session)
}