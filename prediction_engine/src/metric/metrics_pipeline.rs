use sqlx::PgPool;
use anyhow::{Result};
use log::{info, error};
use tokio::task::JoinHandle;
use data_engine::data_service::{ DataService};
use futures_util::future::{join_all};
use crate::metrics_service::MetricsService;

use crate::metric::metric_build;

pub async fn asset_metrics_pipeline(pool: &PgPool, asset_id: &str) -> Result<()> {
    
    let metric_service = MetricsService { pool: pool.clone() };

    //  Database Read 
    let session_contexts = metric_service.fetch_session_contexts(asset_id).await?;
    if session_contexts.is_empty() {
        info!("No session contexts found for {}. Exiting.", asset_id);
        return Ok(());
    }
    
    let bar_contexts = metric_service.fetch_bar_contexts(asset_id).await?;
    if bar_contexts.is_empty() {
        info!("No Eight Hours bar contexts found for {}. Exiting.", asset_id);
        return Ok(());
    }

    let daily_contexts = metric_service.fetch_day_contexts(asset_id).await?; 
    if daily_contexts.is_empty() {
        info!("No daily contexts found for {}. Exiting.", asset_id);
        return Ok(());
    }

    // In-Memory Calculations For Meterics Pipeline
    
    // Calculate CS Bias Base Rate (P_base)
    let session_base_rates = metric_build::cs_base_rates(
        &metric_service,
        &session_contexts,
        asset_id
    ).await?;

    let bar_base_rates = metric_build::bar_base_rates(
        &metric_service,
        &bar_contexts,
        asset_id
    ).await?;

    let daily_base_rates = metric_build::daily_base_rates(
        &metric_service,
        &daily_contexts,
        asset_id
    ).await?;

    let extreme_base = metric_build::session_extreme_base_rates(
        &metric_service, 
        &daily_contexts, 
        asset_id
    ).await?;


    let transition_2nd_order = metric_build::transition_2nd_order_rates(
        &metric_service,
        &session_contexts,
        asset_id
    ).await?;

    let tcs_2nd_order = metric_build::tcs_scores_2nd_order(
        &metric_service,
        transition_2nd_order.clone(),
        session_base_rates.clone(),
        asset_id
    ).await?;

    let bar_transition_2nd_order = metric_build::bar_transition_2nd_order_rates(
        &metric_service,
        &bar_contexts,
        asset_id
    ).await?;

    let stcs_2nd_order = metric_build::stcs_scores_2nd_order(
        &metric_service,
        bar_transition_2nd_order.clone(),
        bar_base_rates.clone(),
        asset_id
    ).await?;

    let bar_to_daily_outcome = metric_build::bar_to_daily_outcome_rates(
        &metric_service,
        &bar_contexts,
        &daily_contexts,
        asset_id
    ).await?;

    let bar_to_daily_lift = metric_build::bar_to_daily_lift_rates(
        &metric_service,
        bar_to_daily_outcome.clone(),
        daily_base_rates.clone(),
        asset_id
    ).await?;

    let session_to_daily_outcome = metric_build::session_to_daily_outcome_rates(
        &metric_service,
        &session_contexts,
        &daily_contexts,
        asset_id
    ).await?;

    let session_to_daily_lift = metric_build::session_to_daily_lift_rates(
        &metric_service,
        session_to_daily_outcome.clone(),
        daily_base_rates.clone(),
        asset_id
    ).await?;

    let session_to_bar_outcome = metric_build::session_to_bar_outcome_rates(
        &metric_service,
        &session_contexts,
        &bar_contexts,
        asset_id
    ).await?;

    // 2. Calculate the Lift (requires bar_base_rates calculated earlier in pipeline)
    let session_to_bar_lift = metric_build::session_to_bar_lift_rates(
        &metric_service,
        session_to_bar_outcome.clone(),
        bar_base_rates.clone(),
        asset_id
    ).await?;

    // 2. Calculate 3rd Order Outcomes (Using the Sliding Window)
    let daily_3rd_outcome = metric_build::daily_3rd_order_outcome_rates(
        &metric_service, 
        &daily_contexts, 
        asset_id
    ).await?;

    // 3. Calculate Lift (Comparing 3rd Order vs Base)
    let daily_3rd_lift = metric_build::daily_3rd_order_lift_rates(
        &metric_service,
        daily_3rd_outcome.clone(),
        daily_base_rates.clone(),
        asset_id
    ).await?;


    let session_continuation = metric_build::session_continuation_rates(
        &metric_service,
        &session_contexts,
        asset_id
    ).await?;

    // ... inside asset_metrics_pipeline(asset_id, ...) ...

    // 1. Bar Continuation (No sliding window needed, data is pre-joined)
    let bar_continuation = metric_build::bar_continuation_rates(
        &metric_service, 
        &bar_contexts, 
        asset_id
    ).await?;

    // 2. Daily Continuation (Sliding window handled inside the service function)
    let daily_continuation = metric_build::daily_continuation_rates(
        &metric_service, 
        &daily_contexts, 
        asset_id
    ).await?;

    let extreme_outcome = metric_build::session_extreme_outcome_rates(
    &metric_service, 
    &session_contexts, 
    &daily_contexts, 
    asset_id
).await?;

// 3. Calculate Lift (Edge/Strength)
let extreme_lift = metric_build::session_extreme_lift_rates(
    &metric_service,
    extreme_outcome.clone(),
    extreme_base.clone(),
    asset_id
).await?;

let (bar_base, bar_outcome, bar_lift) = metric_build::bar_extreme_suite(
    &metric_service, 
    &bar_contexts, // Pre-processed EightContextData
    &daily_contexts,    // Raw Daily data containing high_bar/low_bar
    asset_id
).await?;



    // Persist CS Base Rates ML
   metric_service.persist_cs_base_rates_ml(session_base_rates).await?;

    // Persist Bar Base Rates ML
    metric_service.persist_bar_base_rates_ml(bar_base_rates).await?;

    metric_service.persist_daily_base_rates_ml(daily_base_rates).await?;
    metric_service.persist_transition_2nd_order_ml(transition_2nd_order).await?;
    metric_service.persist_tcs_scores_ml(tcs_2nd_order).await?;
    metric_service.persist_bar_transition_2nd_order_ml(bar_transition_2nd_order).await?;
    metric_service.persist_stcs_scores_ml(stcs_2nd_order).await?;
    metric_service.persist_bar_to_daily_outcome_ml(bar_to_daily_outcome).await?;
    metric_service.persist_bar_to_daily_lift_ml(bar_to_daily_lift).await?;
    metric_service.persist_session_to_daily_outcome_ml(session_to_daily_outcome).await?;
    metric_service.persist_session_to_daily_lift_ml(session_to_daily_lift).await?;
    metric_service.persist_session_to_bar_outcome_ml(session_to_bar_outcome).await?;
    metric_service.persist_session_to_bar_lift_ml(session_to_bar_lift).await?;
    metric_service.persist_daily_3rd_order_outcome_ml(daily_3rd_outcome).await?;
    metric_service.persist_daily_3rd_order_lift_ml(daily_3rd_lift).await?;
    metric_service.persist_session_continuation_ml(session_continuation).await?;
    metric_service.persist_bar_continuation_ml(bar_continuation).await?;
    metric_service.persist_daily_continuation_ml(daily_continuation).await?;
    metric_service.persist_session_extreme_base_rate_ml(extreme_base).await?;
    metric_service.persist_session_extreme_outcome_ml(extreme_outcome).await?;
    metric_service.persist_session_extreme_lift_ml(
        extreme_lift
    ).await?;
    metric_service.persist_bar_extreme_base_rate_ml(bar_base).await?;
    metric_service.persist_bar_extreme_outcome_ml(bar_outcome).await?;
    metric_service.persist_bar_extreme_lift_ml(bar_lift).await?;


    Ok(())
}

