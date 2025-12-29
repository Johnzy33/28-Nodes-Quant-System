use anyhow::{anyhow, Result};
use log::info;
use chrono::{DateTime, Utc, Datelike, Duration, NaiveDate}; 
use sqlx::{FromRow};
use shared_models::candle_pattern::{get_market_classification}; 
use shared_models::signal_type::{MarketType, MarketSubtype};
use std::cmp::Ordering;
use std::collections::HashMap;
use crate::data_service::DataService;
use crate::data_model as dm;
use shared_models as sm;






// pub fn calculate_session_context_core(
//     sessions: Vec<ClassifiedSession>,
// ) -> Vec<SessionContextData> {
    
//     let mut context_results: Vec<SessionContextData> = Vec::new();
//     let mut preceding_sessions: HashMap<String, (Option<ClassifiedSession>, Option<ClassifiedSession>)> = HashMap::new();

//     for cs in sessions {
//         let asset_id = cs.asset_id.clone();
        
//         // Get or initialize the PS1 and PS2 pointers for this asset
//         let entry = preceding_sessions.entry(asset_id.clone())
//             .or_insert((None, None));
//         // entry is &mut (Option<ClassifiedSession>, Option<ClassifiedSession>)
//         let (ps1, ps2) = entry; // ps1, ps2 are &mut Option<ClassifiedSession>

//         let cs_bias = cs.session_type.to_string();

//         // Calculate and Insert ONLY if PS1 exists (WHERE ps1_name IS NOT NULL)
//         if let Some(p1_session) = ps1.as_ref() {
//             let ps1_name = p1_session.session_name.clone();
//             let ps1_bias = p1_session.session_type.to_string();
            
//             let (ps2_name, ps2_bias) = match ps2.as_ref() {
//                 Some(p2_session) => (Some(p2_session.session_name.clone()), Some(p2_session.session_type.to_string())),
//                 None => (None, None),
//             };

//             context_results.push(SessionContextData {
//                 trading_date: cs.trading_date,
//                 asset_id: cs.asset_id.clone(),
//                 session_end_ts: cs.end_ts, 
//                 cs_name: Some(cs.session_name.clone()),
//                 cs_bias: Some(cs_bias), 
//                 ps1_name: Some(ps1_name),
//                 ps1_bias: Some(ps1_bias),
//                 ps2_name: ps2_name,
//                 ps2_bias: ps2_bias,
//             });
//         }

//         // Update Lookback Pointers (LAG shift)
//         *ps2 = (*ps1).clone();
//         *ps1 = Some(cs);
//     }

//     context_results
// }

// pub fn calculate_eight_hrs_context_core(
//     eight_hrs_block: Vec<Classified8HrBlock>,
// ) -> Vec<EightContextData> {
    
//     let mut context_results: Vec<EightContextData> = Vec::new();
//     let mut preceding_block: HashMap<String, (Option<Classified8HrBlock>, Option<Classified8HrBlock>)> = HashMap::new();

//     for cb in eight_hrs_block {
//         let asset_id = cb.asset_id.clone();
        
//         // Get or initialize the PS1 and PS2 pointers for this asset
//         let entry = preceding_block.entry(asset_id.clone())
//             .or_insert((None, None));
//         // entry is &mut (Option<ClassifiedSession>, Option<ClassifiedSession>)
//         let (pb1, pb2) = entry; // ps1, ps2 are &mut Option<ClassifiedSession>

//         //let cb_bias = get_eight_hrs_bias_string(&cb);
//         let cb_bias = cb.block_type.to_string();

//         // Calculate and Insert ONLY if PS1 exists (WHERE ps1_name IS NOT NULL)
//         if let Some(p1_block) = pb1 {
//             let pb1_num = p1_block.block_number.clone();
//             let pb1_bias = p1_block.block_type.to_string();
            
//             let (pb2_num, pb2_bias) = match pb2 {
//                 Some(p2_block) => (Some(p2_block.block_number.clone()), Some(p2_block.block_type.to_string())),
//                 None => (None, None),
//             };

//             context_results.push(EightContextData {
//                 trading_date: cb.trading_date,
//                 asset_id: cb.asset_id.clone(),
//                 session_end_ts: cb.end_ts, 
//                 cb_num: Some(cb.block_number.clone()),
//                 cb_bias: Some(cb_bias), 
//                 pb1_num: Some(pb1_num),
//                 pb1_bias: Some(pb1_bias),
//                 pb2_num: pb2_num,
//                 pb2_bias:pb2_bias,
//             });
//         }

//         // Update Lookback Pointers (LAG shift)
//         *pb2 = (*pb1).clone();
//         *pb1 = Some(cb);
//     }

//     context_results
// }




// ====================================================================
// 11. DataService Implementation
// ====================================================================

// impl super::DataService { 


//     /// Fetches a list of all active asset_ids from the asset_config table.

    


//     async fn get_context_high_watermark(&self, asset_id: &str) -> Result<Option<DateTime<Utc>>> {
//         let query = r#"
//             SELECT MAX(session_end_ts)
//             FROM session_context
//             WHERE asset_id = $1
//         "#;
        
//         // sqlx::query_scalar! macro simplifies fetching a single optional value
//         let max_ts: Option<Option<DateTime<Utc>>> = sqlx::query_scalar(query)
//             .bind(asset_id)
//             .fetch_optional(&self.pool)
//             .await
//             .map_err(|e| anyhow!("Failed to fetch high water mark: {}", e))?;

//         Ok(max_ts.flatten())
//     }

//     async fn get_block_base_high_watermark(&self, asset_id: &str) -> Result<Option<DateTime<Utc>>> {
//         let query = r#"
//             SELECT MAX(end_ts)
//             FROM block_base
//             WHERE asset_id = $1
//         "#;
        
//         let result: Option<Option<DateTime<Utc>>> = sqlx::query_scalar(query)
//             .bind(asset_id)
//             .fetch_optional(&self.pool)
//             .await
//             .map_err(|e| anyhow!("Failed to fetch block base high water mark: {}", e))?;

//         // Convert DbDateTime<Utc> to DateTime<Utc>
//         Ok(result.flatten().map(|dt| dt.into()))
//     }

//     /// Finds the latest trading_date in the daily_views table for an asset.
//     async fn get_daily_view_high_watermark(&self, asset_id: &str) -> Result<Option<NaiveDate>> {
//     let query = r#"
//         SELECT MAX(trading_date)
//         FROM daily_views
//         WHERE asset_id = $1
//     "#;
    
//     // FIX: Using Option<Option<T>> to handle MAX returning NULL
//     let result: Option<Option<NaiveDate>> = sqlx::query_scalar(query)
//         .bind(asset_id)
//         .fetch_optional(&self.pool)
//         .await
//         .map_err(|e| anyhow!("Failed to fetch daily view high water mark: {}", e))?;

