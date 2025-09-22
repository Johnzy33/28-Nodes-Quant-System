
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Thing};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionAgg {
    pub id: Option<Thing>,
    pub asset_id: Thing,
    pub date: String,
    pub session: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub high_ts: Timestamp,
    pub low_ts: Timestamp,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyAgg {
    pub id: Option<Thing>,
    pub asset_id: Thing,
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub high_ts: Timestamp,
    pub low_ts: Timestamp,
}

