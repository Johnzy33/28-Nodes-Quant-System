
use sqlx::PgPool;
use anyhow::{anyhow, Result};
use log::{info, error};
use tokio::task::JoinHandle;
use futures_util::future::join_all;
use crate::data_access::DataService;
use crate::core_views_etl::{run_session_etl, run_8hr_block_etl, run_daily_view_etl, run_weekly_view_etl, run_monthly_view_etl, run_yearly_view_etl};




/// Runs the complete, hierarchical ETL pipeline (L1 -> L5) for a single asset.
pub async fn run_master_asset_etl(pool: &PgPool, asset_id: &str) -> Result<()> {
    
    info!("============================================================");
    info!("🚀 Starting MASTER ETL for asset: {}", asset_id);
    info!("============================================================");

    // L1: Session Views (Foundation)
    run_session_etl(pool.clone(), asset_id).await?;

    // L1.5: 8-Hour Blocks (Depends on Session data)
    run_8hr_block_etl(pool.clone(), asset_id).await?;

    // L2: Daily Views (Depends on L1 data)
    run_daily_view_etl(pool.clone(), asset_id).await?;
    
    // L3: Weekly Views (Depends on L2 data)
    run_weekly_view_etl(pool.clone(), asset_id).await?;
    
    // L4: Monthly Views (Depends on L2 data)
    run_monthly_view_etl(pool.clone(), asset_id).await?;
    
    // L5: Yearly Views (Depends on L4 data)
    run_yearly_view_etl(pool.clone(), asset_id).await?;
    
    info!("============================================================");
    info!("✅ MASTER ETL completed successfully for asset: {}", asset_id);
    info!("============================================================");
    
    Ok(())
}

/// Orchestrates the execution of the full ETL pipeline (L1-L5) for ALL active assets, running them in parallel.
pub async fn run_all_assets_master_etl(pool: &PgPool) -> Result<()> {
    
    let data_service = DataService { pool: pool.clone() };

    info!("==================================================");
    info!("          STARTING MASTER CORE DATA REFRESH         ");
    info!("==================================================");

    // --- PHASE 1: REFRESH CORE BASE VIEWS (Global Aggregation) ---
    
    // 1. Refresh Session Base MV (Relies on market_data)
    info!("1.1: Refreshing session_base MV...");
    data_service.refresh_session_base_mv().await?;

    info!("1.2: Refreshing daily_base MV...");
    data_service.refresh_daily_base_mv().await?;

    
    info!("==================================================");
    info!(" Starting Multi-Asset Master ETL Orchestration.   ");
    info!("==================================================");



    // 1. Fetch the list of assets to process
    let asset_ids = data_service.fetch_all_active_asset_ids().await?;
    let total_assets = asset_ids.len();

    if total_assets == 0 {
        info!("No active assets found to process. Exiting.");
        return Ok(());
    }

    info!("Found {} active assets. Starting parallel ETL execution...", total_assets);

    // 2. Prepare the parallel tasks (futures)
    let mut tasks: Vec<JoinHandle<Result<()>>> = Vec::new();
    
    for asset_id in asset_ids {
        // Clone the pool and other required variables for the spawned task
        let pool_clone = pool.clone();
        let asset_id_clone = asset_id.clone();

        let task = tokio::spawn(async move {
            info!("Starting MASTER ETL for asset: {}", asset_id_clone);
            
            // Execute the sequential master pipeline for this single asset
            let result = run_master_asset_etl(&pool_clone, &asset_id_clone).await;
            
            match result {
                Ok(_) => info!(" ETL Success for asset: {}", asset_id_clone),
                Err(e) => {
                    error!(" ETL FAILED for asset {}: {:?}", asset_id_clone, e);
            
                    return Err(anyhow!("ETL failed for asset {}: {}", asset_id_clone, e));
                }
            }
            Ok(())
        });
        tasks.push(task);
    }

    //  Wait for all tasks to complete
    let results = join_all(tasks).await;

    //  Aggregate results and handle errors
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut errors = Vec::new();

    for res in results {
        match res {
            Ok(Ok(_)) => success_count += 1, // Task completed successfully
            Ok(Err(e)) => { // Task completed but returned an inner ETL error
                failure_count += 1;
                errors.push(format!("{}", e));
            },
            Err(join_err) => { // Task panicked or failed to join
                failure_count += 1;
                errors.push(format!("Task failed to join: {}", join_err));
            }
        }
    }

    info!("==================================================");
    info!(" Multi-Asset Orchestration Summary:     ");
    info!("Total Assets Processed: {}", total_assets);
    info!("Successful ETLs: {}", success_count);
    info!("Failed ETLs: {}", failure_count);
    info!("==================================================");


    if failure_count > 0 {
        return Err(anyhow!(
            "{} of {} assets failed the ETL. First few errors: \n{}",
            failure_count, total_assets, errors.join("\n")
        ));
    }

    Ok(())
}