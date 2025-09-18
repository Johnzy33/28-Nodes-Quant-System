use serde::{Deserialize, Serialize};
use crate::prelude::{Id, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketData {
    pub id: Option<Id>,            // optional DB id (e.g., "market_data:...") 
    pub asset_id: Id,              // record<assets> as String (neutral)
    pub ts: Timestamp,             // unix ms
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: f64,
    pub volume: Option<f64>,
    pub seq: Option<u64>,          // sequence number from feed
    pub source: Option<String>,    // feed id / exchange
}

impl MarketData {
    pub fn new(asset_id: Id, ts: Timestamp, close: f64) -> Self {
        Self {
            id: None,
            asset_id,
            ts,
            open: None,
            high: None,
            low: None,
            close,
            volume: None,
            seq: None,
            source: None,
        }
    }
}
