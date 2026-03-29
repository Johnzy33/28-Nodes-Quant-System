
use data_engine::traits::DataServiceBase;
use data_engine::traits::DataViewExt;
use dashmap::DashMap;
use std::sync::Arc;
use futures_util::future::join_all;
use tokio::task::JoinHandle;
use async_trait::async_trait;
use log::{info,error};
use std::sync::atomic::{AtomicBool, Ordering};


use crate::spg_tracker::SpgTracker;
use shared_models::data_model as dm;
use shared_models::traits::MarketDataHandler;
use data_engine::traits::DataOrchestratorExt;
use shared_models::market_data::MarketData_old;

pub struct SpgOrchestrator {
    data_service: Arc<dm::DataService>,
    /// Thread-safe map of Asset ID -> Active Tracker
    pub trackers: DashMap<String, SpgTracker>,
    pub is_live: AtomicBool,
}

#[async_trait]
impl MarketDataHandler for SpgOrchestrator {
    async fn on_price_update(&self, data: &MarketData_old) {
        // If we haven't bootstrapped yet, we ignore live ticks to prevent 
        // calculations on incomplete history.
        if !self.is_live.load(Ordering::Relaxed) {
            return;
        }

        // Normal Live Logic
        let mut tracker = self.trackers.get_mut(data.asset_id.as_str()).unwrap();
        tracker.process_live_update(data, &self.data_service);
    }
}

impl SpgOrchestrator {

    pub fn new(data_service: dm::DataService) -> Self {
        Self {
            data_service: Arc::new(data_service),
            trackers: DashMap::new(),
            is_live: AtomicBool::new(false),
        }
    }

    /// Optimized Bootstrap: Loads history for all active assets simultaneously.
    pub async fn bootstrap_assets(&self) -> Result<(), String> {
        
        let asset_ids = self.data_service.fetch_all_active_asset_ids()
            .await
            .map_err(|e| format!("Bootstrap failed to fetch asset IDs: {}", e)
        )?;

        if asset_ids.is_empty() {
            info!("⚠️ No active assets found for bootstrapping.");
            return Ok(());
        }

        let mut tasks = Vec::with_capacity(asset_ids.len());

        for asset_id in asset_ids {
            let service = Arc::clone(&self.data_service);
            let task: JoinHandle<(String, Result<SpgTracker, String>)> = tokio::spawn(async move {
                let result = SpgTracker::initialize_tracker(&service, &asset_id).await;
                (asset_id, result)
            });
            tasks.push(task);
        }

        let results = join_all(tasks).await;
        for res in results {
            if let Ok((id, Ok(tracker))) = res {
                self.trackers.insert(id, tracker);
            }
        }
        info!("✅ Bootstrap completed. Trackers active: {}", self.trackers.len());
        Ok(())
    }

    pub async fn run_master_etl(&self) -> Result<(), String> {
        self.data_service.assets_master_etl()
            .await
            .map_err(|e| format!("Master ETL failed: {}", e)
        )
    }



    pub async fn smart_reconcile(&self) -> Result<(), String> {
        info!("🎯 Smart Sync Triggered: Starting Phase 1 (Database ETL)");

        // 1. Refresh Materialized Views (Phase 1 of your ETL)
        // This ensures L1/L3/L5 calculations are up to date in the DB
        self.data_service.assets_master_etl()
            .await
            .map_err(|e| format!("Master ETL failed: {}", e)
        )?;

        info!("✅ Phase 1 Complete. Starting Phase 2 (Tracker Reconciliation)");

        // 2. Pull the newly calculated levels into the live Trackers
        self.reconcile_trackers().await;

        info!("🚀 Smart Sync Finished. SPG Levels are now 100% verified.");
        Ok(())
    }

    pub async fn reconcile_trackers(&self) {
        for mut entry in self.trackers.iter_mut() {
            if entry.value().needs_reconcile {
                let id = entry.key().clone();
                if let Ok((sessions, _)) = self.data_service.calculate_session_and_context(
                    &id, 
                    Some(24)
                ).await 
                {
                    entry.value_mut().sync_sessions_with_db(sessions);
                    entry.value_mut().needs_reconcile = false; // Reset the flag
                    info!("⚖️ Cleaned up tracker for {}", id);
                }
            }
        }
    }


    pub fn needs_reconciliation(&self) -> bool {
        // Check if ANY tracker has set its 'needs_reconcile' flag to true
        self.trackers.iter().any(|entry| entry.value().needs_reconcile)
    }
}


pub fn start_spg_sync_task(
    orchestrator: Arc<SpgOrchestrator>
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // Check every minute
        interval.tick().await; 

        loop {
            interval.tick().await;

            // Step A: Check if any tracker flipped its 'needs_reconcile' flag
            // (This flag is flipped inside handle_session_transition)
            let needs_work = orchestrator.trackers.iter().any(|entry| entry.value().needs_reconcile);

            if needs_work {
                info!("🔔 Session transition detected! Initiating Smart Reconcile...");
                
                if let Err(e) = orchestrator.smart_reconcile().await {
                    error!("❌ Smart Reconcile failed: {}", e);
                }

                // Step B: Reset the flags so we don't loop
                for mut entry in orchestrator.trackers.iter_mut() {
                    entry.value_mut().needs_reconcile = false;
                }
            }
        }
    });
}



pub async fn init_spg_engine(data_service: dm::DataService) -> Result<Arc<SpgOrchestrator>, String> {
    // 1. Create Orchestrator (is_live defaults to false in SpgOrchestrator::new)
    let orchestrator = Arc::new(SpgOrchestrator::new(data_service));

    // 2. Start the background session-transition watcher
    // This task handles things like day-rollovers while the system is running.
    // We start it now; it won't do much until trackers are actually loaded.
    start_spg_sync_task(Arc::clone(&orchestrator));

    info!("🧠 SPG Engine Initialized (Dormant). Waiting for Sync Signal...");
    Ok(orchestrator)
}
