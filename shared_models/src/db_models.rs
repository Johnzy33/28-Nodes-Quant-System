use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use surrealdb::engine::local::{Db, Mem, SurrealKv};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;


pub struct AppDatabases {
    pub mem: Surreal<Db>,
    pub disk: Surreal<Db>,
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



#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Asset {
    pub symbol: String,
    pub name: String,
    pub source: String,
    pub asset_type: String,
    pub sector: String,
    pub industry: String,
    pub calculation: AssetCalculationModel,
    pub execution: AssetExecutionModel,
    pub risk: AssetRiskModel,
    pub pnl: AssetPnLModel,
    pub sessions: Vec<TradingSession>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetCalculationModel {
    pub digits: i32,
    pub stops_level: i32,
    pub tick_value: f64,
    pub chart_mode: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetPnLModel {
    pub profit_currency: String,
    pub swap_long: f64,
    pub swap_short: f64,
    pub swap_rates: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetRiskModel {
    pub initial_margin: f64,
    pub maintenance_margin: f64,
    pub max_leverage: f64,
    pub is_shortable: bool,
    pub margin_currency: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradingSession {
    pub day_index: i32,
    pub open: String,
    pub close: String,
}