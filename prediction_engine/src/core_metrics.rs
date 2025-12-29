
// use anyhow :: {Ok, Result, anyhow};
// use log::info;
// use crate::{ metrics_service::MetricsService};
// use shared_models::{BarDayType2ndOrder, BarTransition2ndOrder, Btcs2ndOrder, BpcsInputData, 
//     BpcsScore2ndOrder, BtcsInputData, CbBaseRate, CsBaseRate, 
//     DailyContextData, DailyTrendContinuationRate, DayOutcome3rdOrder, 
//     DayType2ndOrder, DayTypeBaseRate, Dcs3rdOrder, DcsInputData, 
//     EightContextData, HighLowSession2ndOrder, PcsInputData, PcsScore2ndOrder, 
//     SessionContextData, Tcs2ndOrder, TcsContinuationScore, TcsData, 
//     Transition2ndOrder, SessionToBarMetrics, SessionToBarStcs, StcsInputData
// };

// // CS_bias Base Rate Calculation Call
// pub async fn calculate_and_return_cs_base_rates(
//     metric_service: &MetricsService, 
//     session_contexts: &[SessionContextData], 
//     asset_id: &str
// ) -> Result<Vec<CsBaseRate>> {
    
//     let intervals = metric_service.get_lookback_intervals();
    
//     info!("Starting CS Base Rate Calculation for asset: {}", asset_id);

//     let cs_base_rates = metric_service
//         .calculate_cs_base_rates(session_contexts, &intervals)
//         .await
//         .map_err(|e| anyhow!("CS Base Rate Calculation failed: {}", e))?;

//         if cs_base_rates.is_empty() {
//         info!("No CS Base Rates calculated for {}.", asset_id);
//     }
//     Ok(cs_base_rates)
// }

// pub async fn calculate_and_return_cb_base_rates(
//     metric_service: &MetricsService, 
//     bar_contexts: &[EightContextData], 
//     asset_id: &str
// ) -> Result<Vec<CbBaseRate>> {
    
//     let intervals = metric_service.get_lookback_intervals();
    
//     info!("Starting CS Base Rate Calculation for asset: {}", asset_id);

//     let cb_base_rates = metric_service
//         .calculate_cb_base_rates(bar_contexts, &intervals)
//         .await
//         .map_err(|e| anyhow!("CB Base Rate Calculation failed: {}", e))?;

//         if cb_base_rates.is_empty() {
//         info!("No CB Base Rates calculated for {}.", asset_id);
//     }
//     Ok(cb_base_rates)
// }

// // Day_bias Base Rate Calculation Call
// pub async fn calculate_and_return_day_base_rates(
//     metric_service: &MetricsService, 
//     daily_contexts:&[DailyContextData],
//     asset_id: &str
// ) -> Result<Vec<DayTypeBaseRate>> {
    
//     let intervals = metric_service.get_lookback_intervals();

//     info!("Starting Daily Base Rate Calculation for asset: {}", asset_id);

//     let daily_base_rates = metric_service
//         .calculate_day_type_base_rates(&daily_contexts, &intervals)
//         .await
//         .map_err(|e| anyhow!("Daily Base Rate Calculation failed: {}", e))?;

//         if daily_base_rates.is_empty() {
//         info!("No Daily Base Rates calculated for {}. Skipping persistence.", asset_id);
//     }    
//     Ok(daily_base_rates)
// }

// // 2nd Order Transition Calculation Call
// pub async fn calculate_and_return_transition_2nd_order(
//     metric_service: &MetricsService, 
//     session_contexts: &[SessionContextData],
//     asset_id: &str
// ) -> Result<Vec<Transition2ndOrder>> {
    
//     let intervals = metric_service.get_lookback_intervals();
    
//     info!("Starting 2nd Order Transition Calculation for asset: {}", asset_id);

//     let transitions = metric_service
//         .calculate_transition_2nd_order(session_contexts, &intervals)
//         .await
//         .map_err(|e| anyhow!("2nd Order Transition Calculation failed: {}", e))?;

//         if transitions.is_empty() {
//         info!("No 2nd Order Transitions calculated for {}.", asset_id);
//     }
//     Ok(transitions)
// }

