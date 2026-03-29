use serde::{Deserialize, Serialize};
use surrealdb_types::{SurrealValue, RecordId};




#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct MarketData {
    pub asset_id: RecordId,    
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,    
}