//     // Flatten: resolves nested option (None if MAX was NULL)
//     Ok(result.flatten()) 
//     }

//     /// Finds the latest week_start date in the weekly_views table for an asset.
//     async fn get_weekly_view_high_watermark(&self, asset_id: &str) -> Result<Option<NaiveDate>> {
//     let query = r#"
//         SELECT MAX(week_start)
//         FROM weekly_views
//         WHERE asset_id = $1
//     "#;
    
//     // Use Option<Option<T>> decoding to correctly handle NULL from MAX()
//     let result: Option<Option<NaiveDate>> = sqlx::query_scalar(query)
//         .bind(asset_id)
//         .fetch_optional(&self.pool)
//         .await
//         .map_err(|e| anyhow!("Failed to fetch weekly view high water mark: {}", e))?;

//     Ok(result.flatten()) 
//     }

//     async fn get_monthly_view_high_watermark(&self, asset_id: &str) -> Result<Option<NaiveDate>> {
//     let query = r#"
//         SELECT MAX(month_start)
//         FROM monthly_views
//         WHERE asset_id = $1
//     "#;
    
//     // Use Option<Option<T>> decoding to correctly handle NULL from MAX()
//     let result: Option<Option<NaiveDate>> = sqlx::query_scalar(query)
//         .bind(asset_id)
//         .fetch_optional(&self.pool)
//         .await
//         .map_err(|e| anyhow!("Failed to fetch monthly view high water mark: {}", e))?;

//     Ok(result.flatten()) 
//     }

//     async fn get_yearly_view_high_watermark(&self, asset_id: &str) -> Result<Option<NaiveDate>> {
//     let query = r#"
//         SELECT MAX(year_start)
//         FROM yearly_views
//         WHERE asset_id = $1
//     "#;
    
//     let result: Option<Option<NaiveDate>> = sqlx::query_scalar(query)
//         .bind(asset_id)
//         .fetch_optional(&self.pool)
//         .await
//         .map_err(|e| anyhow!("Failed to fetch yearly view high water mark: {}", e))?;

//     Ok(result.flatten()) 
//     }

//     pub async fn calculate_session_and_context(
//     &self,
//     asset_id: &str,     
//     ) -> Result<(Vec<ClassifiedSession>, Vec<SessionContextData>)> {


//     let max_ts = self.get_context_high_watermark(asset_id).await?;
    
//     let buffer_limit = 3; 


//     let mut query = String::from(
//         "SELECT 
//             trading_date::DATE as trading_date, asset_id, session_name, start_ts, end_ts, 
//             open, high, high_ts, low, low_ts, close, volume, bars
//         FROM session_base 
//         WHERE asset_id = $1 "
//     );

//     let raw_sessions: Vec<RawSessionData> = if let Some(ts) = max_ts {
//         query.push_str(
//             "AND (end_ts >= $2 OR end_ts IN (
//                 SELECT end_ts FROM session_base
//                 WHERE asset_id = $1 AND end_ts < $2
//                 ORDER BY end_ts DESC
//                 LIMIT $3
//             ))
//             ORDER BY end_ts ASC" 
//         );
        
//         info!("HWM found at {} for asset {}. Fetching incremental data with buffer.", ts, asset_id);
        
//         sqlx::query_as(&query)
//             .bind(asset_id)
//             .bind(ts)
//             .bind(buffer_limit)
//             .fetch_all(&self.pool)
//             .await
//             .map_err(|e| anyhow!("Failed incremental fetch: {}", e))?
        
//     } else {
//         // --- 2. Full Refresh Fetch (No HWM, first run) ---
//         query.push_str("ORDER BY end_ts ASC");
//         info!("No HWM found. Performing full historical fetch.");
        
//         sqlx::query_as(&query)
//             .bind(asset_id)
//             .fetch_all(&self.pool)
//             .await
//             .map_err(|e| anyhow!("Failed full fetch: {}", e))?
//     };

//     if raw_sessions.is_empty() {
//         return Ok((Vec::new(), Vec::new()));
//     }
    
    
//     let classified_sessions: Vec<ClassifiedSession> = raw_sessions.into_iter().map(|sbm| {

//         let classification = get_market_classification(sbm.open, sbm.high, sbm.low, sbm.close);
//         ClassifiedSession { 
//             trading_date: sbm.trading_date,
//             asset_id: sbm.asset_id,
//             session_name: sbm.session_name,
//             session_type: classification,
//             start_ts: sbm.start_ts,
//             end_ts: sbm.end_ts,
//             open: sbm.open,
//             high: sbm.high,
//             high_ts: sbm.high_ts,
//             low: sbm.low,
//             low_ts: sbm.low_ts,
//             close: sbm.close,
//             volume: sbm.volume,
//             bars: sbm.bars,
//         }
//     }).collect();
    
//     // L1.5: Sequential Context Calculation ---
//     let session_contexts = calculate_session_context_core(classified_sessions.clone());

//     let final_sessions: Vec<ClassifiedSession> = if max_ts.is_some() {
//             let hwm = max_ts.unwrap();
//             classified_sessions.into_iter()
//                 .filter(|s| s.end_ts >= hwm)
//                 .collect()
//         } else {

//             classified_sessions
//         };
    
//     let final_contexts: Vec<SessionContextData> = if max_ts.is_some() {
//         session_contexts.into_iter()
//             .filter(|ctx| ctx.session_end_ts >= max_ts.unwrap())
//             .collect()
//     } else {
    
//         session_contexts
//     };

//     Ok((final_sessions, final_contexts))
//     }




//     pub async fn calculate_8hr_context_and_blocks(
//         &self,
//         asset_id: &str,
//     ) -> Result<(Vec<Classified8HrBlock>, Vec<EightContextData>)> {

//         // A. Determine High Watermark (HWM) and Buffer
//         let max_end_ts = self.get_block_base_high_watermark(asset_id).await?;
        
//         // Use a small buffer (e.g., 1 hour) to re-aggregate the bar containing the HWM, 
//         // ensuring robustness against late data/partial writes. This is the same logic
//         // as the provided `calculate_classified_8hr_blocks`.
//         let buffer_duration = Duration::hours(1); 

//         let mut query = r#"
//             SELECT
//                 trading_date, asset_id, block_number, start_ts, end_ts, open, high, low, close, volume, bars
//             FROM eight_hr_base 
//             WHERE asset_id = $1 
//         "#.to_string();

//         let all_raw_data: Vec<Raw8HrBlock> = if let Some(last_end_ts) = max_end_ts {
            
//             // Calculate fetch start time: HWM minus buffer
//             let fetch_start_time = last_end_ts - buffer_duration;
            
