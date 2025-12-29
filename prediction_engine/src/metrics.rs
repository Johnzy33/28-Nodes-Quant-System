
// use std::collections::HashMap;
// use anyhow::{Ok, Result};
// use chrono::{Utc, NaiveDate, Duration};
// use shared_models::models::{
//     DailyOutcome,  Transition2ndOrder, LookbackInterval,
//     DayTypeBaseRate, CsBaseRate, CbBaseRate, SessionContextData, HighLowSession2ndOrder, 
//     TcsContinuationScore, DailyContextData, EightContextData, 
//     Tcs2ndOrder,BarTransition2ndOrder, TcsData, BtcsInputData,Btcs2ndOrder, DayType2ndOrder, BarDayType2ndOrder, 
//     PcsScore2ndOrder, PcsInputData,BpcsInputData, BpcsScore2ndOrder,
//     DayOutcome3rdOrder,Dcs3rdOrder, DcsInputData, DailyTrendContinuationRate,
//     SessionToBarMetrics,StcsInputData, SessionToBarStcs,
// };
// use crate::{BarBaseRateContext, BaseRateSessionContext, SessionContinuationContext, 
//     DailyBaseRateContext, DayType2ndOrderContext, DayType2ndOrderBarContext,
//     Transition2ndOrderContext, Transition2ndOrderBarContext, SessionToBar2ndOrderContext,
//     aggregate_weighted_counts, prepare_daily_3rd_order_contexts, prepare_dtcr_contexts, prepare_high_low_contexts
// };



// // Requires a helper function for specific decimal rounding (common in Rust)
// fn round_to_decimal_places(value: f64, places: u32) -> f64 {
//     let factor = 10.0f64.powi(places as i32);
//     (value * factor).round() / factor
// }
    


// impl super::MetricsService {


    //  pub fn get_lookback_intervals(&self) -> Vec<LookbackInterval> {

        
    //     let today = Utc::now().date_naive();             
    //     let six_months_ago = today - Duration::days(180); 
    //     let one_year_ago = today - Duration::days(365);
    //     let all_history_start = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();

    //     vec![
    //         LookbackInterval {
    //             name: "6M",
    //             weight: 0.60, 
    //             start_date: six_months_ago, 
    //         },
    //         LookbackInterval {
    //             name: "1Y",
    //             weight: 0.30,
    //             start_date: one_year_ago, 
    //         },
    //         LookbackInterval {
    //             name: "ALL",
    //             weight: 0.10,
    //             start_date: all_history_start, 
    //         },
    //     ]
    // }


//     // Calculates the overall probability (Base Rate) for each Current Session (CS) Bias.
//    pub async fn calculate_cs_base_rates(
//     &self,
//     session_contexts: &[SessionContextData],
//     intervals: &[LookbackInterval],
// ) -> Result<Vec<CsBaseRate>> {
//     let base_contexts: Vec<BaseRateSessionContext> = session_contexts.iter()
//         .cloned()
//         .map(|s| BaseRateSessionContext { inner: s })
//         .collect();

//     let results = aggregate_weighted_counts(&base_contexts, intervals, 10.0);

//     let cs_base_rate = results.into_iter()
//         .map(|(key, (success, total))| {
//             let (asset_id, pattern_key, cs_bias) = key;
            
//             // pattern_key is the (Asset, SessionName) tuple
//             let cs_name = pattern_key; 
            
//             let p_base = success / total;

//             CsBaseRate {
//                 asset_id,
//                 cs_name, // Now included
//                 cs_bias,
//                 p_base: round_to_decimal_places(p_base, 4),
//             }
//         })
//         .collect();
//     Ok(cs_base_rate)
// }


//     // pub async fn calculate_cb_base_rates(
//     //     &self,
//     //     bar_contexts: &[EightContextData],
//     //     intervals: &[LookbackInterval],
//     // ) -> Result<Vec<CbBaseRate>> {
    
//     //     let base_contexts: Vec<BarBaseRateContext> = bar_contexts.iter()
//     //         .cloned()
//     //         .map(|s| BarBaseRateContext { inner: s })
//     //         .collect();

//     //     let results = aggregate_weighted_counts(
//     //         &base_contexts, 
//     //         intervals, 
//     //         5.0
//     //     );

//     //     let cb_base_rate = results.into_iter()
//     //         .map(|(key, (success, total))| {
                
//     //             let (asset_id, _, cb_bias) = key;
//     //             let p_base = success / total;
//     //             let p_base_rounded = round_to_decimal_places(p_base, 4);

