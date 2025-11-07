use anyhow::{anyhow, Result};
use chrono::{DateTime, TimeZone,NaiveDateTime, Utc};
use chrono_tz::Tz;

// Define the target timezone once for clarity and correctness
const SERVER_TIME: Tz = chrono_tz::Europe::Athens; 

/// Converts a Unix timestamp (milliseconds) into a UTC DateTime object for Postgres.
/// (No change needed here as Unix timestamps are inherently UTC)
pub fn ts_to_utc_datetime(ts_ms: i64) -> Result<DateTime<Utc>> {
    let secs = ts_ms / 1000;
    let nsecs = (ts_ms % 1000) as u32 * 1_000_000;
    
    match NaiveDateTime::from_timestamp_opt(secs, nsecs) {
        Some(ndt) => Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
        None => Err(anyhow!("Invalid timestamp: {}", ts_ms)),
    }
}

/// Parses "YYYY.MM.DD HH:MM" and anchors it to New York Time (NYT) 
/// before converting to UTC for the database.
pub fn parse_ymd_hms_to_utc_datetime(s: &str) -> Result<DateTime<Utc>> {
    let fmt = "%Y.%m.%d %H:%M:%S";
    
    // 1. Parse the string into a NaiveDateTime (timezone-less).
    let naive = NaiveDateTime::parse_from_str(s, fmt)
        .map_err(|e| anyhow!("failed to parse '{}': {}", s, e))?;
        
    // 2. Anchor the NaiveDateTime to the explicit New York Timezone (the crucial step).
    let server_time = SERVER_TIME.from_local_datetime(&naive)
        .single() // Use .single() to handle unambiguous times
        .ok_or_else(|| anyhow!("Ambiguous or invalid time encountered near a DST transition: {}", s))?;

    // 3. Convert the anchored sever time to the absolute UTC time.
    // This is the correct value to send to the PostgreSQL database.
    Ok(server_time.with_timezone(&Utc))
    
}