//             // --- Incremental Fetch Logic ---
//             query.push_str("AND end_ts >= $2 "); 
//             query.push_str("ORDER BY end_ts ASC");
            
//             info!("HWM found at {}. Fetching 8hr Blocks data since {}.", last_end_ts, fetch_start_time);

//             sqlx::query_as(&query)
//                 .bind(asset_id)
//                 .bind(fetch_start_time)
//                 .fetch_all(&self.pool)
//                 .await
//                 .map_err(|e| anyhow!("Failed incremental fetch: {}", e))?
            
//         } else {
//             // --- Full Refresh Fetch (No HWM, first run) ---
//             query.push_str("ORDER BY end_ts ASC");
//             info!("No HWM found. Performing full historical fetch.");

//             sqlx::query_as(&query)
//                 .bind(asset_id)
//                 .fetch_all(&self.pool)
//                 .await
//                 .map_err(|e| anyhow!("Failed full fetch: {}", e))?
//         };

//         if all_raw_data.is_empty() {
//              info!("No new 8hr block data found for {}.", asset_id);
//              return Ok((Vec::new(), Vec::new()));
//         }

//         // B. Classification
//         let classified_blocks: Vec<Classified8HrBlock> = all_raw_data.into_iter()
//             .map(|raw| {
//                 // Placeholder: replace with actual call to your classification function
//                 let classification = get_market_classification(
//                     raw.open, raw.high, raw.low, raw.close
//                 );
            
//                 Classified8HrBlock {
//                     trading_date: raw.trading_date,
//                     asset_id: raw.asset_id,
//                     block_number: raw.block_number,
//                     start_ts: raw.start_ts,
//                     end_ts: raw.end_ts,
//                     open: raw.open,
//                     high: raw.high,
//                     low: raw.low,
//                     close: raw.close,
//                     volume: raw.volume,
//                     bars: raw.bars,
//                     block_type:classification,
                    
//                 }
//             })
//             .collect();
        
//         // C. Sequential Context Calculation
//         let block_contexts = calculate_eight_hrs_context_core(classified_blocks.clone());

//         // D. Final Filtering (Keep only data at or newer than HWM for insertion)
//         let max_end_ts_val = max_end_ts.unwrap_or(DateTime::<Utc>::from_timestamp(0, 0).unwrap());

//         let final_blocks: Vec<Classified8HrBlock> = classified_blocks.into_iter()
//             .filter(|b| b.end_ts >= max_end_ts_val)
//             .collect();
        
//         let final_contexts: Vec<EightContextData> = block_contexts.into_iter()
//             .filter(|ctx| ctx.session_end_ts >= max_end_ts_val)
//             .collect();

//         Ok((final_blocks, final_contexts))
//     }




//     pub async fn calculate_classified_daily_views(
//         &self,
//         asset_id: &str,
//     ) -> Result<Vec<ClassifiedDailyView>> {
        
//         let max_date = self.get_daily_view_high_watermark(asset_id).await?;
//         let buffer_days: i32 = 7; 

//         // Updated Query to include high_bar and low_bar
//         let mut query = r#"
//             SELECT 
//                 trading_date::DATE as trading_date, asset_id, open, high, low, close, 
//                 volume::BIGINT as volume, bars,
//                 start_ts, end_ts, high_ts, high_session, low_ts, low_session,
//                 high_bar, low_bar
//             FROM daily_base 
//             WHERE asset_id = $1 
//         "#.to_string();

//         let raw_data: Vec<RawDailyData> = if let Some(last_date) = max_date {
//             let start_date = last_date.checked_sub_days(chrono::Days::new(buffer_days as u64))
//                 .unwrap_or_else(|| NaiveDate::MIN);

//             query.push_str("AND trading_date >= $2 ");
//             query.push_str("ORDER BY trading_date ASC");

//             sqlx::query_as(&query)
//                 .bind(asset_id)
//                 .bind(start_date)
//                 .fetch_all(&self.pool) 
//                 .await
//                 .map_err(|e| anyhow!("Failed incremental L2 fetch: {}", e))?
//         } else {
//             query.push_str("ORDER BY trading_date ASC");
//             sqlx::query_as(&query)
//                 .bind(asset_id)
//                 .fetch_all(&self.pool) 
//                 .await
//                 .map_err(|e| anyhow!("Failed full L2 fetch: {}", e))?
//         };
        
//         if raw_data.is_empty() {
//             return Ok(Vec::new());
//         }

//         let mut daily_views: Vec<ClassifiedDailyView> = Vec::with_capacity(raw_data.len());
//         let mut prior_day_high: Option<f64> = None;
//         let mut prior_day_low: Option<f64> = None;
        
//         let mut high_history: Vec<(chrono::NaiveDate, f64)> = Vec::new();
//         let mut low_history: Vec<(chrono::NaiveDate, f64)> = Vec::new();
//         let seven_days = chrono::Duration::days(7); 
        
//         for raw in raw_data {
//             let classification = get_market_classification(raw.open, raw.high, raw.low, raw.close);
//             let dow = raw.trading_date.format("%a").to_string(); 

//             let window_start_date = raw.trading_date.checked_sub_days(chrono::Days::new(seven_days.num_days() as u64))
//                                     .unwrap_or_else(|| NaiveDate::MIN);
            
//             high_history.retain(|(date, _)| date >= &window_start_date);
//             low_history.retain(|(date, _)| date >= &window_start_date);
            
//             let prior_week_high = high_history.iter().map(|(_, h)| *h).max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
//             let prior_week_low = low_history.iter().map(|(_, l)| *l).min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

//             // Mapping the NEW fields into the view
//             let view = ClassifiedDailyView {
//                 trading_date: raw.trading_date,
//                 asset_id: raw.asset_id.clone(),
//                 dow,
//                 day_type: classification,
//                 open: raw.open,
//                 high: raw.high,
//                 low: raw.low,
//                 close: raw.close,
//                 volume: raw.volume,
//                 bars: raw.bars,
//                 start_ts: raw.start_ts,
//                 end_ts: raw.end_ts,
//                 high_ts: raw.high_ts,
//                 high_session: raw.high_session,
//                 low_ts: raw.low_ts,
//                 low_session: raw.low_session,
                
//                 // ✅ Mapping the new 8hr block data
//                 high_bar: raw.high_bar,
//                 low_bar: raw.low_bar,
                
//                 prior_day_high,
//                 prior_day_low,
//                 prior_week_high,
//                 prior_week_low,
//             };
//             daily_views.push(view);
            
//             prior_day_high = Some(raw.high);
//             prior_day_low = Some(raw.low);
//             high_history.push((raw.trading_date, raw.high));
//             low_history.push((raw.trading_date, raw.low));
//         }