//     //             CbBaseRate {
//     //                 asset_id,
//     //                 cb_bias,
//     //                 p_base: p_base_rounded,
//     //             }
//     //         })
//     //         .collect();
//     //     Ok(cb_base_rate)

//     // }

//     pub async fn calculate_cb_base_rates(
//     &self,
//     bar_contexts: &[EightContextData],
//     intervals: &[LookbackInterval],
// ) -> Result<Vec<CbBaseRate>> {

//     let base_contexts: Vec<BarBaseRateContext> = bar_contexts.iter()
//         .cloned()
//         .map(|s| BarBaseRateContext { inner: s })
//         .collect();

//     // The aggregator now groups by (Asset, BarNum)
//     let results = aggregate_weighted_counts(
//         &base_contexts, 
//         intervals, 
//         5.0
//     );

//     let cb_base_rate = results.into_iter()
//         .map(|(key, (success, total))| {
//             let (asset_id, pattern_key, cb_bias) = key;
            
//             // pattern_key is now the (String, i32) we defined above
//             let bar_num = pattern_key; 
            
//             let p_base = success / total;
//             let p_base_rounded = round_to_decimal_places(p_base, 4);

//             CbBaseRate {
//                 asset_id,
//                 cb_num: Some(bar_num), // Now included
//                 cb_bias,
//                 p_base: p_base_rounded,
//             }
//         })
//         .collect();

//     Ok(cb_base_rate)
// }

//     // Calculates the overall probability (Base Rate) for each Day Bias
//     pub async fn calculate_day_type_base_rates(
//         &self,
//         daily_contexts: &[DailyContextData],
//         intervals: &[LookbackInterval],
//     ) -> Result<Vec<DayTypeBaseRate>> {
    
//         let base_contexts: Vec<DailyBaseRateContext> = daily_contexts.iter()
//             .cloned()
//             .map(|d| DailyBaseRateContext { inner: d })
//             .collect();


//         let results = aggregate_weighted_counts(
//             &base_contexts, 
//             intervals, 
//             10.0 
//         );

//         let day_type_base_rates = results.into_iter()
//             .map(|(key, (success, total))| {
                
//                 let (asset_id, _, daily_bias) = key;
//                 let raw_p_base_ml = success / total; 
//                 let p_day_type_base = round_to_decimal_places(raw_p_base_ml, 4);
        
//                 DayTypeBaseRate {
//                     asset_id,
//                     daily_bias,
//                     p_day_type_base:p_day_type_base,
//                 }
//             })
//             .collect();
        
//         Ok(day_type_base_rates)
//     }

//     // Calculate the 2nd Order Probability for Daily Transition PS2_bias -> PS1_bias -> CS_bias 
//    pub async fn calculate_transition_2nd_order(
//         &self,
//         session_contexts: &[SessionContextData],
//         intervals: &[LookbackInterval],
//     ) -> Result<Vec<Transition2ndOrder>> {
    
    
//     let transition_contexts: Vec<Transition2ndOrderContext> = session_contexts.iter()
//         .cloned()
//         .map(|s| Transition2ndOrderContext { inner: s })
//         .collect();

    
//     // The key is (asset_id, (PS2 Name, PS2 Bias, PS1 Name, PS1 Bias), (CS Name, CS Bias))
//     let results = aggregate_weighted_counts(
//         &transition_contexts, 
//         intervals, 
//         10.0 
//     );


//     let transition_2nd_order: Vec<Transition2ndOrder> = results.into_iter()
//         .map(|(key, (success, total))| {
            
//             let (asset_id, pattern_key, outcome_key) = key; 
//             let (ps2_name, ps2_bias, ps1_name, ps1_bias) = pattern_key; 
//             let (cs_name, cs_bias) = outcome_key;

//             let p_conditional = success / total;
//             let p_conditional_rounded = round_to_decimal_places(p_conditional, 4);
            
            
//             Transition2ndOrder {
//                 asset_id,
//                 ps2_bias,
//                 ps1_bias,
//                 cs_bias,
//                 ps2_name, 
//                 ps1_name, 
//                 cs_name,  
//                 reliable_total_attempts: total.round(),
//                 reliable_success_count: success.round(),
//                 p_transition_conditional: p_conditional_rounded,
//             }
//         })
//         .collect();
        
//     Ok(transition_2nd_order)
//     }


//      pub async fn calculate_bar_transition_2nd_order(
//         &self,
//         bar_contexts: &[EightContextData],
//         intervals: &[LookbackInterval],
//     ) -> Result<Vec<BarTransition2ndOrder>> {
    
    
//     let bar_transition_contexts: Vec<Transition2ndOrderBarContext> = bar_contexts.iter()
//         .cloned()
//         .map(|s| Transition2ndOrderBarContext { inner: s })
//         .collect();

    
//     // The key is (asset_id, (PS2 Name, PS2 Bias, PS1 Name, PS1 Bias), (CS Name, CS Bias))
//     let results = aggregate_weighted_counts(
//         &bar_transition_contexts, 
//         intervals, 
//         10.0 
//     );


