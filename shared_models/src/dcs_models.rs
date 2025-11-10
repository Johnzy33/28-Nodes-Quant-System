
// In shared_models/src/dcs_models.rs (New File)
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use chrono::NaiveDate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcsSnapshot {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub dcs: f64, // Numeric(10,4) maps well to f64 in Rust
    pub dcs_classification: String,
    pub f_reversal: f64,
    pub f_dm1_factor: f64,
    pub f_commitment: f64,
    pub f_sustainability: f64,
}

impl DcsSnapshot {
    pub fn from_row(row: &Row) -> Self {
        // Note: We convert NUMERIC(10,4) to f64 using the get::<_, f64>
        DcsSnapshot {
            trading_date: row.get("trading_date"),
            asset_id: row.get("asset_id"),
            dcs: row.get("dcs"),
            dcs_classification: row.get("dcs_classification"),
            f_reversal: row.get("f_reversal"),
            f_dm1_factor: row.get("f_dm1_factor"),
            f_commitment: row.get("f_commitment"),
            f_sustainability: row.get("f_sustainability"),
        }
    }
}