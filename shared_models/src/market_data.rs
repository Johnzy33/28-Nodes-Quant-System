use serde::{Deserialize, Serialize};

/// Core data contract for Kafka. 1-Hour candle data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketData {
    pub asset_id: String,   // Foreign Key
    pub ts: i64,            // Unix milliseconds
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub seq: Option<u64>,
    pub source: Option<String>,
}