pub async fn run_master_asset_metrics(pool: &PgPool) -> Result<()> {

    

    let data_service = DataService { pool: pool.clone() };

    // fetch the list of assets to process
    let asset_ids = data_service.fetch_all_active_asset_ids().await?;
    let total_assets = asset_ids.len();

    if total_assets == 0 {
        info!("No active assets found for metrics processing. Exiting.");
        return Ok(());
    }
    info!("Starting metrics processing for {} active assets.", total_assets);

    let mut tasks: Vec<JoinHandle<Result<()>>> = Vec::new();

    for asset_id in asset_ids {
        let pool_clone = pool.clone();
        let asset_id_clone = asset_id.clone();
        let task = tokio::spawn(async move {
            info!("Starting metrics pipeline for asset: {}", asset_id_clone);
            let result = asset_metrics_pipeline(&pool_clone, &asset_id_clone).await;
            match &result {
                Ok(_) => info!("Completed metrics pipeline for asset: {}", asset_id_clone),
                Err(e) => error!("Error in metrics pipeline for asset {}: {}", asset_id_clone, e),
            }
            Ok(())
        });
        tasks.push(task);
    }

   let mut result = join_all(tasks).await;

   let mut succes_count = 0;
   let mut fail_count = 0;
    for res in result.drain(..) {
         match res {
              Ok(Ok(_)) => succes_count += 1,
              Ok(Err(_)) | Err(_) => fail_count += 1,
         }
    }
    info!("Metrics processing completed. Total Assets {} Success: {}, Fail: {}", total_assets, succes_count, fail_count);
    Ok(())
}