// pub async fn calculate_and_return_bar_transition_2nd_order(
//     metric_service: &MetricsService, 
//     bar_contexts: &[EightContextData],
//     asset_id: &str
// ) -> Result<Vec<BarTransition2ndOrder>> {
    
//     let intervals = metric_service.get_lookback_intervals();
    
//     info!("Starting 2nd Order Transition Calculation for asset: {}", asset_id);

//     let bar_transitions = metric_service
//         .calculate_bar_transition_2nd_order(bar_contexts, &intervals)
//         .await
//         .map_err(|e| anyhow!("2nd Order Bar Transition Calculation failed: {}", e))?;

//         if bar_transitions.is_empty() {
//         info!("No 2nd Order Bar Transitions calculated for {}.", asset_id);
//     }
//     Ok(bar_transitions)
// }
// // 2nd Order Day_Type Calculation Call
// pub async fn calculate_and_return_day_type_2nd_order(
//     metric_service: &MetricsService
//     , session_contexts: &[SessionContextData]
//     , daily_contexts:&[DailyContextData],
//     asset_id: &str
// ) -> Result<Vec<DayType2ndOrder>>{
//     let intervals = metric_service.get_lookback_intervals();

//     info!("Starting 2nd Order Day Type Calculation for asset: {}", asset_id);

//     let day_type = metric_service
//         .calculate_day_type_2nd_order(session_contexts, daily_contexts, &intervals)
//         .await
//         .map_err(|e| anyhow!("2nd Order Day Type Calculation failed: {}", e))?;

//         if day_type.is_empty(){
//         info!("No 2nd Order Day Type Calculations for {}", asset_id);
//     }
//     Ok(day_type)
// }

// pub async fn calculate_and_return_bar_day_type_2nd_order(
//     metric_service: &MetricsService
//     , bar_contexts: &[EightContextData]
//     , daily_contexts:&[DailyContextData],
//     asset_id: &str
// ) -> Result<Vec<BarDayType2ndOrder>>{
//     let intervals = metric_service.get_lookback_intervals();

//     info!("Starting Bar 2nd Order Day Type Calculation for asset: {}", asset_id);

//     let bar_day_type = metric_service
//         .calculate_bar_day_type_2nd_order(bar_contexts, daily_contexts, &intervals)
//         .await
//         .map_err(|e| anyhow!("2nd Order Bar Day Type Calculation failed: {}", e))?;

//         if bar_day_type.is_empty(){
//         info!("No 2nd Order Bar Day Type Calculations for {}", asset_id);
//     }
//     Ok(bar_day_type)
// }

// // 2nd Order Session-to-Bar Calculation Call
// pub async fn calculate_and_return_session_to_bar_2nd_order(
//     metric_service: &MetricsService,
//     session_contexts: &[SessionContextData],
//     eight_hour_contexts: &[EightContextData],
//     asset_id: &str
// ) -> Result<Vec<SessionToBarMetrics>> {
//     let intervals = metric_service.get_lookback_intervals();

//     info!("Starting 2nd Order Session-to-Bar Calculation for asset: {}", asset_id);

//     // This calls the temporal join and aggregation logic we defined
//     let bar_metrics = metric_service
//         .calculate_session_to_bar_2nd_order(session_contexts, eight_hour_contexts, &intervals)
//         .await
//         .map_err(|e| anyhow!("2nd Order Session-to-Bar Calculation failed for {}: {}", asset_id, e))?;

//     if bar_metrics.is_empty() {
//         info!("No 2nd Order Session-to-Bar Calculations found for asset: {}", asset_id);
//     } else {
//         info!(
//             "Successfully calculated {} Session-to-Bar metrics for {}", 
//             bar_metrics.len(), 
//             asset_id
//         );
//     }

//     Ok(bar_metrics)
// }

// // 2nd Order Sessions Extremes Calculation Call
// pub async fn calculate_and_return_exetreme_session_2nd_order_metric(
//     metric_service: &MetricsService, 
//     session_contexts: &[SessionContextData], 
//     daily_extremes: &[DailyContextData], 
//     asset_id: &str
// ) -> Result<Vec<HighLowSession2ndOrder>> {
    
