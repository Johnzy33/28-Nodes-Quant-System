use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use surrealdb::engine::remote::ws::{Client};
use surrealdb::Surreal;
use surrealdb_types::{SurrealValue, RecordId,  RecordIdKey,Datetime};
use std::collections::BTreeMap;
pub struct AppDatabases {
    pub mem: Surreal<Client>,
    pub disk: Surreal<Client>,
    pub mirror: Option<Surreal<Client>>,
}





#[derive(Serialize, Deserialize, Debug)]
pub struct DstRangeRaw {
    pub s: String,
    pub e: String,
}

#[derive(Serialize, Deserialize, Debug, SurrealValue)]
pub struct DstRange {
    pub s: Datetime,
    pub e: Datetime,
}

#[derive(Debug, Serialize, Deserialize, SurrealValue)]
pub struct DstConfig {
    pub years: HashMap<String, DstRange>,
}

#[derive(Debug, Serialize, Deserialize, SurrealValue, Clone)]
pub struct MonitorUpdate {
    pub symbol: String,
    pub source: String,
    pub last_bid: f64,
    pub last_ask: f64,
    pub volume: f64,
}

#[derive(Debug, Serialize, Deserialize, SurrealValue, Clone)]
pub struct MonitorUpdates {
    pub asset_id: RecordId,

    // #[serde(skip_serializing)] 
    #[serde(skip)]
    pub time_msc: i64,

    pub last_bid: f64,
    pub last_ask: f64,
    pub volume: f64,
    pub time: Option<Datetime>, 
}

#[derive(Debug, Serialize, Deserialize, SurrealValue, Clone)]

pub struct MarketData {
    pub asset_id: RecordId,
    pub time: Datetime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,      // Total Official Volume (from iTickVolume)
    pub buy_volume: f64,  // Share of volume from aggressive buyers
    pub sell_volume: f64, // Share of volume from aggressive sellers
    // Maps Price String -> (Buy Volume f64, Sell Volume f64)
    pub levels: BTreeMap<String, (f64, f64)>,
}
// pub struct MarketData {
//     pub asset_id: RecordId,
//     pub time: Datetime,
//     pub open: f64,
//     pub high: f64,
//     pub low: f64,
//     pub close: f64,
//     pub volume: f64,
    
// }


pub type AppError = Box<dyn std::error::Error + Send + Sync>;
pub type AppResult<T> = Result<T, AppError>;

// 1. Define a trait

#[derive(Serialize, SurrealValue, Clone)]
pub struct CandleIngestRow {
   pub id: RecordId,
    pub data: MarketData,
}

pub trait SurrealIdExt {
    fn to_raw_string(&self) -> String;
}

// Implement for the Key (what we did before)
impl SurrealIdExt for RecordIdKey {
    fn to_raw_string(&self) -> String {
        match self {
            RecordIdKey::Number(n) => n.to_string(),
            RecordIdKey::String(s) => s.clone(),
            RecordIdKey::Uuid(u) => u.to_string(),
            RecordIdKey::Array(arr) => arr.iter().map(|v| v.to_raw_string()).collect::<Vec<String>>().join(","),
            other => format!("{:?}", other),
        }
    }
}

// Implement for the full RecordId (Table + Key)
impl SurrealIdExt for RecordId {
    fn to_raw_string(&self) -> String {
        // This creates the "table:key" format
        format!("{}:{}", self.table, self.key.to_raw_string())
    }
}

impl SurrealIdExt for surrealdb_types::Value {
    fn to_raw_string(&self) -> String {
        match self {
            surrealdb_types::Value::String(s) => s.clone(),
            surrealdb_types::Value::Number(n) => n.to_string(),
            surrealdb_types::Value::RecordId(id) => id.key.to_raw_string(), // Recurse to get the key string
            surrealdb_types::Value::Datetime(dt) => dt.to_string(),
            surrealdb_types::Value::Array(arr) => arr.iter().map(|v| v.to_raw_string()).collect::<Vec<String>>().join(","),   
            other => format!("{:?}", other),
        }
    }
}


pub trait ToSurrealValue {
    fn to_value(&self) -> surrealdb_types::Value;
}

impl ToSurrealValue for RecordId {
    fn to_value(&self) -> surrealdb_types::Value {
        surrealdb_types::Value::RecordId(self.clone())
    }
}

impl ToSurrealValue for Datetime {
    fn to_value(&self) -> surrealdb_types::Value {
        surrealdb_types::Value::Datetime(self.clone())
    }
}

impl ToSurrealValue for String {
    fn to_value(&self) -> surrealdb_types::Value {
        surrealdb_types::Value::String(self.clone())
    }
}

impl ToSurrealValue for &str {
    fn to_value(&self) -> surrealdb_types::Value {
        surrealdb_types::Value::String(self.to_string())
    }
}

impl ToSurrealValue for surrealdb_types::Array {
    fn to_value(&self) -> surrealdb_types::Value {
        surrealdb_types::Value::Array(self.clone())
    }
}