//         // Filter by HWM as before
//         let final_daily_views: Vec<ClassifiedDailyView> = if let Some(hwm) = max_date {
//             daily_views.into_iter()
//                 .filter(|view| view.trading_date >= hwm)
//                 .collect()
//         } else {
//             daily_views
//         };

//         Ok(final_daily_views)
//     }
    
//     // --- 11.3. Classified Weekly Views ETL (L3) ---
    
//     pub async fn calculate_classified_weekly_views(
//         &self,
//         asset_id: &str,
//     ) -> Result<Vec<ClassifiedWeeklyView>> {
        
//         // A. Determine High Watermark (HWM) and Buffer
//         let max_week_start = self.get_weekly_view_high_watermark(asset_id).await?;
        
//         let mut query = r#"
//             SELECT
//                 trading_date, asset_id, open, high, low, close, volume, bars,
//                 high_ts, high_session, low_ts, low_session
//             FROM daily_views 
//             WHERE asset_id = $1 
//         "#.to_string();

//         let all_daily_data: Vec<RawDailyView> = if let Some(last_week_start) = max_week_start {
            
//             // To be safe, we start fetching from the beginning of the previous week.
//             // This ensures we capture any late updates or re-process the last completed week 
//             // and include the entire current, incomplete week.
//             let fetch_start_date = last_week_start; // Fetch starting from the last persisted week_start

//             query.push_str("AND trading_date >= $2 ");
//             query.push_str("ORDER BY trading_date ASC");
            
//             info!("L3 HWM found at {}. Fetching Daily Views data since {}.", last_week_start, fetch_start_date);

//             sqlx::query_as(&query)
//                 .bind(asset_id)
//                 .bind(fetch_start_date)
//                 .fetch_all(&self.pool)
//                 .await
//                 .map_err(|e| anyhow!("Failed incremental L3 fetch: {}", e))?
            
//         } else {
//             // Full Refresh Fetch (No HWM)
//             query.push_str("ORDER BY trading_date ASC");
//             info!("No L3 HWM found. Performing full historical fetch.");

//             sqlx::query_as(&query)
//                 .bind(asset_id)
//                 .fetch_all(&self.pool)
//                 .await
//                 .map_err(|e| anyhow!("Failed full L3 fetch: {}", e))?
//         };

//         if all_daily_data.is_empty() {
//              info!("No daily data found for weekly calculation for {}.", asset_id);
//              return Ok(Vec::new());
//         }

//         // B. Aggregation Loop (Remains largely the same, but only runs on incremental data)
//         let mut weekly_map: HashMap<chrono::NaiveDate, ClassifiedWeeklyView> = HashMap::new();

//         for daily_record in all_daily_data {
//             // Calculate week start (Monday)
//             let days_since_monday = daily_record.trading_date.weekday().num_days_from_monday() as i64;
//             let week_start_date: chrono::NaiveDate = daily_record.trading_date 
//                 - Duration::days(days_since_monday);
            
//             let weekly_state = weekly_map.entry(week_start_date)
//                 .or_insert_with(|| {
//                     // Initialize state for a NEW week
//                     let month_of_year = week_start_date.month() as i32;
//                     ClassifiedWeeklyView {
//                         week_start: week_start_date,
//                         asset_id: daily_record.asset_id.clone(),
//                         month_of_year,
//                         open: daily_record.open, // OPEN is the OPEN of the first day in the week
//                         high: daily_record.high,
//                         low: daily_record.low,
//                         close: daily_record.close,
//                         volume: 0,
//                         bars: 0,
//                         high_trading_date: daily_record.trading_date,
//                         high_ts: daily_record.high_ts,
//                         high_session: daily_record.high_session.clone(),
//                         low_trading_date: daily_record.trading_date,
//                         low_ts: daily_record.low_ts,
//                         low_session: daily_record.low_session.clone(),
//                         weekly_type: MarketType::Other,
                       
//                     }
//                 });
            
//             // The rest of the logic remains the same:
//             // CLOSE: latest close price is the weekly close
//             weekly_state.close = daily_record.close;

//             // HIGH: Track MAX(high) and update metadata
//             if daily_record.high > weekly_state.high {
//                 weekly_state.high = daily_record.high;
//                 weekly_state.high_trading_date = daily_record.trading_date;
//                 weekly_state.high_ts = daily_record.high_ts;
//                 weekly_state.high_session = daily_record.high_session.clone();
//             }

//             // LOW: Track MIN(low) and update metadata
//             if daily_record.low < weekly_state.low {
//                 weekly_state.low = daily_record.low;
//                 weekly_state.low_trading_date = daily_record.trading_date;
//                 weekly_state.low_ts = daily_record.low_ts;
//                 weekly_state.low_session = daily_record.low_session.clone();
//             }

//             // VOLUME/BARS: SUM()
//             weekly_state.volume += daily_record.volume;
//             weekly_state.bars += daily_record.bars;
//         }

//         // C. Final Classification, Cleanup, and Filtering

//         let all_calculated_weekly_views: Vec<ClassifiedWeeklyView> = weekly_map.into_values()
//             .map(|mut view| {
//                 // The function returns MarketType directly
//                 view.weekly_type = get_market_classification( 
//                     view.open, view.high, view.low, view.close
//                 );
//                 // The line below is now INCORRECT and must be removed or handled differently.
//                 // view.consolidation_subtype = classification.subtype; 
//                 view
//             })
//             .collect();
            
//         // Filter out historical records (keeping only the HWM week and newer)
//         // ... (Filtering and sorting logic remains the same) ...
//         let max_week_start_val = max_week_start.unwrap_or(chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

//         let mut classified_weekly_views: Vec<ClassifiedWeeklyView> = all_calculated_weekly_views.into_iter()
//             .filter(|view| view.week_start >= max_week_start_val)
//             .collect();

//         classified_weekly_views.sort_by_key(|v| v.week_start);

//         Ok(classified_weekly_views)
//     }

//     // --- 11.4. Classified Monthly Views ETL (L4) ---

//     pub async fn calculate_classified_monthly_views(
//         &self,
//         asset_id: &str,
//     ) -> Result<Vec<ClassifiedMonthlyView>> {
        
//         // A. Determine High Watermark (HWM) and Buffer
//         let max_month_start = self.get_monthly_view_high_watermark(asset_id).await?;
        
//         let mut query = r#"
//             SELECT
//                 trading_date, asset_id, open, high, low, close, volume, bars,
//                 high_ts, high_session, 
//                 low_ts, low_session
//             FROM daily_views 
//             WHERE asset_id = $1 
//         "#.to_string();

//         let all_daily_data: Vec<RawDailyView> = if let Some(last_month_start) = max_month_start {
            
//             // FIX: Step back 1 month for the fetch start date (buffer)
//             // This ensures we re-aggregate the previous complete month bar, 
//             // and include the entire current, incomplete month.
//             let fetch_start_date = last_month_start;
            
