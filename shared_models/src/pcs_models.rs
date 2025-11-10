
// In shared_models/src/pcs_models.rs (New File)
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pcs1stOrderData {
    pub asset_id: String,
    pub ps_name: String,
    pub ps_bias_3: String, // 3-state bias
    pub cs_name: String,
    pub cs_bias_3: String, // 3-state bias
    pub daily_outcome_7: String,
    pub day_type_tier: String,
    pub lookback_period: String,
    pub p_day_type_conditional: f64,
    pub p_day_type_base: f64,
    pub pcs_score: f64,
}

impl Pcs1stOrderData {
    pub fn from_row(row: &Row) -> Self {
        Pcs1stOrderData {
            asset_id: row.get("asset_id"),
            ps_name: row.get("ps_name"),
            ps_bias_3: row.get("ps_bias_3"),
            cs_name: row.get("cs_name"),
            cs_bias_3: row.get("cs_bias_3"),
            daily_outcome_7: row.get("daily_outcome_7"),
            day_type_tier: row.get("day_type_tier"),
            lookback_period: row.get("lookback_period"),
            p_day_type_conditional: row.get("p_day_type_conditional"),
            p_day_type_base: row.get("p_day_type_base"),
            pcs_score: row.get("pcs_score"),
        }
    }
}