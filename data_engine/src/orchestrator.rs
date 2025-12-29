

use anyhow::{anyhow, Result};
use crate::data_service::DataService;


use std::sync::Arc;
use tokio::sync::Semaphore;
use futures::stream::{self, StreamExt};
use log::{info, error, warn};

impl DataService {
    /// The entry point for the entire system refresh.
    pub async fn assets_master_etl(&self) -> Result<()> {
        info!("==================================================");
        info!("          STARTING MASTER CORE DATA REFRESH       ");
        info!("==================================================");

        // --- PHASE 1: GLOBAL REFRESH (Sequential) ---
        // These MVs must be fresh before any asset-specific ETL starts.
        info!("Phase 1: Refreshing Materialized Views...");
        self.refresh_session_base_mv().await?;
        self.refresh_daily_base_mv().await?;
        self.refresh_bar_base_mv().await?;

        // --- PHASE 2: MULTI-ASSET ETL (Parallel with Concurrency Limit) ---
        let asset_ids = self.fetch_all_active_asset_ids().await?;
        let total_assets = asset_ids.len();

        if total_assets == 0 {
            warn!("No active assets found to process.");
            return Ok(());
        }

        // Limit concurrency to avoid pool exhaustion. 
        // 5-10 is usually a sweet spot for database-heavy tasks.
        let concurrency_limit = 8; 
        let semaphore = Arc::new(Semaphore::new(concurrency_limit));
        let service = Arc::new(self.clone()); // Assuming DataService is clonable (contains Arc<Pool>)

        info!("Starting Parallel ETL for {} assets (Concurrency: {})", total_assets, concurrency_limit);

        // Convert Vec to Stream for controlled parallel processing
        let results = stream::iter(asset_ids)
            .map(|asset_id| {
                let sem = Arc::clone(&semaphore);
                let svc = Arc::clone(&service);
                
                async move {
                    // Acquire permit from semaphore
                    let _permit = sem.acquire().await.expect("Semaphore closed");
                    
                    // Execute the pipeline we defined in DataService
                    let res = svc.sync_asset_data(&asset_id).await;
                    (asset_id, res)
                }
            })
            .buffer_unordered(concurrency_limit) // This is the "magic" that runs them in parallel
            .collect::<Vec<(String, Result<()>)>>()
            .await;

        // --- PHASE 3: AGGREGATE RESULTS ---
        let mut success_count = 0;
        let mut errors = Vec::new();

        for (id, res) in results {
            match res {
                Ok(_) => {
                    success_count += 1;
                    info!("ETL Success: {}", id);
                }
                Err(e) => {
                    error!("ETL Failure [{}]: {:?}", id, e);
                    errors.push(format!("{}: {}", id, e));
                }
            }
        }

        self.print_summary(total_assets, success_count, &errors);

        if !errors.is_empty() {
            return Err(anyhow!("{} assets failed during ETL", errors.len()));
        }

        Ok(())
    }

    fn print_summary(&self, total: usize, success: usize, errors: &[String]) {
        info!("==================================================");
        info!("         MASTER ORCHESTRATION SUMMARY           ");
        info!("Total Assets:      {}", total);
        info!("Successful:        {}", success);
        info!("Failed:            {}", errors.len());
        if !errors.is_empty() {
            info!("First error:       {}", errors[0]);
        }
        info!("==================================================");
    }
}