
// data_service.rs

// use crate::traits;
// use sqlx::{PgPool, Result as SqlxResult};
// use anyhow::{Result, anyhow};
// use log::info;
// use shared_models::data_model as dm;
// use shared_models::data_model::DataService;

use anyhow::{Result, anyhow, };

use log::{info, error};
use sqlx::{PgPool, Result as SqlxResult};

// 1. You must import the attribute macro explicitly
use async_trait::async_trait; 

// 2. You must import the trait you defined in traits.rs

use crate::traits::{DataServiceBase, DataPersistExt, DataViewExt, DataIngestionExt};

// 3. Foundation models
use shared_models::data_model::DataService;
use shared_models::data_model as dm;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use std::sync::Arc;
use std::error::Error;
use crate::producer_config::{IngestionCoordinatorConfig,AssetIngestJob};
use crate::watchdog::WatchdogState;
use shared_models::db_models as db;
//use tracing::{info, error};

#[async_trait]
impl DataServiceBase for DataService {

    async fn run(
        &self, 
        addr: &str, 
        data_service: Arc<DataService>, // This is the Arc holding 'self'
        state: WatchdogState
      //  config: IngestionCoordinatorConfig
    ) -> Result<(), Box<dyn Error>> {
        
        let listener = TcpListener::bind(addr).await?;
        let connection_limit = Arc::new(Semaphore::new(10));
        let shutdown_token = CancellationToken::new();
        let state_for_worker = state.clone();
      //  let shared_config = Arc::new(config);

        info!("🚀 Ingestion Engine Online: {}", addr);

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    shutdown_token.cancel();
                    break;
                }
                accept_res = listener.accept() => {
                    let (socket, _) = accept_res?;
                    
                    // Clones for the move
                   // let cfg = Arc::clone(&shared_config);
                    let token = shutdown_token.clone();
                    let permit = Arc::clone(&connection_limit).acquire_owned().await?;
                    
                    // FIXED: Clone the Arc handle to the service
                    let service_handle = Arc::clone(&data_service); 
                    let state_for_worker = state.clone();

                    tokio::spawn(async move {
                        // Now we have all 4 parameters: 
                        // 1. service_handle (self), 2. socket, 3. state_for_worker, 4. token
                        if let Err(e) = service_handle.handle_mt5_ingestion(socket, token, state_for_worker).await {
                            error!("Ingestion worker error: {}", e);
                        }
                        drop(permit);
                    });
                }
            }
        }
        Ok(())
    }

    async fn run_new(
            &self, 
            addr: &str, 
            dbs: Arc<db::AppDatabases>,
            data_service: Arc<DataService>, // This is the Arc holding 'self'
            state: WatchdogState
        //  config: IngestionCoordinatorConfig
        ) -> Result<(), Box<dyn Error>> {
            
            let listener = TcpListener::bind(addr).await?;
            let connection_limit = Arc::new(Semaphore::new(10));
            let shutdown_token = CancellationToken::new();
            
        //  let shared_config = Arc::new(config);

            info!("🚀 Ingestion Engine Online: {}", addr);

            loop {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        shutdown_token.cancel();
                        break;
                    }
                    accept_res = listener.accept() => {
                        let (socket, _) = accept_res?;
                        
                        // Clones for the move
                    // let cfg = Arc::clone(&shared_config);
                        let token = shutdown_token.clone();
                        let permit = Arc::clone(&connection_limit).acquire_owned().await?;
                        
                        // FIXED: Clone the Arc handle to the service
                        let service_handle = Arc::clone(&data_service); 
                        let state_for_worker = state.clone();
                        let dbs_for_worker = dbs.clone();

                        tokio::spawn(async move {
                            // Now we have all 4 parameters: 
                            // 1. service_handle (self), 2. socket, 3. dbs_for_worker, 4. token, 5. state_for_worker
                            if let Err(e) = service_handle.handle_mt5_ingestion_new(socket, dbs_for_worker, token, state_for_worker).await {
                                error!("Ingestion worker error: {}", e);
                            }
                            drop(permit);
                        });
                    }
                }
            }
            Ok(())
        }


    async fn get_market_data_high_watermark(&self, asset_id: &str) -> Result<Option<i64>> {
        // 1. Remove * 1000. MT5 needs SECONDS.
        // 2. Subtract 3600 (1 hour in seconds) to ensure we don't miss the partial current bar.
        let query = r#"
            SELECT (EXTRACT(EPOCH FROM MAX(time)))::BIGINT
            FROM market_data
            WHERE asset_id = $1
        "#;
        
        let max_ts: SqlxResult<Option<i64>> = sqlx::query_scalar(query)
            .bind(asset_id)
            .fetch_one(&self.pool)
            .await;

        match max_ts {
            Ok(ts_option) => Ok(ts_option), 
            Err(e) => Err(anyhow!("Failed to fetch HWM: {}", e)),
        }
    }

    async fn fetch_all_active_asset_ids(&self) -> Result<Vec<String>> {
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
    // pub async fn refresh_session_base_mv(&self) -> Result<()> {
    //     let query = "REFRESH MATERIALIZED VIEW CONCURRENTLY session_base";
    //     sqlx::query(query)
    //         .execute(&self.pool)
    //         .await
    //         .map(|_| ()) // Map the result to () for simple success return
    //         .map_err(|e| anyhow!("Failed to refresh session_base MV: {}", e))?;
        
    //     Ok(())
    // }

    // /// Executes REFRESH MATERIALIZED VIEW on daily_base.
    // pub async fn refresh_daily_base_mv(&self) -> Result<()> {
    //     let query = "REFRESH MATERIALIZED VIEW CONCURRENTLY daily_base";
    //     sqlx::query(query)
    //         .execute(&self.pool)
    //         .await
    //         .map(|_| ())
    //         .map_err(|e| anyhow!("Failed to refresh daily_base MV: {}", e))?;
        
    //     Ok(())
    // }

    // pub async fn refresh_bar_base_mv(&self) -> Result<()> {
    //     let query = "REFRESH MATERIALIZED VIEW CONCURRENTLY eight_hr_base";
    //     sqlx::query(query)
    //         .execute(&self.pool)
    //         .await
    //         .map(|_| ())
    //         .map_err(|e| anyhow!("Failed to refresh eight_hr_base MV: {}", e))?;
        
    //     Ok(())
    // }

    async fn refresh_session_base_mv(&self) -> Result<()> {
        // Removed CONCURRENTLY for raw speed
        let query = "REFRESH MATERIALIZED VIEW session_base";
        sqlx::query(query)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| anyhow!("Failed to refresh session_base MV: {}", e))
    }

    async fn refresh_daily_base_mv(&self) -> Result<()> {
        let query = "REFRESH MATERIALIZED VIEW daily_base";
        sqlx::query(query)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| anyhow!("Failed to refresh daily_base MV: {}", e))
    }

    async fn refresh_bar_base_mv(&self) -> Result<()> {
        let query = "REFRESH MATERIALIZED VIEW eight_hr_base";
        sqlx::query(query)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| anyhow!("Failed to refresh eight_hr_base MV: {}", e))
    }

    async fn sync_asset_data(&self, asset_id: &str) -> Result<()> {

        info!("============================================================");
        info!(" Starting MASTER ETL for asset: {}", asset_id);
        info!("============================================================");
        // --L1: Process Sessions
        let (sessions, s_contexts) = self.calculate_session_and_context(asset_id, Some(180)).await?;
        self.persist_data(&sessions).await?;
        self.persist_data(&s_contexts).await?;

        // --L1.5: Process 8hr Blocks
        let (blocks, b_contexts) = self.calculate_8hr_context_and_blocks(asset_id).await?;
        self.persist_data(&blocks).await?;
        self.persist_data(&b_contexts).await?;

     // --- L2: Process Daily ---
        let daily = self.daily_views(asset_id, None).await?;
        let has_new_data = !daily.is_empty();
        self.persist_data(&daily).await?;

        // If no new daily bars exist, nothing above can change. Exit early.
        if !has_new_data {
            return Ok(());
        }

        // --- L3: Weekly (Always update if daily changed) ---
        let weekly = self.weekly_views(asset_id, None).await?;
        self.persist_data(&weekly).await?;

        // --- L4 & L5: Monthly/Yearly "Live Anchor" Logic ---
        // Instead of gating the WHOLE function, we let the internal 
        // HWM logic of monthly_views/yearly_views handle it.
        
        // To optimize, we can pass a 'limit_to_current' flag or 
        // simply rely on the fact that your HWM should only pull 
        // the current month if the previous ones are already in daily_views.
        let monthly = self.monthly_views(asset_id, None).await?;
        self.persist_data(&monthly).await?;

        let yearly = self.yearly_views(asset_id, None).await?;
        self.persist_data(&yearly).await?;

        info!("============================================================");
        info!("MASTER ETL completed successfully for asset: {}", asset_id);
        info!("============================================================");
        
        Ok(())
    }

}

