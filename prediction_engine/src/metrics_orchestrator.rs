

// use sqlx::PgPool;
// use anyhow::{Result, anyhow};
// use log::{info, error};
// use tokio::task::JoinHandle;
// use futures_util::future::{join_all};
// use data_engine::data_access::DataService;
// use crate::metrics_service::MetricsService;
// use crate::core_metrics::{calculate_and_return_cs_base_rates,calculate_and_return_tcs_scores, 
//     calculate_and_return_transition_2nd_order, calculate_and_return_day_base_rates, 
//     calculate_and_return_day_type_2nd_order, calculate_and_return_pcs_score, 
//     calculate_and_return_bar_day_type_2nd_order,calculate_and_return_bar_transition_2nd_order,
//     calculate_and_return_bpcs_score, calculate_and_return_btcs_scores,calculate_and_return_cb_base_rates,
//     calculate_and_return_tcs_continuation_score, calculate_and_return_exetreme_session_2nd_order_metric,
//     calculate_and_return_daily_trend_continuation_score, calculate_and_return_day_type_3rd_order, 
//     calculate_and_return_dcs_score, calculate_and_return_session_to_bar_2nd_order, calculate_and_return_stcs_scores,
// };


// pub async fn run_master_asset_metrics(pool: &PgPool, asset_id: &str) -> Result<()> {
    
//     let metric_service = MetricsService { pool: pool.clone() };

   
    
//     //  Database Read 
//     let session_contexts = metric_service.fetch_session_contexts_for_ml(asset_id).await?;
//     if session_contexts.is_empty() {
//         info!("No session contexts found for {}. Exiting.", asset_id);
//         return Ok(());
//     }
    
//     let bar_contexts = metric_service.fetch_bar_contexts_for_ml(asset_id).await?;
//     if bar_contexts.is_empty() {
//         info!("No Eight Hours bar contexts found for {}. Exiting.", asset_id);
//         return Ok(());
//     }

//     let daily_contexts = metric_service.fetch_day_contexts_for_ml(asset_id).await?; 
//     if daily_contexts.is_empty() {
//         info!("No daily contexts found for {}. Exiting.", asset_id);
//         return Ok(());
//     }

//     // In-Memory Calculations For Meterics Pipeline
    
//     // Calculate CS Bias Base Rate (P_base)
//     let cs_base_rates = calculate_and_return_cs_base_rates(
//         &metric_service, 
//         &session_contexts, 
//         asset_id
//     ).await?;

    

//     let cb_base_rates = calculate_and_return_cb_base_rates(
//         &metric_service, 
//         &bar_contexts, 
//         asset_id
//     ).await?;

//     // Calculate Daily Bias Base Rate (P_base)
//     let daily_base_rates = calculate_and_return_day_base_rates(     
//         &metric_service,
//         &daily_contexts,
//         asset_id
//     ).await?;

//     // Calculate 2nd Order Transition (P_conditional)
//     let transitions = calculate_and_return_transition_2nd_order(
//         &metric_service, 
//         &session_contexts, 
//         asset_id
//     ).await?;

//     let bar_transitions = calculate_and_return_bar_transition_2nd_order(
//         &metric_service, 
//         &bar_contexts, 
//         asset_id
//     ).await?;
    
//     // C. Calculate TCS Score (P_conditional / P_base) 
//     let tcs_scores = calculate_and_return_tcs_scores(
//         &metric_service,
//         transitions.clone(), 
//         cs_base_rates.clone(), 
//         asset_id
//     ).await?;

//      let btcs_scores = calculate_and_return_btcs_scores(
//         &metric_service,
//         bar_transitions.clone(), 
//         cb_base_rates.clone(), 
//         asset_id
//     ).await?;

//     //  Calculate Day Type 2nd Order (P_conditional for PCS)
//     let day_type_2nd_order = calculate_and_return_day_type_2nd_order(
//         &metric_service,
//         &session_contexts, 
//         &daily_contexts, 
//         asset_id
//     ).await?;

//      let bar_day_type_2nd_order = calculate_and_return_bar_day_type_2nd_order(
//         &metric_service,
//         &bar_contexts, 
//         &daily_contexts, 
//         asset_id
//     ).await?;

//     let sessione_to_bar_2nd_order = calculate_and_return_session_to_bar_2nd_order(
//         &metric_service,
//         &session_contexts,
//         &bar_contexts,
//         asset_id
//     ).await?;

//     let stcs_scores = calculate_and_return_stcs_scores(
//         &metric_service,
//         sessione_to_bar_2nd_order.clone(),
//         cb_base_rates.clone(),
//         asset_id
//     ).await?;

//     //  Calculate PCS Score (P_conditional / P_day_base)
//     let pcs_scores = calculate_and_return_pcs_score(
//         &metric_service,
//         day_type_2nd_order.clone(),
//         daily_base_rates.clone(),
//         asset_id
//     ).await?;