//     let bar_transition_2nd_order: Vec<BarTransition2ndOrder> = results.into_iter()
//         .map(|(key, (success, total))| {
            
//             let (asset_id, pattern_key, outcome_key) = key; 
//             let (pb2_num, pb2_bias, pb1_num, pb1_bias) = pattern_key; 
//             let (cb_num, cb_bias) = outcome_key;

//             let p_conditional = success / total;
//             let p_conditional_rounded = round_to_decimal_places(p_conditional, 4);
            
            
//             BarTransition2ndOrder {
//                 asset_id,
//                 pb2_bias: pb2_bias,
//                 pb1_bias: pb1_bias,
//                 cb_bias: cb_bias,
//                 pb2_num: pb2_num, 
//                 pb1_num: pb1_num, 
//                 cb_num: cb_num,  
//                 reliable_total_attempts: total.round(),
//                 reliable_success_count: success.round(),
//                 p_transition_conditional: p_conditional_rounded,
//             }
//         })
//         .collect();
        
//     Ok(bar_transition_2nd_order)
//     }

//     // Calculate the 2nd Order Probability for Daily_bias  PS2_bias -> PS1_bias -> Day_bias 
//     pub async fn calculate_day_type_2nd_order(
//         &self,
//         session_contexts: &[SessionContextData],
//         daily_contexts: &[DailyContextData],
//         intervals: &[LookbackInterval],
//     ) -> Result<Vec<DayType2ndOrder>> { 

    
//         let daily_map: HashMap<NaiveDate, DailyContextData> = daily_contexts.iter()
//             .map(|d| (d.trading_date, d.clone()))
//             .collect();
        
        
//         let day_type_contexts: Vec<DayType2ndOrderContext> = session_contexts.iter()
//             .filter_map(|s| {

//                 daily_map.get(&s.trading_date)
//                     .map(|d| DayType2ndOrderContext {
//                         inner: s.clone(),
//                         outcome: d.clone(),
//                     })
//             })
//             .collect();

//         if day_type_contexts.is_empty() {
//             return Ok(Vec::new());
//         }

//         let results = aggregate_weighted_counts(
//             &day_type_contexts, 
//             intervals, 
//             10.0
//         );


//         let day_type_2nd_order: Vec<DayType2ndOrder> = results.into_iter()
//             .map(|(key, (success, total))| {

//                 let (asset_id, pattern_key, daily_bias) = key; 
//                 let (ps2_name, ps2_bias, ps1_name, ps1_bias) = pattern_key; 
                
//                 let p_conditional = success / total;
//                 let p_conditional_rounded = round_to_decimal_places(p_conditional, 4);
                
                
//                 DayType2ndOrder {
//                     asset_id,
//                     ps2_name, 
//                     ps2_bias,
//                     ps1_name, 
//                     ps1_bias,
//                     day_type:daily_bias,
//                     reliable_total_attempts: total.round(),
//                     reliable_success_count: success.round(),
//                     p_day_type_conditional: p_conditional_rounded, 
//                 }
//             })
//             .collect();
            
//         Ok(day_type_2nd_order)
//     }

//     pub async fn calculate_bar_day_type_2nd_order(
//         &self,
//         bar_contexts: &[EightContextData],
//         daily_contexts: &[DailyContextData],
//         intervals: &[LookbackInterval],
//     ) -> Result<Vec<BarDayType2ndOrder>> { 

    
//         let daily_map: HashMap<NaiveDate, DailyContextData> = daily_contexts.iter()
//             .map(|d| (d.trading_date, d.clone()))
//             .collect();
        
        
//         let day_type_contexts: Vec<DayType2ndOrderBarContext> = bar_contexts.iter()
//             .filter_map(|s| {

//                 daily_map.get(&s.trading_date)
//                     .map(|d| DayType2ndOrderBarContext {
//                         inner: s.clone(),
//                         outcome: d.clone(),
//                     })
//             })
//             .collect();

//         if day_type_contexts.is_empty() {
//             return Ok(Vec::new());
//         }

//         let results = aggregate_weighted_counts(
//             &day_type_contexts, 
//             intervals, 
//             10.0
//         );


//         let bar_day_type_2nd_order: Vec<BarDayType2ndOrder> = results.into_iter()
//             .map(|(key, (success, total))| {

