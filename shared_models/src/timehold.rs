use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, Utc};

/// Converts a Unix timestamp (milliseconds) into a UTC DateTime object for Postgres.
pub fn ts_to_utc_datetime(ts_ms: i64) -> Result<DateTime<Utc>> {
    let secs = ts_ms / 1000;
    let nsecs = (ts_ms % 1000) as u32 * 1_000_000;
    
    match NaiveDateTime::from_timestamp_opt(secs, nsecs) {
        Some(ndt) => Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
        None => Err(anyhow!("Invalid timestamp: {}", ts_ms)),
    }
}

/// Parses "YYYY.MM.DD HH:MM:SS" into a UTC DateTime object.
pub fn parse_ymd_hms_to_utc_datetime(s: &str) -> Result<DateTime<Utc>> {
    let fmt = "%Y.%m.%d %H:%M";
    let naive = NaiveDateTime::parse_from_str(s, fmt)
        .map_err(|e| anyhow!("failed to parse '{}': {}", s, e))?;
        
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

// Ok(ny_datetime.with_timezone(&Utc))