//             query.push_str("AND trading_date >= $2 ");
//             query.push_str("ORDER BY trading_date ASC");
            
//             info!("L4 HWM found at {}. Fetching Daily Views data since {}.", last_month_start, fetch_start_date);

//             sqlx::query_as(&query)
//                 .bind(asset_id)
//                 .bind(fetch_start_date)
//                 .fetch_all(&self.pool)
//                 .await
//                 .map_err(|e| anyhow!("Failed incremental L4 fetch: {}", e))?
            
//         } else {
//             // Full Refresh Fetch (No HWM)
//             query.push_str("ORDER BY trading_date ASC");
//             info!("No L4 HWM found. Performing full historical fetch.");

//             sqlx::query_as(&query)
//                 .bind(asset_id)
//                 .fetch_all(&self.pool)
//                 .await
//                 .map_err(|e| anyhow!("Failed full L4 fetch: {}", e))?
//         };

//         if all_daily_data.is_empty() {
//              info!("No daily data found for monthly calculation for {}.", asset_id);
//              return Ok(Vec::new());
//         }

//         // B. Aggregation Loop (Uses your existing correct logic on the incremental data)
//         let mut monthly_map: HashMap<chrono::NaiveDate, ClassifiedMonthlyView> = HashMap::new();

//         for daily_record in all_daily_data {
//             // CORRECT KEY: Use the first day of the calendar month for grouping.
//             let month_start_date: chrono::NaiveDate = daily_record.trading_date
//                 .with_day(1) 
//                 .unwrap_or(daily_record.trading_date); 

//             let monthly_state = monthly_map.entry(month_start_date)
//                 .or_insert_with(|| {
//                     // Initialization logic remains correct: use first day's open/metadata
//                     ClassifiedMonthlyView {
//                         month_start: month_start_date,
//                         asset_id: daily_record.asset_id.clone(),
//                         open: daily_record.open, 
//                         high: daily_record.high, 
//                         low: daily_record.low,
//                         close: daily_record.close,
//                         volume: 0,
//                         bars: 0,
//                         monthly_type: MarketType::Other, 
//                         start_trading_date: daily_record.trading_date, 
//                         end_trading_date: daily_record.trading_date,   
//                         high_trading_date: daily_record.trading_date,
//                         high_ts: daily_record.high_ts,
//                         high_session: daily_record.high_session.clone(),
//                         low_trading_date: daily_record.trading_date,
//                         low_ts: daily_record.low_ts,
//                         low_session: daily_record.low_session.clone(),
//                     }
//                 });
            
//             // Aggregation logic remains correct: update close, high, low, sum volume/bars
//             monthly_state.close = daily_record.close;
//             monthly_state.end_trading_date = daily_record.trading_date; 

//             if daily_record.high > monthly_state.high {
//                 monthly_state.high = daily_record.high;
//                 monthly_state.high_trading_date = daily_record.trading_date;
//                 monthly_state.high_ts = daily_record.high_ts;
//                 monthly_state.high_session = daily_record.high_session.clone();
//             }

//             if daily_record.low < monthly_state.low {
//                 monthly_state.low = daily_record.low;
//                 monthly_state.low_trading_date = daily_record.trading_date;
//                 monthly_state.low_ts = daily_record.low_ts;
//                 monthly_state.low_session = daily_record.low_session.clone();
//             }

//             monthly_state.volume += daily_record.volume;
//             monthly_state.bars += daily_record.bars;
//         }

//         // C. Final Classification and Filtering
//         let all_calculated_monthly_views: Vec<ClassifiedMonthlyView> = monthly_map.into_values()
//             .map(|mut view| {
                
//                 view.monthly_type = get_market_classification(
//                     view.open, view.high, view.low, view.close
//                 );
                
//                 view
//             })
//             .collect();
            
//         // Filter out historical records (keeping only the HWM month and newer)
//         let mut classified_monthly_views: Vec<ClassifiedMonthlyView> = if max_month_start.is_some() {
//             let hwm = max_month_start.unwrap();
//             all_calculated_monthly_views.into_iter()
//                 .filter(|view| view.month_start >= hwm)
//                 .collect()
//         } else {
//             all_calculated_monthly_views
//         };

//         classified_monthly_views.sort_by_key(|v| v.month_start);

//         Ok(classified_monthly_views)
//     }

//     // --- 11.5. Classified Yearly Views ETL (L5) ---

//    pub async fn calculate_classified_yearly_views(
//         &self,
//         asset_id: &str,
//     ) -> Result<Vec<ClassifiedYearlyView>> {
        
//         // A. Determine High Watermark (HWM) and Fetch Start Date
//         let max_year_start = self.get_yearly_view_high_watermark(asset_id).await?;
        
//         let mut query = r#"
//             SELECT
//                 month_start, asset_id, open, high, low, close, volume, bars,
//                 start_trading_date, end_trading_date, high_trading_date, high_ts, high_session, 
//                 low_trading_date, low_ts, low_session
//             FROM monthly_views 
//             WHERE asset_id = $1 
//         "#.to_string();

//         let all_monthly_data: Vec<RawMonthlyView> = if let Some(last_year_start) = max_year_start {
            
//             // The fetch starts exactly on the HWM date. This fetches the current, incomplete year.
//             let fetch_start_date = last_year_start; 
            
//             query.push_str("AND month_start >= $2 ");
//             query.push_str("ORDER BY month_start ASC");
            
//             info!("L5 HWM found at {}. Fetching Monthly Views data since {}.", last_year_start, fetch_start_date);
//             // 

//             sqlx::query_as(&query)
//                 .bind(asset_id)
//                 .bind(fetch_start_date)
//                 .fetch_all(&self.pool)
//                 .await
//                 .map_err(|e| anyhow!("Failed incremental L5 fetch: {}", e))?
            
//         } else {
//             // Full Refresh Fetch (No HWM)
//             query.push_str("ORDER BY month_start ASC");
//             info!("No L5 HWM found. Performing full historical fetch.");

//             sqlx::query_as(&query)
//                 .bind(asset_id)
//                 .fetch_all(&self.pool)
//                 .await
//                 .map_err(|e| anyhow!("Failed full L5 fetch: {}", e))?
//         };

//         if all_monthly_data.is_empty() {
//              info!("No monthly data found for yearly calculation for {}.", asset_id);
//              return Ok(Vec::new());
//         }

//         // B. Aggregation Loop
//         // Group by the integer year for simplicity during the loop
//         let mut yearly_map: HashMap<i32, ClassifiedYearlyView> = HashMap::new(); 

//         for monthly_record in all_monthly_data {
//             // Determine the year and year start date
//             let year = monthly_record.month_start.year();
//             let year_start_date: NaiveDate = monthly_record.month_start
//                 .with_month(1).unwrap()
//                 .with_day(1).unwrap(); 

