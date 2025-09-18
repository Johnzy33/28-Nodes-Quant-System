use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use surrealdb::sql::Datetime;

/// Parses "YYYY.MM.DD HH:MM:SS" into a SurrealDB Datetime.
/// Assumes the input is UTC.
pub fn parse_ymd_hms_to_datetime(s: &str) -> Result<Datetime> {
    let fmt = "%Y.%m.%d %H:%M:%S";
    let naive = NaiveDateTime::parse_from_str(s, fmt)
        .map_err(|e| anyhow!("failed to parse '{}': {}", s, e))?;
    let dt = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
    Ok(Datetime::from(dt))
}

/// Formats a SurrealDB Datetime into an ISO-8601 UTC string.
pub fn format_datetime_iso(dt: &Datetime) -> String {
    let chrono_dt: DateTime<Utc> = dt.clone().into(); // Convert Datetime to Chrono's DateTime
    chrono_dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}