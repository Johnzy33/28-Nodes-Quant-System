

use sqlx::{PgPool, Result as SqlxResult};
use anyhow::{Result, Context, anyhow};

pub struct DataService {
pub pool: PgPool,
}

impl DataService {
    /// Constructs the DataService using the pre-initialized connection pool.
    pub fn new(pool: PgPool) -> Self {
    DataService { pool }
    }

   // Fetches the maximum timestamp (ts in milliseconds) for a given asset from market_data.
    pub async fn get_market_data_high_watermark(&self, asset_id: &str) -> Result<Option<i64>> {
        // We query the market_data table where the CSV records end up
        let query = r#"
            SELECT (EXTRACT(EPOCH FROM MAX(time)) * 1000)::BIGINT - 1800000
            FROM market_data
            WHERE asset_id = $1
        "#;
        
        // sqlx::query_scalar! macro simplifies fetching a single optional value
        let max_ts: SqlxResult<Option<i64>> = sqlx::query_scalar(query)
            .bind(asset_id)
            .fetch_one(&self.pool)
            .await;

        match max_ts {
            // Case 1: Query succeeded. The result is an Option<i64> (either a value or NULL)
            Ok(ts_option) => Ok(ts_option), 
            
            // Case 2: Query failed (e.g., connection error)
            Err(e) => Err(anyhow!("Failed to fetch market data high water mark: {}", e)),
        }
    }
}






















