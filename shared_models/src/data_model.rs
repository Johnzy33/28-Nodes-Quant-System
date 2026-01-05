
use chrono::{DateTime, Utc, NaiveDate}; 
use sqlx::{PgPool, Result as SqlxResult};
use sqlx::{FromRow};
use crate::market_classification::MarketType;
use anyhow::{Context, Result};
 use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    config::ClientConfig,
};
use std::env;



#[derive(Clone)]
pub struct DataService {
pub pool: PgPool,

}

impl DataService{
    pub fn new(pool: PgPool) -> Self {
        DataService { pool }
    }  

    pub async fn initialize_producer() -> Result<FutureProducer> {
    let brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .create()
        .context("Producer creation error")?;

    Ok(producer)
}
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