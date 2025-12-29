use anyhow::{anyhow, Result};
use chrono::{DateTime, TimeZone, NaiveDateTime, Utc, Duration, Datelike, NaiveDate, Weekday};
use chrono_tz::Tz;
use chrono_tz::America::New_York;
use chrono_tz::Europe::Athens;

// --- CONSTANTS ---
// Timezone used to anchor incoming non-timezone-aware data (e.g., from external APIs/files).
const ANCHOR_TIMEZONE: Tz = Athens; 
// The trading day calculation uses New York Time to align with the AS session start.
const TRADING_TIMEZONE: Tz = New_York;
// The PL/pgSQL shift: ts + INTERVAL '6 hours'.
const TRADING_DAY_SHIFT_HOURS: i64 = 6;


// =========================================================================
// 1. RAW TIMESTAMP CONVERSION
// =========================================================================

/// Converts a Unix timestamp (milliseconds) into a UTC DateTime object.
pub fn ts_to_utc_datetime(ts_ms: i64) -> Result<DateTime<Utc>> {
    let secs = ts_ms / 1000;
    let nsecs = (ts_ms % 1000) as u32 * 1_000_000;
    
    match NaiveDateTime::from_timestamp_opt(secs, nsecs) {
        Some(ndt) => Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)),
        None => Err(anyhow!("Invalid timestamp: {}", ts_ms)),
    }
}

/// Parses "YYYY.MM.DD HH:MM:SS" string and anchors it to the ANCHOR_TIMEZONE 
/// before converting to UTC.
pub fn parse_ymd_hms_to_utc_datetime(s: &str) -> Result<DateTime<Utc>> {
    let fmt = "%Y.%m.%d %H:%M:%S"; 
    
    // 1. Parse the string into a NaiveDateTime (timezone-less).
    let naive = NaiveDateTime::parse_from_str(s, fmt)
        .map_err(|e| anyhow!("Failed to parse '{}': {}", s, e))?;
        
    // 2. Anchor the NaiveDateTime to the ANCHOR_TIMEZONE.
    let anchored_time = ANCHOR_TIMEZONE.from_local_datetime(&naive)
        .single() 
        .ok_or_else(|| anyhow!("Ambiguous or invalid time encountered near a DST transition: {}", s))?;

    // 3. Convert the anchored time to the absolute UTC time for the database.
    Ok(anchored_time.with_timezone(&Utc))
}


// =========================================================================
// 2. TRADING DAY CALCULATION (PL/pgSQL Conversion)
// =========================================================================

/// Converts a UTC timestamp (in milliseconds) into the start of its defined Trading Date.
/// Corresponds to the public.get_trading_date PL/pgSQL function:
/// SELECT date_trunc('day', ts + INTERVAL '6 hours') AT TIME ZONE 'America/New_York';
pub fn get_trading_date_start_ms(ts_ms: i64) -> Result<i64> {
    
    let utc_dt = ts_to_utc_datetime(ts_ms)?;

    // 1. Apply the 6-hour forward shift (ts + INTERVAL '6 hours').
    let shifted_utc_dt = utc_dt + Duration::hours(TRADING_DAY_SHIFT_HOURS);

    // 2. Truncate the shifted time to the start of the calendar day (midnight UTC).
    let shifted_midnight_utc = shifted_utc_dt.date_naive().and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("Failed to truncate shifted time."))?;

    // 3. The final result is the timestamp (in milliseconds) of that midnight UTC.
    let final_utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(shifted_midnight_utc, Utc);
    
    Ok(final_utc_dt.timestamp_millis())
}


// =========================================================================
// 3. ANCHOR TIME BOUNDARY CALCULATION (L1, L3, L4, L2)
// =========================================================================

/// Calculates the start of the previous trading year (January 1st, 00:00 UTC of the prior year).
pub fn get_prev_year_start(now_ms: i64) -> Result<i64> {
    let now = ts_to_utc_datetime(now_ms)?;
    let prev_year = now.year() - 1;
    
    let start_date = NaiveDate::from_ymd_opt(prev_year, 1, 1) // Corrected: NaiveDate
        .ok_or_else(|| anyhow!("Invalid date for previous year start."))?;

    let start_datetime = start_date.and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("Invalid time component for start date."))?;
        
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(start_datetime, Utc).timestamp_millis())
}

/// Calculates the start of the previous trading week (Monday 00:00 UTC of the prior week).
pub fn get_prev_week_start(now_ms: i64) -> Result<i64> {
    let now = ts_to_utc_datetime(now_ms)?;
    
    // 1. Calculate how many days back to get to the start of the current Monday (00:00 UTC)
    let days_since_monday = now.weekday().num_days_from_monday();
    let current_monday_naive = now.date_naive().and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("Failed to get current Monday start."))?;
    let current_monday = DateTime::<Utc>::from_naive_utc_and_offset(current_monday_naive, Utc) - Duration::days(days_since_monday as i64);

    // 2. Calculate the Monday of the *previous* week (subtract 7 days).
    let prev_monday = current_monday - Duration::days(7);
    
    Ok(prev_monday.timestamp_millis())
}

/// Placeholder for the start of the Critical H8 (L2) swing period.
pub fn get_critical_h8_start(time_now: i64) -> Result<i64> {
    // Placeholder: Return the timestamp for a fixed 60-day lookback 
    let sixty_days_ms = 60 * 24 * 60 * 60 * 1000;
    Ok(time_now - sixty_days_ms)
}