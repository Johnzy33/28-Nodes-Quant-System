// shared_models/src/models.rs

use chrono::NaiveDate;
use sqlx::FromRow;
use serde::Serialize; 

// Matches the 'assets' table schema
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AssetInfo {
    pub id: String,
    pub symbol: String,
    pub timezone: String,
    pub source: String,
    pub name: Option<String>,
    pub asset_class: Option<String>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub active: Option<bool>,
}

// Matches the output of get_trading_signal and get_signal_for_backtest_pattern
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct FciSignalOutput {
    pub asset_id: Option<String>,
    pub trading_date: Option<NaiveDate>,
    pub ps2_name: Option<String>,
    pub ps1_name: Option<String>,
    pub ps2_bias: Option<String>,
    pub ps1_bias: Option<String>,
    pub cs_name: String,
    pub predicted_day_type: String,
    pub signal_direction: String,
    pub fci_score: f64,
    pub signal_confidence: String,
    pub is_vetoed: bool,
    pub pcs_anchor_score: f64,
    pub tcs_multiplier: f64,
    pub p_continuation_raw: Option<f64>,
}

// Matches the output of the daily_composite_score table/query
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DcsScore {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub dcs: Option<f64>,
    pub dcs_classification: Option<String>,
    pub f_reversal: Option<f64>,
    pub f_dm1_factor: Option<f64>,
    pub f_commitment: Option<f64>,
    pub f_sustainability: Option<f64>,
}
