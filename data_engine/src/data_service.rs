
use sqlx::{PgPool, Result as SqlxResult};
use anyhow::{Result, anyhow};
use log::info;

#[derive(Clone)]
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

    pub async fn fetch_all_active_asset_ids(&self) -> Result<Vec<String>> {
        let asset_ids = sqlx::query_scalar!(
            r#"
            SELECT id AS "asset_id!"
            FROM assets
            WHERE active = TRUE
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch active asset IDs: {}", e))?;

        Ok(asset_ids)
    }

    // --- 1. MV Refresh Tooling ---

    /// Executes REFRESH MATERIALIZED VIEW on session_base.
    pub async fn refresh_session_base_mv(&self) -> Result<()> {
        let query = "REFRESH MATERIALIZED VIEW CONCURRENTLY session_base";
        sqlx::query(query)
            .execute(&self.pool)
            .await
            .map(|_| ()) // Map the result to () for simple success return
            .map_err(|e| anyhow!("Failed to refresh session_base MV: {}", e))?;
        
        Ok(())
    }

    /// Executes REFRESH MATERIALIZED VIEW on daily_base.
    pub async fn refresh_daily_base_mv(&self) -> Result<()> {
        let query = "REFRESH MATERIALIZED VIEW CONCURRENTLY daily_base";
        sqlx::query(query)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| anyhow!("Failed to refresh daily_base MV: {}", e))?;
        
        Ok(())
    }

    pub async fn refresh_bar_base_mv(&self) -> Result<()> {
        let query = "REFRESH MATERIALIZED VIEW CONCURRENTLY eight_hr_base";
        sqlx::query(query)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| anyhow!("Failed to refresh eight_hr_base MV: {}", e))?;
        
        Ok(())
    }

    pub async fn sync_asset_data(&self, asset_id: &str) -> Result<()> {

        info!("============================================================");
        info!(" Starting MASTER ETL for asset: {}", asset_id);
        info!("============================================================");
        // --L1: Process Sessions
        let (sessions, s_contexts) = self.calculate_session_and_context(asset_id).await?;
        self.persist_data(&sessions).await?;
        self.persist_data(&s_contexts).await?;

        // --L1.5: Process 8hr Blocks
        let (blocks, b_contexts) = self.calculate_8hr_context_and_blocks(asset_id).await?;
        self.persist_data(&blocks).await?;
        self.persist_data(&b_contexts).await?;

        // --L2: Process Daily
        let daily = self.daily_views(asset_id).await?;
        self.persist_data(&daily).await?;

        // --L3: Process Weekly
        let weekly = self.weekly_views(asset_id).await?;
        self.persist_data(&weekly).await?;

        // --L4: Process Monthly
        let monthly = self.monthly_views(asset_id).await?;
        self.persist_data(&monthly).await?;

        // --L5: Process Yearly
        let yearly = self.yearly_views(asset_id).await?;
        self.persist_data(&yearly).await?; 

        info!("============================================================");
        info!("MASTER ETL completed successfully for asset: {}", asset_id);
        info!("============================================================");
        
        Ok(())
    }

}

