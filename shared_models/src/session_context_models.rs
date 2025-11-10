// In shared_models/src/session_context_models.rs
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentSessionContext {
    pub cs_name: String,
    pub cs_bias_7: Option<String>,
    pub ps1_name: Option<String>,
    pub ps1_bias_7: Option<String>,
    pub ps2_name: Option<String>,
    pub ps2_bias_7: Option<String>,
}

impl CurrentSessionContext {
    pub fn from_row(row: &Row) -> Self {
        CurrentSessionContext {
            cs_name: row.get("cs_name"),
            cs_bias_7: row.try_get("cs_bias_7").ok(),
            ps1_name: row.try_get("ps1_name").ok(),
            ps1_bias_7: row.try_get("ps1_bias_7").ok(),
            ps2_name: row.try_get("ps2_name").ok(),
            ps2_bias_7: row.try_get("ps2_bias_7").ok(),
        }
    }
}