//     let intervals = metric_service.get_lookback_intervals();

//     info!("Starting High Low Session 2nd Order Calculation for asset: {}", asset_id);

    
//     let extremes = metric_service
//         .calculate_high_low_session_2nd_order(session_contexts, daily_extremes, &intervals, 10.0)
//         .await
//         .map_err(|e| anyhow!("2nd Order Day Type Calculation failed: {}", e))?;
    
//     if extremes.is_empty() {
//         info!("No High Low contexts calculated for {}. Skipping persistence.", asset_id);
        
//     }
      
//     Ok(extremes)
// }

// // TCS Continuation Calculation Call
// pub async fn calculate_and_return_tcs_continuation_score(
//     metric_service: &MetricsService, 
//     session_contexts: &[SessionContextData],
//     asset_id: &str
// ) -> Result<Vec<TcsContinuationScore>> {
    
//     let intervals = metric_service.get_lookback_intervals();
    
//     info!("Starting TCS Continuation Score Calculation for asset: {}", asset_id);

//     let tcs_continuation_score = metric_service
//         .calculate_tcs_continuation_score(session_contexts, &intervals)
//         .await
//         .map_err(|e| anyhow!("2nd Order Transition Calculation failed: {}", e))?;

//     if tcs_continuation_score.is_empty() {
//         info!("No TCS Continuation Score Calculation for {}.", asset_id);
//     }
//     Ok(tcs_continuation_score)
// }


// // TCS Score Calculation Call
// pub async fn calculate_and_return_tcs_scores(
//     metric_service: &MetricsService, 
//     transitions: Vec<Transition2ndOrder>, 
//     base_rates: Vec<CsBaseRate>,
//     asset_id: &str
// ) -> Result<Vec<Tcs2ndOrder>> {

//     info!("Starting TCS Score Calculation (In-Memory) for asset: {}", asset_id);
    
//     let input_data = TcsData { transitions, base_rates };

//     let tcs_scores = metric_service
//         .calculate_tcs_scores(input_data)
//         .await
//         .map_err(|e| anyhow!("TCS Score Calculation failed: {}", e))?;

//         if tcs_scores.is_empty() {
//         info!("No TCS Scores calculated for {}.", asset_id);
//     }
//     Ok(tcs_scores)
// }

// pub async fn calculate_and_return_btcs_scores(
//     metric_service: &MetricsService, 
//     transitions: Vec<BarTransition2ndOrder>, 
//     base_rates: Vec<CbBaseRate>,
//     asset_id: &str
// ) -> Result<Vec<Btcs2ndOrder>> {

//     info!("Starting BTCS Score Calculation for asset: {}", asset_id);
    
//     let input_data = BtcsInputData { transitions, base_rates };

//     let btcs_scores = metric_service
//         .calculate_btcs_scores(input_data)
//         .await
//         .map_err(|e| anyhow!("BTCS Score Calculation failed: {}", e))?;

//         if btcs_scores.is_empty() {
//         info!("No BTCS Scores calculated for {}.", asset_id);
//     }
//     Ok(btcs_scores)
// }

// // Calculate and Return Session-to-Bar STCS (Lift)
// pub async fn calculate_and_return_stcs_scores(
//     metric_service: &MetricsService,
//     conditional: Vec<SessionToBarMetrics>,
//     base_rates: Vec<CbBaseRate>,
//     asset_id: &str
// ) -> Result<Vec<SessionToBarStcs>> {
    
//     info!("Starting STCS (Lift) Calculation for asset: {}", asset_id);

//     let input_data = StcsInputData {
//         conditionals: conditional,
//         base_rates,
//     };

//     let stcs_results = metric_service
//         .calculate_stcs_scores(input_data)
//         .await
//         .map_err(|e| anyhow!("STCS Score Calculation failed for {}: {}", asset_id, e))?;

//     if stcs_results.is_empty() {
//         info!("No STCS Scores generated for {}. Check if CbBaseRates exist for target bar numbers.", asset_id);
//     } else {
//         info!("Successfully calculated {} STCS scores for {}", stcs_results.len(), asset_id);
//     }

//     Ok(stcs_results)
// }