//                 let (asset_id, pattern_key, daily_bias) = key; 
//                 let (pb2_num, pb2_bias, pb1_num, pb1_bias) = pattern_key; 
                
//                 let p_conditional = success / total;
//                 let p_conditional_rounded = round_to_decimal_places(p_conditional, 4);
                
                
//                 BarDayType2ndOrder {
//                     asset_id,
//                     pb2_num:pb2_num, 
//                     pb2_bias: pb2_bias,
//                     pb1_num: pb1_num, 
//                     pb1_bias: pb1_bias,
//                     day_type:daily_bias,
//                     reliable_total_attempts: total.round(),
//                     reliable_success_count: success.round(),
//                     p_day_type_conditional: p_conditional_rounded, 
//                 }
//             })
//             .collect();
            
//         Ok(bar_day_type_2nd_order)
//     }





//     /// Calculates the TCS Continuation Score (Raw Probability P(CS Continues Bias | PS2, PS1)).
//     pub async  fn calculate_tcs_continuation_score(
//         &self,
//         session_contexts: &[SessionContextData],
//         intervals: &[LookbackInterval],
//         //min_attempts: f64,
//     ) -> Result<Vec<TcsContinuationScore>> {
        
        
//         let cont_contexts: Vec<SessionContinuationContext> = session_contexts.iter()
//             .cloned()
//             .map(|s| SessionContinuationContext { inner: s })
//             .collect();

//         let results = aggregate_weighted_counts(
//             &cont_contexts, 
//             intervals, 
//             10.0
//         );

//         let tcs_cnntinuation: Vec<TcsContinuationScore> = results.into_iter()
//             .filter(|((_, _, is_cont), _)| *is_cont) 
//             .map(|(key, (success, total))| {

//                 let (asset_id, pattern_key, _) = key;
//                 let (ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name) = pattern_key;

//                 let p_continuation = success / total; 
//                 let p_continuation_rounded = round_to_decimal_places(p_continuation,4);
                
//                 TcsContinuationScore {
//                     asset_id,
//                     ps2_bias,
//                     ps1_bias,
//                     ps2_name, 
//                     ps1_name, 
//                     p_continuation:p_continuation_rounded
//                 }
//             })
//             .collect();
//         Ok(tcs_cnntinuation)
//     }


//     // Calcuation the Transitional Conditinal Score (TCS) Probability for a give Cs P(Cs_Transition_Conditional/Cs_base_rate)
//     pub async fn calculate_tcs_scores(
//         &self,
//         input_data: TcsData,
//     ) -> Result<Vec<Tcs2ndOrder>> {
        
//         if input_data.transitions.is_empty() {
//             return Ok(vec![]);
//         }

        
//         let base_rate_map: HashMap<(String, String), f64> = input_data.base_rates.into_iter()
//             .map(|r| ((r.cs_name, r.cs_bias), r.p_base))
//             .collect();

//         let tcs_scores: Vec<Tcs2ndOrder> = input_data.transitions.into_iter()
//             .filter_map(|t| {

//                 let p_base = *base_rate_map.get(&(t.cs_name.clone(), t.cs_bias.clone())).unwrap_or(&0.0001); 

//                 if p_base <= 0.0 {
//                     return None;
//                 }

//                 // TCS Score = P(Conditional) / P(Base)
//                 let tcs_score_raw = t.p_transition_conditional / p_base;
//                 let tcs_score_rounded = round_to_decimal_places(tcs_score_raw, 2);

//                 Some(Tcs2ndOrder {
//                     asset_id: t.asset_id,
//                     ps2_name: t.ps2_name,
//                     ps2_bias: t.ps2_bias,
//                     ps1_name: t.ps1_name,
//                     ps1_bias: t.ps1_bias,
//                     cs_name: t.cs_name,
//                     cs_bias: t.cs_bias,
//                     p_transition_conditional: t.p_transition_conditional, 
//                     p_cs_base: p_base, 
//                     tcs_score: tcs_score_rounded, 
//                 })
//             })
//             .collect();

//         Ok(tcs_scores)
//     }


//      pub async fn calculate_btcs_scores(
//         &self,
//         input_data: BtcsInputData,
//     ) -> Result<Vec<Btcs2ndOrder>> {
        
//         if input_data.transitions.is_empty() {
//             return Ok(vec![]);
//         }

        
//         let base_rate_map: HashMap<String, f64> = input_data.base_rates.into_iter()
//             .map(|r| (r.cb_bias, r.p_base))
//             .collect();

//         let btcs_scores: Vec<Btcs2ndOrder> = input_data.transitions.into_iter()
//             .filter_map(|t| {