//             let yearly_state = yearly_map.entry(year)
//                 .or_insert_with(|| {
//                     // Initialize with the data from the first month of the year
//                     ClassifiedYearlyView {
//                         year_start: year_start_date,
//                         asset_id: monthly_record.asset_id.clone(),
//                         open: monthly_record.open, // Open is the open of the first month
//                         high: monthly_record.high, 
//                         low: monthly_record.low,
//                         close: monthly_record.close,
//                         volume: 0,
//                         bars: 0,
//                         yearly_type: MarketType::Other,
//                         high_trading_date: monthly_record.high_trading_date,
//                         high_ts: monthly_record.high_ts,
//                         high_session: monthly_record.high_session.clone(), // Corrected initialization
//                         low_trading_date: monthly_record.low_trading_date,
//                         low_ts: monthly_record.low_ts,
//                         low_session: monthly_record.low_session.clone(), // Corrected initialization
//                     }
//                 });
            
//             // --- Aggregation Logic ---
            
//             // CLOSE: The close of the latest month is the yearly close
//             yearly_state.close = monthly_record.close;

//             // HIGH: Track MAX(high) and update all high metadata, INCLUDING SESSION
//             if monthly_record.high > yearly_state.high {
//                 yearly_state.high = monthly_record.high;
//                 yearly_state.high_trading_date = monthly_record.high_trading_date;
//                 yearly_state.high_ts = monthly_record.high_ts;
//                 yearly_state.high_session = monthly_record.high_session.clone(); // ✅ FIXED
//             }

//             // LOW: Track MIN(low) and update all low metadata, INCLUDING SESSION
//             if monthly_record.low < yearly_state.low {
//                 yearly_state.low = monthly_record.low;
//                 yearly_state.low_trading_date = monthly_record.low_trading_date;
//                 yearly_state.low_ts = monthly_record.low_ts;
//                 yearly_state.low_session = monthly_record.low_session.clone(); // ✅ FIXED
//             }

//             // VOLUME/BARS: SUM()
//             yearly_state.volume += monthly_record.volume;
//             yearly_state.bars += monthly_record.bars;
//         }

//         // C. Final Classification and Filtering
//         let all_calculated_yearly_views: Vec<ClassifiedYearlyView> = yearly_map.into_values()
//             .map(|mut view| {
                
//                 view.yearly_type = get_market_classification(
//                     view.open, view.high, view.low, view.close
//                 );
               
//                 view
//             })
//             .collect();
            
//         // Filter out historical records (keeping only the HWM year and newer)
//         let mut classified_yearly_views: Vec<ClassifiedYearlyView> = if max_year_start.is_some() {
//             let hwm = max_year_start.unwrap();
//             all_calculated_yearly_views.into_iter()
//                 .filter(|view| view.year_start >= hwm)
//                 .collect()
//         } else {
//             all_calculated_yearly_views
//         };

//         classified_yearly_views.sort_by_key(|v| v.year_start);

//         Ok(classified_yearly_views)
//     }
// }

pub fn generate_context<T, R, F>(data: &[T], mut transform: F) -> Vec<R>
where
    F: FnMut(&T, Option<&T>, Option<&T>) -> Option<R>,
{
    let mut results = Vec::new();
    
    // We use indices or windows to avoid cloning the elements inside the loop
    for i in 0..data.len() {
        let current = &data[i];
        let p1 = if i >= 1 { Some(&data[i - 1]) } else { None };
        let p2 = if i >= 2 { Some(&data[i - 2]) } else { None };

        if let Some(res) = transform(current, p1, p2) {
            results.push(res);
        }
    }
    results
}

impl DataService {



    /// Generic High Watermark: Works for any table, column, and return type (Date or DateTime)
    async fn get_hwm<T>(
        &self, 
        table: &str, 
        column: &str, 
        asset_id: &str
    ) -> Result<Option<T>>
    where
        T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,
    {
        let query = format!("SELECT MAX({}) FROM {} WHERE asset_id = $1", column, table);
        let result: Option<Option<T>> = sqlx::query_scalar(&query)
            .bind(asset_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow!("HWM Error {}.{}: {}", table, column, e))?;
        Ok(result.flatten())
    }