// // PCS Score Calculation Call
// pub async fn calculate_and_return_pcs_score(
//     metric_service: &MetricsService,
//     conditionals: Vec<DayType2ndOrder>,
//     base_rates: Vec<DayTypeBaseRate>,
//     asset_id: &str
// ) -> Result<Vec<PcsScore2ndOrder>>{

//     info!("Starting PCS Score Calculation for assets: {}", asset_id);

//     let input_data = PcsInputData {conditionals, base_rates};

//     let pcs_scores = metric_service
//         .calculate_pcs_scores(input_data)
//         .await
//         .map_err(|e| anyhow!("PCS Score Calculation failed: {}", e))?;

//         if pcs_scores.is_empty() {
//         info!("No PCS Scores calculated for {}.", asset_id);
//     }
//     Ok(pcs_scores)
// }


// pub async fn calculate_and_return_bpcs_score(
//     metric_service: &MetricsService,
//     conditionals: Vec<BarDayType2ndOrder>,
//     base_rates: Vec<DayTypeBaseRate>,
//     asset_id: &str
// ) -> Result<Vec<BpcsScore2ndOrder>>{

//     info!("Starting PCS Score Calculation for assets: {}", asset_id);

//     let input_data = BpcsInputData {conditionals, base_rates};

//     let bpcs_scores = metric_service
//         .calculate_bpcs_scores(input_data)
//         .await
//         .map_err(|e| anyhow!("PCS Score Calculation failed: {}", e))?;

//         if bpcs_scores.is_empty() {
//         info!("No PCS Scores calculated for {}.", asset_id);
//     }
//     Ok(bpcs_scores)
// }

// //              --- Daily Metrics Begins here ---

// // 3rd Order Day type conditional Calculation Call 

// pub async fn calculate_and_return_day_type_3rd_order(
//     metric_service: &MetricsService,
//     daily_contexts:&[DailyContextData],
//     asset_id: &str
// ) -> Result<Vec<DayOutcome3rdOrder>>{
//     let intervals = metric_service.get_lookback_intervals();

//     info!("Starting 3rd Order Day Type Calculation for asset: {}", asset_id);

//     let day_type = metric_service
//         .calculate_daily_3rd_order_conditional( daily_contexts, &intervals, 5.0)
//         .await
//         .map_err(|e| anyhow!("3rd Order Day Type Calculation failed: {}", e))?;

//         if day_type.is_empty(){
//         info!("No 3rd Order Day Type Calculations for {}", asset_id);
//     }
//     Ok(day_type)
// }

// // DCS calculation Call 

// pub async fn calculate_and_return_dcs_score(
//     metric_service: &MetricsService,
//     conditionals: Vec<DayOutcome3rdOrder>,
//     base_rates: Vec<DayTypeBaseRate>,
//     asset_id: &str
// ) -> Result<Vec<Dcs3rdOrder>>{

//     info!("Starting DCS Score Calculation for assets: {}", asset_id);

//     let input_data = DcsInputData {conditionals, base_rates};

//     let dcs_scores = metric_service
//         .calculate_dcs_scores(input_data)
//         .await
//         .map_err(|e| anyhow!("DCS Score Calculation failed: {}", e))?;

//         if dcs_scores.is_empty() {
//         info!("No DCS Scores calculated for {}.", asset_id);
//     }
//     Ok(dcs_scores)
// }

// pub async fn calculate_and_return_daily_trend_continuation_score(
//     metric_service: &MetricsService, 
//     daily_contexts:&[DailyContextData],
//     asset_id: &str
// ) -> Result<Vec<DailyTrendContinuationRate>> {
    
//     let intervals = metric_service.get_lookback_intervals();
    
//     info!("Starting Daily Trend Continuation Score Calculation for asset: {}", asset_id);

//     let daily_trend_continuation_score = metric_service
//         .calculate_daily_trend_continuation_rate(daily_contexts, &intervals, 0.0)
//         .await
//         .map_err(|e| anyhow!("3rd Order Transition Calculation failed: {}", e))?;

//     if daily_trend_continuation_score.is_empty() {
//         info!("No Daily Trend Continuation Score Calculation for {}.", asset_id);
//     }
//     Ok(daily_trend_continuation_score)
// }