//                 let p_base = *base_rate_map.get(&t.cb_bias).unwrap_or(&0.0001); 

//                 if p_base <= 0.0 {
//                     return None;
//                 }

//                 // TCS Score = P(Conditional) / P(Base)
//                 let bcs_score_raw = t.p_transition_conditional / p_base;
//                 let bcs_score_rounded = round_to_decimal_places(bcs_score_raw, 2);

//                 Some(Btcs2ndOrder {
//                     asset_id: t.asset_id,
//                     pb2_num: t.pb2_num,
//                     pb2_bias: t.pb2_bias,
//                     pb1_num: t.pb1_num,
//                     pb1_bias: t.pb1_bias,
//                     cb_num: t.cb_num,
//                     cb_bias: t.cb_bias,
//                     p_transition_conditional: t.p_transition_conditional, 
//                     p_cb_base: p_base, 
//                     btcs_score: bcs_score_rounded, 
//                 })
//             })
//             .collect();

//         Ok(btcs_scores)
//     }

//     pub async fn calculate_stcs_scores(
//     &self,
//     input_data: StcsInputData,
// ) -> Result<Vec<SessionToBarStcs>> {
    
//     if input_data.conditionals.is_empty() {
//         return Ok(vec![]);
//     }

//     // Create a lookup map using (BarNumber, Bias) as the key
//     // This ensures Bar 1 Bullish uses a different base rate than Bar 3 Bullish
//     let base_rate_map: HashMap<(i32, String), f64> = input_data.base_rates.into_iter()
//         .filter_map(|r| {
//             if let Some(num) = r.cb_num {
//                 Some(((num, r.cb_bias), r.p_base))
//             } else {
//                 None
//             }
//         })
//         .collect();

//     let stcs_results: Vec<SessionToBarStcs> = input_data.conditionals.into_iter()
//         .filter_map(|t| {
//             // Lookup base rate for the specific bar and bias predicted
//             let key = (t.target_bar_num, t.target_bar_bias.clone());
//             let p_base = *base_rate_map.get(&key).unwrap_or(&0.0001); 

//             if p_base <= 0.0 {
//                 return None;
//             }

//             // STCS Score = P(Conditional) / P(Base)
//             let lift_raw = t.probability / p_base;
//             let lift_rounded = round_to_decimal_places(lift_raw, 2);

//             Some(SessionToBarStcs {
//                 asset_id: t.asset_id,
//                 ps2_name: t.ps2_name,
//                 ps2_bias: t.ps2_bias,
//                 ps1_name: t.ps1_name,
//                 ps1_bias: t.ps1_bias,
//                 target_bar_num: t.target_bar_num,
//                 target_bar_bias: t.target_bar_bias,
//                 probability: t.probability,
//                 p_bar_base: p_base,
//                 stcs_score: lift_rounded,
//             })
//         })
//         .collect();

//     Ok(stcs_results)
// }

//     // Calcuation the Preciditive Confidence Score (PCS) Probability for a give Day, given P(DayType_Condtional/Day_base_rate)
//     pub async fn calculate_pcs_scores(
//         &self,
//         input_data: PcsInputData,
//     ) -> Result<Vec<PcsScore2ndOrder>> { 
        
//         if input_data.conditionals.is_empty() {
//             return Ok(vec![]);
//         }

        
//         let base_rate_map: HashMap<String, f64> = input_data.base_rates.into_iter()
//             .map(|r| (r.daily_bias, r.p_day_type_base))
//             .collect();

        
//         let pcs_scores: Vec<PcsScore2ndOrder> = input_data.conditionals.into_iter()
//             .filter_map(|p| { 
                
                
//                 let p_base = *base_rate_map.get(&p.day_type).unwrap_or(&0.0001); 

//                 if p_base <= 0.0 {
//                     return None;
//                 }
                
//                 // PCS Score = P(Conditional) / P(Base)
//                 let pcs_score_raw = p.p_day_type_conditional / p_base;
//                 let pcs_rounded = round_to_decimal_places(pcs_score_raw, 2);
                
//                 Some(PcsScore2ndOrder {
//                     asset_id: p.asset_id,
//                     ps2_name: p.ps2_name,
//                     ps2_bias: p.ps2_bias,
//                     ps1_name: p.ps1_name,
//                     ps1_bias: p.ps1_bias,
//                     daily_bias: p.day_type,
//                     p_day_type_conditional: p.p_day_type_conditional,
//                     p_day_type_base: p_base,
//                     pcs_score: pcs_rounded,
//                 })
//             })
//             .collect();
            
//         Ok(pcs_scores)
//     }