    /// Unified Incremental Fetcher: Handles the "Buffer" logic for both Date and DateTime types
    async fn fetch_buffered<T, H>(
        &self, 
        query_base: &str, 
        asset_id: &str, 
        hwm: Option<H>, 
        buffer: Duration,
        filter_col: &str, // Explicitly pass the column name (e.g., "month_start")
    ) -> Result<Vec<T>>
    where 
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
        H: Into<WatermarkValue> + Copy,
    {
        let mut final_query = match hwm {
            Some(_) => {
                // Use the provided filter_col instead of guessing
                format!("{} AND {} >= $2 ORDER BY {} ASC", query_base, filter_col, filter_col)
            },
            None => format!("{} ORDER BY 1 ASC", query_base),
        };

        let q = sqlx::query_as::<_, T>(&final_query).bind(asset_id);
        
        let result = match hwm {
            Some(h) => match h.into() {
                WatermarkValue::DateTime(ts) => q.bind(ts - buffer).fetch_all(&self.pool).await,
                WatermarkValue::Date(d) => q.bind(d - Duration::days(buffer.num_days())).fetch_all(&self.pool).await,
            },
            None => q.fetch_all(&self.pool).await,
        };

        result.map_err(|e| anyhow!("Buffered Fetch Failed on {}: {}", filter_col, e))
    }

pub async fn calculate_session_and_context(
    &self, 
    asset_id: &str
) -> Result<(Vec<dm::ClassifiedSession>, Vec<sm::SessionContextData>)> {

    let hwm: Option<DateTime<Utc>> = self.get_hwm("session_context", "session_end_ts", asset_id).await?;
    
    let raw: Vec<dm::RawSessionData> = self.fetch_buffered(
        "SELECT * FROM session_base WHERE asset_id = $1", 
        asset_id, hwm, Duration::hours(1),
        "end_ts"
    ).await?;

    // Map raw data to classified sessions
    let classified: Vec<dm::ClassifiedSession> = raw.into_iter().map(|s| dm::ClassifiedSession {
        session_type: get_market_classification(s.open, s.high, s.low, s.close),
        trading_date: s.trading_date, 
        asset_id: s.asset_id, 
        session_name: s.session_name,
        start_ts: s.start_ts, 
        end_ts: s.end_ts, 
        open: s.open, 
        high: s.high, 
        high_ts: s.high_ts, 
        low: s.low, 
        low_ts: s.low_ts, 
        close: s.close, 
        volume: s.volume, 
        bars: s.bars
    }).collect();

    // OPTIMIZED: Passing &classified as a slice (&[T]) avoids a full heap clone
    let contexts = generate_context(&classified, |curr, p1, p2| {
        p1.map(|prev1| sm::SessionContextData {
            trading_date: curr.trading_date,
            asset_id: curr.asset_id.clone(),
            session_end_ts: curr.end_ts,
            cs_name: Some(curr.session_name.clone()),
            cs_bias: Some(curr.session_type.to_string()),
            ps1_name: Some(prev1.session_name.clone()),
            ps1_bias: Some(prev1.session_type.to_string()),
            ps2_name: p2.map(|p| p.session_name.clone()),
            ps2_bias: p2.map(|p| p.session_type.to_string()),
        })
    });

    let limit = hwm.unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
    
    Ok((
        classified.into_iter().filter(|s| s.end_ts >= limit).collect(),
        contexts.into_iter().filter(|c| c.session_end_ts >= limit).collect()
    ))
}

pub async fn calculate_8hr_context_and_blocks(
    &self,
    asset_id: &str,
) -> Result<(Vec<dm::Classified8HrBlock>, Vec<sm::EightContextData>)> {
    
    let hwm: Option<DateTime<Utc>> = self.get_hwm("block_base", "end_ts", asset_id).await?;
    
    let raw: Vec<dm::Raw8HrBlock> = self.fetch_buffered(
        "SELECT * FROM eight_hr_base WHERE asset_id = $1",
        asset_id,
        hwm,
        Duration::hours(1),
        "end_ts"

    ).await?;

    if raw.is_empty() { return Ok((Vec::new(), Vec::new())); }

    let classified: Vec<dm::Classified8HrBlock> = raw.into_iter().map(|r| dm::Classified8HrBlock {
        block_type: get_market_classification(r.open, r.high, r.low, r.close),
        trading_date: r.trading_date, 
        asset_id: r.asset_id, 
        block_number: r.block_number,
        start_ts: r.start_ts, 
        end_ts: r.end_ts, 
        open: r.open, 
        high: r.high,
        low: r.low, 
        close: r.close, 
        volume: r.volume, 
        bars: r.bars,
    }).collect();

    // OPTIMIZED: generate_context now borrows 'classified'
    let contexts = generate_context(&classified, |curr, p1, p2| {
        p1.map(|pb1| sm::EightContextData {
            trading_date: curr.trading_date,
            asset_id: curr.asset_id.clone(),
            session_end_ts: curr.end_ts,
            cb_num: Some(curr.block_number),
            cb_bias: Some(curr.block_type.to_string()),
            pb1_num: Some(pb1.block_number),
            pb1_bias: Some(pb1.block_type.to_string()),
            pb2_num: p2.map(|p| p.block_number),
            pb2_bias: p2.map(|p| p.block_type.to_string()),
        })
    });

    let limit = hwm.unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
    
    Ok((
        classified.into_iter().filter(|b| b.end_ts >= limit).collect(),
        contexts.into_iter().filter(|c| c.session_end_ts >= limit).collect()
    ))
}

    pub async fn calculate_classified_daily_views(
        &self,
        asset_id: &str,
    ) -> Result<Vec<dm::ClassifiedDailyView>> {
        let hwm: Option<NaiveDate> = self.get_hwm("daily_views", "trading_date", asset_id).await?;
        
        // Fetch daily_base with 7-day buffer to populate Prior Day and Prior Week high/lows
        let raw_data: Vec<dm::RawDailyData> = self.fetch_buffered(
            "SELECT * FROM daily_base WHERE asset_id = $1",
            asset_id,
            hwm,
            Duration::days(7),
            "trading_date" // Explicitly specify the filter column

        ).await?;

        if raw_data.is_empty() { return Ok(Vec::new()); }

        let mut daily_views = Vec::with_capacity(raw_data.len());
        let mut prior_day_high: Option<f64> = None;
        let mut prior_day_low: Option<f64> = None;
        
        // Tracking for rolling 7-day window
        let mut high_history: Vec<(NaiveDate, f64)> = Vec::new();
        let mut low_history: Vec<(NaiveDate, f64)> = Vec::new();

        for raw in raw_data {
            let classification = get_market_classification(raw.open, raw.high, raw.low, raw.close);
            let window_start = raw.trading_date - Duration::days(7);
            
            // Clean up history and calculate Prior Week High/Low
            high_history.retain(|(date, _)| date >= &window_start);
            low_history.retain(|(date, _)| date >= &window_start);
            
            let prior_week_high = high_history.iter().map(|(_, h)| *h).max_by(|a, b| a.partial_cmp(b).unwrap());
            let prior_week_low = low_history.iter().map(|(_, l)| *l).min_by(|a, b| a.partial_cmp(b).unwrap());

            daily_views.push(dm::ClassifiedDailyView {
                trading_date: raw.trading_date,
                asset_id: raw.asset_id.clone(),
                dow: raw.trading_date.format("%a").to_string(),
                day_type: classification,
                open: raw.open, high: raw.high, low: raw.low, close: raw.close,
                volume: raw.volume, bars: raw.bars,
                start_ts: raw.start_ts, end_ts: raw.end_ts,
                high_ts: raw.high_ts, high_session: raw.high_session,
                low_ts: raw.low_ts, low_session: raw.low_session,
                high_bar: raw.high_bar, low_bar: raw.low_bar,
                prior_day_high,
                prior_day_low,
                prior_week_high,
                prior_week_low,
            });

            // Update lookback state for next iteration
            prior_day_high = Some(raw.high);
            prior_day_low = Some(raw.low);
            high_history.push((raw.trading_date, raw.high));
            low_history.push((raw.trading_date, raw.low));
        }

        let limit = hwm.unwrap_or(NaiveDate::MIN);
        Ok(daily_views.into_iter().filter(|v| v.trading_date >= limit).collect())
    }