impl ToSurrealValue for surrealdb_types::Value {
    fn to_value(&self) -> surrealdb_types::Value {
        self.clone()
    }
}


pub fn make_composite_id(table: &str, parts: &[&dyn ToSurrealValue]) -> RecordId {
    let mut arr = surrealdb_types::Array::new();
    
    for part in parts {
        arr.push(part.to_value());
    }

    RecordId {
        table: table.into(),
        key: surrealdb_types::RecordIdKey::Array(arr),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetMicrostructure {
    pub data_channels: DataChannels,
    pub latency_buffer_ms: f64,
    pub sessions: Vec<TradingSession>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DataChannels {
    pub l1: String,
    pub l2: String,
}



use bytemuck::{Pod, Zeroable};


#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct AssetCalculation {
    pub digits: i32,
    pub stops_level: i32,
    pub tick_value: f64,
    pub chart_mode: [u8; 16],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct AssetExecution {
    pub tick_size: f64,
    pub lot_size: f64,
    pub min_quantity: f64,
    pub volume_step: f64,
    pub max_quantity: f64,
    pub gtc_mode: [u8; 32],
    pub filling_modes: i32,
    pub order_modes: i32,
    pub expiration_modes: i32,
    pub _pad: i32, // Matches MQL5 padding
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct AssetSwapRates {
    pub monday: f64,
    pub tuesday: f64,
    pub wednesday: f64,
    pub thursday: f64,
    pub friday: f64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct AssetPnL {
    pub profit_currency: [u8; 8],
    pub swap_long: f64,
    pub swap_short: f64,
    pub swap_type: [u8; 16],
    pub swap_rates: AssetSwapRates,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct AssetRisk {
    pub initial_margin: f64,
    pub maintenance_margin: f64,
    pub initial_usd_per_lot: f64,
    pub max_leverage: f64,
    pub is_shortable: i32,
    pub margin_currency: [u8; 8],
    pub _pad: i32, // Matches MQL5 padding
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct AssetSession {
    pub is_active: i32,
    pub open_hour: i32,
    pub open_min: i32,
    pub close_hour: i32,
    pub close_min: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct AssetInfoPacket {
    pub asset: [u8; 16],
    pub name: [u8; 64],
    pub source: [u8; 16],
    pub asset_type: [u8; 16],
    pub sector: [u8; 32],
    pub industry: [u8; 32],
    pub calculation: AssetCalculation,
    pub execution: AssetExecution,
    pub risk: AssetRisk,
    pub pnl: AssetPnL,
    pub sessions: [AssetSession; 7],
    pub _pad: i32,
}

impl Default for AssetInfoPacket {
    fn default() -> Self {
        Self {
            asset: [0; 16],
            name: [0; 64],
            source: [0; 16],
            asset_type: [0; 16],
            sector: [0; 32],
            industry: [0; 32],
            calculation: AssetCalculation::default(),
            execution: AssetExecution::default(),
            risk: AssetRisk::default(),
            pnl: AssetPnL::default(),
            sessions: [AssetSession::default(); 7],
            _pad: 0,
        }
    }
}

// Model for SurrealDB storage



#[derive(Debug, Serialize, Deserialize,SurrealValue, Clone)]
pub struct Asset {
    pub symbol: String,
    pub name: String,
    pub source: String,
    pub asset_type: String,
    pub timezone: String,
    pub exchange: String,
    pub sector: String,
    pub industry: String,
    pub calculation: AssetCalculationModel,
    pub execution: AssetExecutionModel,
    pub risk: AssetRiskModel,
    pub pnl: AssetPnLModel,
    pub sessions: Vec<TradingSession>,
}

#[derive(Debug, Serialize, Deserialize,SurrealValue, Clone)]
pub struct AssetCalculationModel {
    pub digits: i32,
    pub stops_level: i32,
    pub tick_value: f64,
    pub chart_mode: String,
}

#[derive(Debug, Serialize, Deserialize,SurrealValue, Clone)]
pub struct AssetExecutionModel {
    pub tick_size: f64,
    pub lot_size: f64,
    pub min_quantity: f64,
    pub max_quantity: f64,
    pub volume_step: f64,
    pub filling_modes: i32,
    pub order_modes: i32,
    pub expiration_modes: i32,
}

#[derive(Debug, Serialize, Deserialize,SurrealValue, Clone)]
pub struct AssetPnLModel {
    pub profit_currency: String,
    pub swap_long: f64,
    pub swap_short: f64,
    pub swap_rates: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize,SurrealValue, Clone)]
pub struct AssetRiskModel {
    pub initial_margin: f64,
    pub maintenance_margin: f64,
    pub max_leverage: f64,
    pub is_shortable: bool,
    pub margin_currency: String,
}

#[derive(Debug, Serialize, Deserialize,SurrealValue, Clone)]
pub struct TradingSession {
    pub day_index: i32,
    pub open: String,
    pub close: String,
}

#[derive(Deserialize, Clone, SurrealValue)]
pub struct StaticMetadata {
    pub timezone: String,
    pub exchange: String,
}