//      pub async fn calculate_bpcs_scores(
//         &self,
//         input_data: BpcsInputData,
//     ) -> Result<Vec<BpcsScore2ndOrder>> { 
        
//         if input_data.conditionals.is_empty() {
//             return Ok(vec![]);
//         }

        
//         let base_rate_map: HashMap<String, f64> = input_data.base_rates.into_iter()
//             .map(|r| (r.daily_bias, r.p_day_type_base))
//             .collect();

        
//         let bpcs_scores: Vec<BpcsScore2ndOrder> = input_data.conditionals.into_iter()
//             .filter_map(|p| { 
                
                
//                 let p_base = *base_rate_map.get(&p.day_type).unwrap_or(&0.0001); 

//                 if p_base <= 0.0 {
//                     return None;
//                 }
                
//                 // PCS Score = P(Conditional) / P(Base)
//                 let pcs_score_raw = p.p_day_type_conditional / p_base;
//                 let pcs_rounded = round_to_decimal_places(pcs_score_raw, 2);
                
//                 Some(BpcsScore2ndOrder {
//                     asset_id: p.asset_id,
//                     pb2_num: p.pb2_num,
//                     pb2_bias: p.pb2_bias,
//                     pb1_num: p.pb1_num,
//                     pb1_bias: p.pb1_bias,
//                     daily_bias: p.day_type,
//                     p_day_type_conditional: p.p_day_type_conditional,
//                     p_day_type_base: p_base,
//                     pcs_score: pcs_rounded,
//                 })
//             })
//             .collect();
            
//         Ok(bpcs_scores)
//     }

//     pub async fn calculate_session_to_bar_2nd_order(
//     &self,
//     session_contexts: &[SessionContextData],
//     eight_hour_contexts: &[EightContextData],
//     intervals: &[LookbackInterval],
// ) -> Result<Vec<SessionToBarMetrics>> {

//     // 1. Group and Sort Bars by Asset and Timestamp
//     let mut bar_map: HashMap<String, Vec<EightContextData>> = HashMap::new();
//     for b in eight_hour_contexts {
//         bar_map.entry(b.asset_id.clone()).or_default().push(b.clone());
//     }
//     for list in bar_map.values_mut() {
//         list.sort_by_key(|b| b.session_end_ts);
//     }

//     // 2. Map Sessions to the correct "Future" Bar
//     // 2. Perform Temporal Join with Strict Forward Logic
//     let joined_contexts: Vec<SessionToBar2ndOrderContext> = session_contexts.iter()
//         .filter_map(|s| {
//             let asset_bars = bar_map.get(&s.asset_id)?;

//             // PS1 is the session that just finished. 
//             // We want the Bar that follows PS1.
//             let target_num = match s.ps1_name.as_deref() {
//                 Some("NYPM") => 1, // NYPM ends at 18:00 -> Next is Bar 1
//                 Some("AS")   => 2, // AS ends at 02:00   -> Next is Bar 2
//                 Some("LN")   => 3, // LN ends at 08:00   -> Next is Bar 3
//                 Some("NYAM") => 3, // NYAM ends at 12:00 -> Next is Bar 3
//                 Some("NYL")  => 3, // NYL ends at 14:00  -> Next is Bar 3
//                 _ => return None,
//             };

//             // Now find the ACTUAL bar object for that number 
//             // that is closest in time to our current row.
//             let target = asset_bars.iter()
//                 .filter(|b| b.cb_num == Some(target_num))
//                 // We want the bar that ends AFTER the session we are looking at.
//                 .find(|b| b.session_end_ts > s.session_end_ts)?;

//             Some(SessionToBar2ndOrderContext {
//                 inner: s.clone(),
//                 target_bar: target.clone(),
//             })
//         })
//         .collect();

//     if joined_contexts.is_empty() {
//         return Ok(Vec::new());
//     }

//     // 3. Run the Weighted Aggregator
//     let results = aggregate_weighted_counts(&joined_contexts, intervals, 10.0);

//     // 4. Parse the "BarNum:Bias" string back into discrete fields for the final result
//     let metrics: Vec<SessionToBarMetrics> = results.into_iter()
//         .map(|(key, (success, total))| {
//             let (asset_id, pattern_key, outcome_str) = key; 
//             let (ps2_n, ps2_b, ps1_n, ps1_b) = pattern_key; 
            
//             let parts: Vec<&str> = outcome_str.split(':').collect();
//             let bar_num = parts.get(0).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
//             let bar_bias = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
            
