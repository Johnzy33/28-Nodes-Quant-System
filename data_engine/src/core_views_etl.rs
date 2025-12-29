use anyhow::{anyhow, Result};
use sqlx::PgPool;
use log::info;
use crate::data_service::DataService; // Adjust crate path as necessary



// // ====================================================================
// // L1: Session Views (Original, kept for modularity)``
// // ====================================================================

// /// Runs the complete ETL pipeline for classifying and persisting session data (L1).
// pub async fn run_session_etl(pool: PgPool, asset_id: &str) -> Result<()> {

//     let data_service = DataService { pool };
//     info!("Starting L1/L1.5 Combined ETL (Session and Context) on asset: {}", asset_id);

//     // 1. Dual Calculation (L1 Classification and L1.5 Sequential Context)
//     let (classified_sessions, session_contexts) = data_service
//         .calculate_session_and_context(asset_id)
//         .await
//         .map_err(|e| anyhow!("L1/L1.5 Calculation failed: {}", e))?;
    
//     if classified_sessions.is_empty() {
//         info!("No sessions processed for {}. Skipping persistence.", asset_id);
//         return Ok(());
//     }
    
//     // 2. Persistence L1 (session_views)
//     data_service
//         .persist_classified_sessions(&classified_sessions)
//         .await
//         .map_err(|e| anyhow!("L1 Persistence (session_views) failed: {}", e))?;

//     // 3. Persistence L1.5 (session_context)
//     data_service
//         .persist_session_context(&session_contexts)
//         .await
//         .map_err(|e| anyhow!("L1.5 Persistence (session_context) failed: {}", e))?;

//     info!("L1/L1.5 ETL completed. {} sessions and {} contexts processed.", 
//           classified_sessions.len(), 
//           session_contexts.len()
//     );
//     Ok(())
// }

// pub async fn run_8hr_block_etl(pool: PgPool, asset_id: &str) -> Result<()> {

//     let data_service = DataService { pool };
//     info!("Starting 8-Hour Block ETL on asset: {}", asset_id);

//     // 1. Calculation
//     let (classified_8hr_blocks,block_contexts) = data_service
//         .calculate_8hr_context_and_blocks(asset_id)
//         .await
//         .map_err(|e| anyhow!("8-Hour Block Calculation failed: {}", e))?;

//     if classified_8hr_blocks.is_empty() {
//         info!("No 8-Hour blocks processed for {}. Skipping persistence.", asset_id);
//         return Ok(());
//     }

//     // 2. Persistence
//     data_service
//         .persist_8hr_blocks(&classified_8hr_blocks)
//         .await
//         .map_err(|e| anyhow!("8-Hour Block Persistence failed: {}", e))?;

//     info!("8-Hour Block ETL completed. {} blocks processed.", classified_8hr_blocks.len());
    

//     data_service
//         .persist_block_context(&block_contexts)
//         .await
//         .map_err(|e| anyhow!("8-Hour Block Persistence failed: {}", e))?;

//     info!("8-Hour Block ETL completed. {} blocks processed.", block_contexts.len());
//     Ok(())
// }

// // ====================================================================
// // L2: Daily Views
// // ====================================================================

// /// Runs the complete ETL pipeline for generating and persisting daily aggregated views (L2).
// pub async fn run_daily_view_etl(pool: PgPool, asset_id: &str) -> Result<()> {
    
//     let data_service = DataService { pool };
//     info!("Starting L2 ETL (Daily Views) on asset: {}", asset_id);

//     // 1. Calculation
//     let daily_views = data_service
//         .calculate_classified_daily_views(asset_id)
//         .await
//         .map_err(|e| anyhow!("L2 Calculation failed: {}", e))?;

//     // 2. Persistence
//     data_service
//         .persist_classified_daily_views(&daily_views)
//         .await
//         .map_err(|e| anyhow!("L2 Persistence failed: {}", e))?;

//     info!("L2 ETL (Daily Views) completed. {} views processed.", daily_views.len());
//     Ok(())
// }

// // ====================================================================
// // L3: Weekly Views
// // ====================================================================

