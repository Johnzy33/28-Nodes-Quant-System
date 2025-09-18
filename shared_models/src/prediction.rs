use serde::{Deserialize, Serialize};
use crate::prelude::{Id, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Prediction {
    pub id: Option<Id>,
    pub asset_id: Id,
    pub ts: Timestamp,         // when prediction was made
    pub horizon_ms: i64,       // prediction horizon in ms
    pub predicted_price: f64,
    pub confidence: Option<f32>,
    pub model_id: Option<String>,
    pub input_market_data_id: Option<Id>, // optional link to MarketData
    pub notes: Option<String>,
}

impl Prediction {
    pub fn new(asset_id: Id, ts: Timestamp, horizon_ms: i64, predicted_price: f64) -> Self {
        Self {
            id: None,
            asset_id,
            ts,
            horizon_ms,
            predicted_price,
            confidence: None,
            model_id: None,
            input_market_data_id: None,
            notes: None,
        }
    }
}