//             SessionToBarMetrics {
//                 asset_id,
//                 ps2_name: ps2_n, ps2_bias: ps2_b,
//                 ps1_name: ps1_n, ps1_bias: ps1_b,
//                 target_bar_num: bar_num,
//                 target_bar_bias: bar_bias,
//                 total_attempts: total.round(),
//                 success_count: success.round(),
//                 probability: round_to_decimal_places(success / total, 4), 
//             }
//         })
//         .collect();
            
//     Ok(metrics)
// }

    

//     pub async fn calculate_high_low_session_2nd_order(
//         &self,
//         session_contexts: &[SessionContextData],
//         daily_extremes: &[DailyContextData], 
//         intervals: &[LookbackInterval],
//         min_attempts: f64,
//     ) -> Result<Vec<HighLowSession2ndOrder>> {
        
    
//         let high_low_contexts = prepare_high_low_contexts(session_contexts, daily_extremes);
        
//         if high_low_contexts.is_empty() {
//             return Ok(vec![]);
//             //return Vec::new();
//         }

        
//         let results = aggregate_weighted_counts(
//             &high_low_contexts, 
//             intervals, 
//             min_attempts
//         );

//         let session_extemes: Vec<HighLowSession2ndOrder> = results.into_iter()
//             .map(|(key, (success, total))| {
                
//                 let (asset_id, pattern_key, extreme_session_name) = key;
//                 let (ps2_name, ps2_bias, ps1_name, ps1_bias, session_extreme) = pattern_key;

//                 let p_conditional = success / total;
//                 let p_conditional_rounded = round_to_decimal_places(p_conditional, 4);

//                 HighLowSession2ndOrder {
//                     asset_id,
//                     ps2_bias,
//                     ps1_bias,
//                     ps2_name,
//                     ps1_name, 
//                     session_extreme,
//                     extreme_session_name,
//                     reliable_total_attempts: total.round(),
//                     reliable_success_count: success.round(),
//                     p_extreme_session_conditional: p_conditional_rounded,
//                 }
//             })
//             .collect();
//         Ok(session_extemes)
//     }

//     // --- Daily Metrics Pipeline Starts Here ----

//     // Calcuate the 3rd Order Probability of Daily Bias PD2->PD1->Dow1 ->C_D_Bias
//     pub async fn calculate_daily_3rd_order_conditional(
//         &self,
//         daily_contexts: &[DailyContextData], 
//         intervals: &[LookbackInterval],
//         min_attempts: f64,
//     ) -> Result<Vec<DayOutcome3rdOrder>> { 

        
//         let daily_3rd_order_contexts = prepare_daily_3rd_order_contexts(daily_contexts);

//         if daily_3rd_order_contexts.is_empty() {
//             return Ok(Vec::new());
//         }

        
//         let results = aggregate_weighted_counts(
//             &daily_3rd_order_contexts, 
//             intervals, 
//             min_attempts
//         );

        
//         let day_type_3rd_order: Vec<DayOutcome3rdOrder> = results.into_iter()
//             .map(|(key, (success, total))| {
                
//                 let (asset_id, pattern_key, daily_bias) = key; 
//                 let (pd2_bias, pd1_bias, pd1_dow) = pattern_key; 
                
//                 let p_conditional = success / total;
//                 let p_conditional_rounded = round_to_decimal_places(p_conditional, 4);
                
                
//                 DayOutcome3rdOrder {
//                     asset_id,
//                     pd2_bias,
//                     pd1_bias,
//                     pd1_dow,
//                     c_day_bias: daily_bias,
//                     reliable_total_attempts: total.round(),
//                     reliable_success_count: success.round(),
//                     p_day_bias_conditional: p_conditional_rounded,
//                 }
//             })
//             .collect();
            
//         Ok(day_type_3rd_order)
//     }

//     // --- Calculate Daily Confidence Score ---- 
//     pub async fn calculate_dcs_scores(
//         &self,
//         input_data: DcsInputData,
//     ) -> Result<Vec<Dcs3rdOrder>> { 
        
//         if input_data.conditionals.is_empty() {
//             return Ok(vec![]);
//         }

//         // 1. Map Daily Base Rates for fast lookup: { daily_bias -> p_day_type_base }
//         let base_rate_map: HashMap<String, f64> = input_data.base_rates.into_iter()
//             .map(|r| (r.daily_bias, r.p_day_type_base))
//             .collect();

//         // 2. Iterate and calculate the final score
//         let dcs_scores: Vec<Dcs3rdOrder> = input_data.conditionals.into_iter()
//             .filter_map(|d| { 
                
//                 // Get the base rate for the Current Day Bias
//                 let p_base = *base_rate_map.get(&d.c_day_bias).unwrap_or(&0.0001); 

