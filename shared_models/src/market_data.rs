use serde::{Deserialize, Serialize};

use crate::MarketRatios;

use smol_str::SmolStr;


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// pub struct MarketData {
//     pub asset_id: String,   
//     pub ts: i64,            
//     pub open: f64,
//     pub high: f64,
//     pub low: f64,
//     pub close: f64,
//     pub volume: f64,
//     pub seq: Option<u64>,
//     pub source: Option<String>,
//     // pub ratios: MarketRatios,
// }

pub struct MarketData {
    pub asset_id: SmolStr,      // "assets:XAUUSD:MT5" is 17 chars (fits on stack)
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    // pub last_bid: Option<f64>,
    // pub last_ask: Option<f64>,
    // pub last_ts: Option<i64>,

    // last_bid: tick_data.bid,
            // last_ask: tick_data.ask,
            // last_ts: tick_data.ts,
    pub volume: f64,
    pub source: Option<SmolStr>,
    pub seq: Option<u64>,
}

pub struct TickTracker {
    pub last_bid: f64,
    pub last_ask: f64,
    pub last_ts_msc: i64,
    pub last_m1_ts: i64,             // The start timestamp of the current M1 bar
    pub vol_before_current_m1: f64,
}

pub struct MarketDataLive {
    pub asset_id: SmolStr,      // "assets:XAUUSD:MT5" is 17 chars (fits on stack)
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub last_bid: Option<f64>,
    pub last_ask: Option<f64>,
    pub last_ts: Option<i64>,

    // last_bid: tick_data.bid,
            // last_ask: tick_data.ask,
            // last_ts: tick_data.ts,
    pub volume: f64,
    pub source: Option<SmolStr>,
    pub seq: Option<u64>,
}