    pub async fn calculate_classified_weekly_views(&self, asset_id: &str) -> Result<Vec<dm::ClassifiedWeeklyView>> {
        let hwm: Option<NaiveDate> = self.get_hwm("weekly_views", "week_start", asset_id).await?;
        
        let daily_data: Vec<dm::RawDailyView> = self.fetch_buffered(
            "SELECT * FROM daily_views WHERE asset_id = $1", 
            asset_id, hwm, Duration::days(7),
            "trading_date" // Explicitly specify the filter column

        ).await?;

        let mut weekly_map: HashMap<NaiveDate, dm::ClassifiedWeeklyView> = HashMap::new();

        for d in daily_data {
            let week_start = d.trading_date - Duration::days(d.trading_date.weekday().num_days_from_monday() as i64);
            
            let entry = weekly_map.entry(week_start).or_insert_with(|| dm::ClassifiedWeeklyView {
                week_start, asset_id: d.asset_id.clone(), month_of_year: week_start.month() as i32,
                open: d.open, high: d.high, low: d.low, close: d.close, volume: 0, bars: 0,
                high_trading_date: d.trading_date, high_ts: d.high_ts, high_session: d.high_session.clone(),
                low_trading_date: d.trading_date, low_ts: d.low_ts, low_session: d.low_session.clone(),
                weekly_type: MarketType::Other,
            });

            entry.close = d.close;
            entry.volume += d.volume;
            entry.bars += d.bars;

            if d.high > entry.high {
                entry.high = d.high;
                entry.high_trading_date = d.trading_date;
                entry.high_ts = d.high_ts;
                entry.high_session = d.high_session.clone();
            }
            if d.low < entry.low {
                entry.low = d.low;
                entry.low_trading_date = d.trading_date;
                entry.low_ts = d.low_ts;
                entry.low_session = d.low_session.clone();
            }
        }

        let limit = hwm.unwrap_or(NaiveDate::MIN);
        let mut result: Vec<_> = weekly_map.into_values().map(|mut w| {
            w.weekly_type = get_market_classification(w.open, w.high, w.low, w.close);
            w
        }).filter(|w| w.week_start >= limit).collect();
        
        result.sort_by_key(|v| v.week_start);
        Ok(result)
    }

    pub async fn calculate_classified_monthly_views(
        &self,
        asset_id: &str,
    ) -> Result<Vec<dm::ClassifiedMonthlyView>> {
        // 1. Get HWM from the target table
        let hwm: Option<NaiveDate> = self.get_hwm("monthly_views", "month_start", asset_id).await?;
        
        // 2. Fetch Daily Views with a buffer to re-aggregate the current month
        let daily_data: Vec<dm::RawDailyView> = self.fetch_buffered(
            "SELECT * FROM daily_views WHERE asset_id = $1", 
            asset_id, 
            hwm, 
            Duration::days(31), // Buffer for a full month
            "trading_date" // Explicitly specify the filter column
        ).await?;

        if daily_data.is_empty() { return Ok(Vec::new()); }

        let mut monthly_map: HashMap<NaiveDate, dm::ClassifiedMonthlyView> = HashMap::new();

        for d in daily_data {
            // Normalize trading_date to the 1st of the month for grouping
            let month_start = d.trading_date.with_day(1).unwrap();
            
            let entry = monthly_map.entry(month_start).or_insert_with(|| dm::ClassifiedMonthlyView {
                month_start,
                asset_id: d.asset_id.clone(),
                open: d.open,
                high: d.high,
                low: d.low,
                close: d.close,
                volume: 0,
                bars: 0,
                monthly_type: MarketType::Other,
                start_trading_date: d.trading_date,
                end_trading_date: d.trading_date,
                high_trading_date: d.trading_date,
                high_ts: d.high_ts,
                high_session: d.high_session.clone(),
                low_trading_date: d.trading_date,
                low_ts: d.low_ts,
                low_session: d.low_session.clone(),
            });

            // Update aggregation state
            entry.close = d.close;
            entry.end_trading_date = d.trading_date;
            entry.volume += d.volume;
            entry.bars += d.bars;

            if d.high > entry.high {
                entry.high = d.high;
                entry.high_trading_date = d.trading_date;
                entry.high_ts = d.high_ts;
                entry.high_session = d.high_session.clone();
            }
            if d.low < entry.low {
                entry.low = d.low;
                entry.low_trading_date = d.trading_date;
                entry.low_ts = d.low_ts;
                entry.low_session = d.low_session.clone();
            }
        }

        let limit = hwm.unwrap_or(NaiveDate::MIN);
        let mut result: Vec<_> = monthly_map.into_values().map(|mut m| {
            m.monthly_type = get_market_classification(m.open, m.high, m.low, m.close);
            m
        }).filter(|m| m.month_start >= limit).collect();

        result.sort_by_key(|v| v.month_start);
        Ok(result)
    }


    

    pub async fn calculate_classified_yearly_views(
        &self,
        asset_id: &str,
    ) -> Result<Vec<dm::ClassifiedYearlyView>> {
        // 1. Get HWM from the target table
        let hwm: Option<NaiveDate> = self.get_hwm("yearly_views", "year_start", asset_id).await?;
        
        // 2. Fetch Monthly Views (no buffer needed usually, or 365 days for safety)
        let monthly_data: Vec<dm::RawMonthlyView> = self.fetch_buffered(
            "SELECT * FROM monthly_views WHERE asset_id = $1", 
            asset_id, 
            hwm, 
            Duration::days(365),
            "month_start" // Explicitly specify the filter column
        ).await?;

        if monthly_data.is_empty() { return Ok(Vec::new()); }

        let mut yearly_map: HashMap<i32, dm::ClassifiedYearlyView> = HashMap::new();

        for m in monthly_data {
            let year = m.month_start.year();
            let year_start = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();

            let entry = yearly_map.entry(year).or_insert_with(|| dm::ClassifiedYearlyView {
                year_start,
                asset_id: m.asset_id.clone(),
                open: m.open,
                high: m.high,
                low: m.low,
                close: m.close,
                volume: 0,
                bars: 0,
                yearly_type: MarketType::Other,
                high_trading_date: m.high_trading_date,
                high_ts: m.high_ts,
                high_session: m.high_session.clone(),
                low_trading_date: m.low_trading_date,
                low_ts: m.low_ts,
                low_session: m.low_session.clone(),
            });

            // Update aggregation state
            entry.close = m.close;
            entry.volume += m.volume;
            entry.bars += m.bars;

            if m.high > entry.high {
                entry.high = m.high;
                entry.high_trading_date = m.high_trading_date;
                entry.high_ts = m.high_ts;
                entry.high_session = m.high_session.clone();
            }
            if m.low < entry.low {
                entry.low = m.low;
                entry.low_trading_date = m.low_trading_date;
                entry.low_ts = m.low_ts;
                entry.low_session = m.low_session.clone();
            }
        }

        let limit = hwm.unwrap_or(NaiveDate::MIN);
        let mut result: Vec<_> = yearly_map.into_values().map(|mut y| {
            y.yearly_type = get_market_classification(y.open, y.high, y.low, y.close);
            y
        }).filter(|y| y.year_start >= limit).collect();

        result.sort_by_key(|v| v.year_start);
        Ok(result)
    }

}

// Helper enum for generic watermark handling
pub enum WatermarkValue { DateTime(DateTime<Utc>), Date(NaiveDate) }
impl From<DateTime<Utc>> for WatermarkValue { fn from(dt: DateTime<Utc>) -> Self { Self::DateTime(dt) } }
impl From<NaiveDate> for WatermarkValue { fn from(d: NaiveDate) -> Self { Self::Date(d) } }