// /// Runs the complete ETL pipeline for generating and persisting weekly aggregated views (L3).
// pub async fn run_weekly_view_etl(pool: PgPool, asset_id: &str) -> Result<()> {
    
//     let data_service = DataService { pool };
//     info!("Starting L3 ETL (Weekly Views) on asset: {}", asset_id);
    
//     // 1. Calculation
//     let weekly_views = data_service
//         .calculate_classified_weekly_views(asset_id)
//         .await
//         .map_err(|e| anyhow!("L3 Calculation failed: {}", e))?;

//     // 2. Persistence
//     data_service
//         .persist_classified_weekly_views(&weekly_views)
//         .await
//         .map_err(|e| anyhow!("L3 Persistence failed: {}", e))?;

//     info!("L3 ETL (Weekly Views) completed. {} views processed.", weekly_views.len());
//     Ok(())
// }

// // ====================================================================
// // L4: Monthly Views
// // ====================================================================

// /// Runs the complete ETL pipeline for generating and persisting monthly aggregated views (L4).
// pub async fn run_monthly_view_etl(pool: PgPool, asset_id: &str) -> Result<()> {
    
//     let data_service = DataService { pool };
//     info!("Starting L4 ETL (Monthly Views) on asset: {}", asset_id);

//     // 1. Calculation
//     let monthly_views = data_service
//         .calculate_classified_monthly_views(asset_id)
//         .await
//         .map_err(|e| anyhow!("L4 Calculation failed: {}", e))?;

//     // 2. Persistence
//     data_service
//         .persist_classified_monthly_views(&monthly_views)
//         .await
//         .map_err(|e| anyhow!("L4 Persistence failed: {}", e))?;

//     info!("L4 ETL (Monthly Views) completed. {} views processed.", monthly_views.len());
//     Ok(())
// }

// // ====================================================================
// // L5: Yearly Views
// // ====================================================================

// /// Runs the complete ETL pipeline for generating and persisting yearly aggregated views (L5).
// pub async fn run_yearly_view_etl(pool: PgPool, asset_id: &str) -> Result<()> {
    
//     let data_service = DataService { pool };
//     info!("Starting L5 ETL (Yearly Views) on asset: {}", asset_id);

//     // 1. Calculation
//     let yearly_views = data_service
//         .calculate_classified_yearly_views(asset_id)
//         .await
//         .map_err(|e| anyhow!("L5 Calculation failed: {}", e))?;

//     // 2. Persistence
//     data_service
//         .persist_classified_yearly_views(&yearly_views)
//         .await
//         .map_err(|e| anyhow!("L5 Persistence failed: {}", e))?;

//     info!("L5 ETL (Yearly Views) completed. {} views processed.", yearly_views.len());
//     Ok(())
// }

// impl DataService {

//     pub async fn sync_asset_data(&self, asset_id: &str) -> Result<()> {

//         info!("============================================================");
//         info!(" Starting MASTER ETL for asset: {}", asset_id);
//         info!("============================================================");
//         // 1. Process Sessions
//         let (sessions, s_contexts) = self.calculate_session_and_context(asset_id).await?;
//         self.persist_batch(&sessions).await?;
//         self.persist_batch(&s_contexts).await?;

//         // 2. Process 8hr Blocks
//         let (blocks, b_contexts) = self.calculate_8hr_context_and_blocks(asset_id).await?;
//         self.persist_batch(&blocks).await?;
//         self.persist_batch(&b_contexts).await?;

//         // 3. Process Daily
//         let daily = self.calculate_classified_daily_views(asset_id).await?;
//         self.persist_batch(&daily).await?;

//         let weekly = self.calculate_classified_weekly_views(asset_id).await?;
//         self.persist_batch(&weekly).await?;

//         let monthly = self.calculate_classified_monthly_views(asset_id).await?;
//         self.persist_batch(&monthly).await?;

//         let yearly = self.calculate_classified_yearly_views(asset_id).await?;
//         self.persist_batch(&yearly).await?; 

//         info!("============================================================");
//         info!("MASTER ETL completed successfully for asset: {}", asset_id);
//         info!("============================================================");

//         Ok(())
//     }
// }