//     let bpcs_scores = calculate_and_return_bpcs_score(
//         &metric_service,
//         bar_day_type_2nd_order.clone(),
//         daily_base_rates.clone(),
//         asset_id
//     ).await?;


//     // Calculate 2nd Order TCS Continuation Score (P_conditional)
//     let tcs_continuation_score = calculate_and_return_tcs_continuation_score(
//         &metric_service, 
//         &session_contexts, 
//         asset_id
//     ).await?;

//     // Calculate 2nd Order Session Extremes (P_conditional)
//     let session_extrems = calculate_and_return_exetreme_session_2nd_order_metric(
//         &metric_service, 
//         &session_contexts, 
//         &daily_contexts, 
//         asset_id
//     ).await?;

//     // ---- Daily Metrics Begins here ----

//     let day_type_3rd_order = calculate_and_return_day_type_3rd_order(
//         &metric_service, 
//         &daily_contexts, 
//         asset_id
//     ).await?;

//     let dcs_scores = calculate_and_return_dcs_score(
//         &metric_service,
//         day_type_3rd_order.clone(),
//         daily_base_rates.clone(),
//         asset_id
//     ).await?;

//     let daily_trend_continuation_rates = calculate_and_return_daily_trend_continuation_score(
//         &metric_service, 
//         &daily_contexts, 
//         asset_id
//     ).await?;


//     // Database Write (Persistence) ---
//     info!("Starting batch persistence for all calculated session metrics...");
    

//     // Persist all calculated results
//     metric_service.persist_cs_base_rates(cs_base_rates).await?;
//     metric_service.persist_cb_base_rates(cb_base_rates).await?;
//     metric_service.persist_transition_2nd_order(transitions).await?;
//     metric_service.persist_bar_transition_2nd_order(bar_transitions).await?;
//     metric_service.persist_tcs_scores(tcs_scores).await?; 
//     metric_service.persist_btcs_scores(btcs_scores).await?; 
//     metric_service.persist_day_type_2nd_order(day_type_2nd_order).await?;
//     metric_service.persist_bar_day_type_2nd_order(bar_day_type_2nd_order).await?;
//     metric_service.persist_pcs_scores(pcs_scores).await?;
//     metric_service.persist_bpcs_scores(bpcs_scores).await?;
//     metric_service.persist_session_to_bar_metrics(sessione_to_bar_2nd_order).await?;
//     metric_service.persist_stcs_scores(stcs_scores).await?;
//     metric_service.persist_tcs_continuation_scores(tcs_continuation_score).await?;
//     metric_service.persist_high_low_session_2nd_order(session_extrems).await?; 
//     metric_service.persist_daily_3rd_order_conditional(day_type_3rd_order).await?;
//     metric_service.persist_dcs_scores(dcs_scores).await?;
//     metric_service.persist_daily_trend_continuation_rate(daily_trend_continuation_rates).await?;   

//     Ok(())
// }



// pub async fn run_all_assets_master_etl(pool: &PgPool) -> Result<()> {
    
//     let data_service = DataService { pool: pool.clone() };
    
//     info!("==================================================");
//     info!(" Starting Multi-Asset Master Metric Orchestration. ");
//     info!("==================================================");



//     //  Fetch the list of assets to process
//     let asset_ids = data_service.fetch_all_active_asset_ids().await?;
//     let total_assets = asset_ids.len();

//     if total_assets == 0 {
//         info!("No active assets found to process. Exiting.");
//         return Ok(());
//     }

//     info!("Found {} active assets. Starting parallel ETL execution...", total_assets);

    
//     let mut tasks: Vec<JoinHandle<Result<()>>> = Vec::new();

//     for asset_id in asset_ids {
        
//         let pool_clone = pool.clone();
//         let asset_id_clone = asset_id.clone();

//         let task = tokio::spawn(async move {
//             info!("Starting MASTER METRICS  for asset: {}", asset_id_clone);
            
//             let result = run_master_asset_metrics(&pool_clone, &asset_id_clone).await;
            
//             match result {
//                 Ok(_) => info!(" Metrics Success for asset: {}", asset_id_clone),
//                 Err(e) => {
//                     error!(" Metrics FAILED for asset {}: {:?}", asset_id_clone, e);
            
//                     return Err(anyhow!("ETL failed for asset {}: {}", asset_id_clone, e));
//                 }
//             }
//             Ok(())
//         });
//         tasks.push(task);
//     }

    
//     let results = join_all(tasks).await;

    
//     let mut success_count = 0;
//     let mut failure_count = 0;

//     for result in results {
//         match result {
//             Ok(_) => success_count += 1,
//             Err(_) => failure_count += 1,
//         }
//     }

//     info!("==================================================");
//     info!(" Master Metric Orchestration Summary ");
//     info!("==================================================");
//     info!(" Total Assets Processed: {}", total_assets);
//     info!(" Successful ETL Runs: {}", success_count);
//     info!(" Failed ETL Runs: {}", failure_count);
//     info!("==================================================");

//     Ok(())
// }