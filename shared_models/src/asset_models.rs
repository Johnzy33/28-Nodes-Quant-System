use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

/// Represents an active asset in the database, used for list selection (Query A2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub id: String,
    pub symbol: String,
    pub name: Option<String>,
}

impl AssetInfo {
    /// Helper to deserialize a database row into AssetInfo
    pub fn from_row(row: &Row) -> Self {
        AssetInfo {
            id: row.get("id"),
            symbol: row.get("symbol"),
            // Use try_get to safely handle a potentially NULL name column
            name: row.try_get("name").ok(),
        }
    }
}