//                 if p_base <= 0.0 {
//                     return None;
//                 }

//                 // DCS Score = P(Conditional) / P(Base)
//                 let dcs_score_raw = d.p_day_bias_conditional / p_base;

                
//                 //d.p_day_type_base = round_to_decimal_places(p_base, 4); 
//                 let dcs_score_rounded = round_to_decimal_places(dcs_score_raw, 2);

//                 Some(Dcs3rdOrder { 
//                     asset_id: d.asset_id,
//                     pd2_bias: d.pd2_bias,
//                     pd1_bias: d.pd1_bias,
//                     pd1_dow: d.pd1_dow, 
//                     c_day_bias: d.c_day_bias,
//                     p_day_bias_conditional: d.p_day_bias_conditional, 
//                     p_day_type_base: p_base, 
//                     dcs_score: dcs_score_rounded 
//                     })
//             })
//             .collect();

//         Ok(dcs_scores)
//     }


//     pub async fn calculate_daily_trend_continuation_rate(
//         &self,
//         daily_contexts: &[DailyContextData], 
//         intervals: &[LookbackInterval],
//         min_attempts: f64,
//     ) -> Result<Vec<DailyTrendContinuationRate>> { 

//         // 1. Prepare contexts (no change needed here)
//         let dtcr_contexts = prepare_dtcr_contexts(daily_contexts);
//         // ... checks ...

//         // 2. Aggregate Weighted Counts (results now has the composite outcome key)
//         let results = aggregate_weighted_counts(
//             &dtcr_contexts, 
//             intervals, 
//             min_attempts
//         );

//         // 3. Process Results
//         let dtcr_rates: Vec<DailyTrendContinuationRate> = results.into_iter()
//             .filter_map(|(key, (success, total))| {
//                 // Key format: (asset_id, (pd2, pd1, dow), (c_day_bias, continuation_outcome))
//                 let (asset_id, pattern_key, outcome_key) = key; 
                
//                 let (pd2_bias, pd1_bias, pd1_dow) = pattern_key; 
//                 let (c_day_bias, continuation_outcome) = outcome_key; // <-- Accessing c_day_bias!
                
//                 // We only care about the Continuation probability for the final metric table
//                 if continuation_outcome != "Continuation" {
//                     return None;
//                 }
                
//                 let p_conditional = success / total;
//                 let p_conditional_rounded = round_to_decimal_places(p_conditional, 4);
                
//                 // The final struct includes the c_day_bias
//                 Some(DailyTrendContinuationRate {
//                     asset_id,
//                     pd2_bias,
//                     pd1_bias,
//                     pd1_dow,
//                     c_day_bias, // <-- Now correctly derived from the outcome key!
//                     reliable_total_attempts: total.round(),
//                     reliable_success_count: success.round(),
//                     p_continuation_conditional: p_conditional_rounded,
//                 })
//             })
//             .collect();
            
//         Ok(dtcr_rates)
//     }


    
// }





// pub struct DailyOutcomeValue {
//     pub day_outcome: String,
// }

// pub fn build_daily_outcome_map(
//     daily_outcomes: &Vec<DailyOutcome>
// ) -> HashMap<(NaiveDate, String), DailyOutcomeValue> {

//     daily_outcomes.iter()
//         .map(|d| {
//             let key = (d.trading_date, d.asset_id.clone());
//             let value = DailyOutcomeValue { 
//                 day_outcome: d.day_bias.clone() 
//             };
//             (key, value)
//         })
//         .collect()
// }




























// // Assume this is part of a function generating a single row (vector)
// fn one_hot_encode_bias(bias: &str) -> HashMap<String, i32> {
//     // Define all possible states
//     let all_biases = vec![
//         "Bullish", "Bearish", "Bullish_Reversal", "Bearish_Reversal", 
//         "Failed_Bearish", "Failed_Bullish", "Pure_Consolidation",
//         // Add more if needed, e.g., "Failed_Bearish"
//     ]; 
    
//     let mut encoded_map: HashMap<String, i32> = HashMap::new();
    
//     for state in all_biases {
//         // Feature name in the ML vector
//         let feature_name = format!("ps1_bias__{}", state);
        
//         // Set to 1 if the current bias matches the state, 0 otherwise
//         let value = if bias == state { 1 } else { 0 };
        
//         encoded_map.insert(feature_name, value);
//     }
//     encoded_map
// }

// // In your main feature generation loop, you would call this and append the results:
// // let ps1_bias_features = one_hot_encode_bias(session_context.ps1_bias.as_deref().unwrap_or("Unknown"));
// // // Append the keys and values of ps1_bias_features to the final vector row.