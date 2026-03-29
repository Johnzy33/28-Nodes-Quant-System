
use log::{info, error, warn};
use async_trait::async_trait;
use surrealdb_types::SurrealValue; 
use crate::traits::{DataServiceBase, DataIngestionExt};
use tokio::time::Duration;
use shared_models::data_model::DataService;
use tokio::net::{TcpListener};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use std::sync::Arc;
use crate::watchdog::WatchdogState;
use shared_models::db_models as db;
use surrealdb::Connection;
use surrealdb::types::{RecordId};


#[async_trait]
impl DataServiceBase for DataService {

    async fn get_hwm<T, C: Connection>(
        &self,
        dbs: &surrealdb::Surreal<C>,
        table: &str,
        asset_id: &RecordId,
    ) -> surrealdb::Result<Option<T>>
    where
        T: serde::de::DeserializeOwned+ Send + Sync + SurrealValue,
    {
      
    let query = format!(
        "SELECT asset_id, time AS last_time FROM {} WHERE asset_id = $asset_id ORDER BY asset_id, time DESC LIMIT 1",
        table
    );


        let mut result = dbs
            .query(query)
            .bind(("asset_id", asset_id.clone())) // Bind the inner ID key
            .await?;

        // We can take the value directly from the result without defining an extra struct
        // SurrealDB will try to map the 'last_time' field directly to Option<T>
        let last_time: Option<T> = result.take("last_time")?;
        
        Ok(last_time)
    }

    async fn run(
        &self, 
        addr: &str, 
        dbs: Arc<db::AppDatabases>,
        data_service: Arc<DataService>,
        state: WatchdogState
    ) -> db::AppResult<()> {
        let listener = TcpListener::bind(addr).await?;
        let connection_limit = Arc::new(Semaphore::new(10));
        let shutdown_token = CancellationToken::new();
        let mut worker_handles = Vec::new();

        info!("Ingestion Engine Online: {}", addr);

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Ctrl+C detected. Signaling workers to stop...");
                    shutdown_token.cancel();
                    break;
                }
                accept_res = listener.accept() => {
                    let (socket, _) = accept_res?;
                    let token = shutdown_token.clone();
                    let permit = Arc::clone(&connection_limit).acquire_owned().await?;
                    
                    let service_handle = Arc::clone(&data_service); 
                    let state_for_worker = state.clone();
                    let dbs_for_worker = dbs.clone();

                    let handle = tokio::spawn(async move {
                        if let Err(e) = service_handle.broker_ingestion(socket, dbs_for_worker, token, state_for_worker).await {
                            error!("Ingestion worker error: {}", e);
                        }
                        drop(permit);
                    });
                    worker_handles.push(handle);
                }
            }
        }

        // --- GRACEFUL SHUTDOWN WITH TIMEOUT ---
        info!("Waiting for {} workers to finish flushing (Timeout: 5s)...", worker_handles.len());

        // Wrap the joining process in a timeout
        let join_all = futures::future::join_all(worker_handles);
        
        match tokio::time::timeout(Duration::from_secs(5), join_all).await {
            Ok(_) => info!("All workers exited gracefully."),
            Err(_) => warn!("Shutdown timed out! Forcing exit to prevent process hang."),
        }

        Ok(())
    }


 

}

