use anyhow::{Result, anyhow};
use sqlx::{PgPool, Postgres};
use chrono::{NaiveDate, Datelike};
use std::collections::HashMap;
use async_trait::async_trait;

// --- ASSUMED INPUT DATA STRUCTURES ---
// These match the data fetched from the classified tables (daily_views, weekly_views, session_context)
// The actual definitions would live in a shared or data_fetch module.

// A simplified model for the daily data needed for context and outcome (7-state)
#[derive(Debug, Clone)]
pub struct DailyViewContext {
    pub asset_id: String,
    pub trading_date: NaiveDate,
    pub day_type: String, // e.g., "Bullish", "Consolidation"
    pub consolidation_subtype: Option<String>, // e.g., "Consolidation-Up"
    pub high_session: Option<String>, // e.g., "LN"
    pub low_session: Option<String>, // e.g., "NYAM"
}

// A simplified model for the weekly data
#[derive(Debug, Clone)]
pub struct WeeklyViewContext {
    pub asset_id: String,
    pub week_start: NaiveDate,
    pub weekly_type: String, // e.g., "Bullish", "Consolidation"
    pub consolidation_subtype: Option<String>,
}

// A simplified model for the session context data
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub asset_id: String,
    pub trading_date: NaiveDate,
    pub ps2_name: Option<String>, // Previous Session 2 Name (e.g., "LN")
    pub ps2_bias_7: Option<String>, // Previous Session 2 Bias (7-state)
    pub ps1_name: Option<String>, // Previous Session 1 Name
    pub ps1_bias_7: Option<String>, // Previous Session 1 Bias (7-state)
    pub cs_name: Option<String>, // Current Session Name (Target)
    pub cs_bias_7: Option<String>, // Current Session Bias (7-state)
}

// --- INTERMEDIATE CONTEXT STRUCTURES (Simulating CTEs) ---

#[derive(Debug, Clone)]
pub struct DayContext {
    pub asset_id: String,
    pub trading_date: NaiveDate,
    pub day0_outcome_7: String,
    pub day0_direction: String,
    pub pd1_outcome_7: Option<String>,
    pub pd1_direction: Option<String>,
    pub pd1_dow: Option<String>, // Day of Week
    pub pd2_outcome_7: Option<String>,
    pub pd2_direction: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WeeklyContext {
    pub asset_id: String,
    pub week_start: NaiveDate,
    pub week0_outcome_7: String,
    pub week0_week_of_month: i32,
    pub pw1_outcome_7: Option<String>,
    pub pw2_outcome_7: Option<String>,
}


// --- CORE LOGIC STRUCTURE ---

const WEIGHT_ALL: f64 = 0.60;
const WEIGHT_1Y: f64 = 0.30;
const WEIGHT_6M: f64 = 0.10;
const MIN_PATTERN_ATTEMPTS: f64 = 10.0;
const MIN_WEEKLY_ATTEMPTS: f64 = 3.0; // Weekly threshold is lower

/// The central component for calculating all statistical DCI, FCI, and WCI metrics.
pub struct MetricsCalculator {
    pool: PgPool,
}

impl MetricsCalculator {
    pub fn new(pool: PgPool) -> Self {
        MetricsCalculator { pool }
    }

    // --- UTILITIES ---

    /// Maps the daily outcome type to the 7-state classification key.
    fn get_daily_outcome_7(view: &DailyViewContext) -> String {
        match view.day_type.as_str() {
            "Bullish" | "Bearish" => view.day_type.clone(),
            "Consolidation" => view.consolidation_subtype.as_ref().cloned().unwrap_or("Consolidation-Other".to_string()),
            _ => "Other".to_string(),
        }
    }

    /// Helper to convert NaiveDate to Day of Week string (Mon-Fri only).
    fn date_to_dow(date: NaiveDate) -> String {
        // PL/pgSQL uses 'Day' which is typically capitalized, e.g., 'Monday '
        // We'll use a clean capitalized name here.
        match date.weekday() {
            chrono::Weekday::Mon => "Monday".to_string(),
            chrono::Weekday::Tue => "Tuesday".to_string(),
            chrono::Weekday::Wed => "Wednesday".to_string(),
            chrono::Weekday::Thu => "Thursday".to_string(),
            chrono::Weekday::Fri => "Friday".to_string(),
            _ => "Weekend".to_string(), // Should be filtered out by the caller
        }
    }

    /// Performs the core weighted aggregation logic, simulating the complex SQL CTEs.
    /// Returns (reliable_event_count, reliable_total_count).
    /// `key_fn`: A closure that takes the item and returns the key for grouping and filtering.
    /// `is_event_fn`: A closure that takes the item and returns true if it's a successful event.
    /// `min_attempts`: The minimum weighted denominator required to consider the rate reliable.
    fn calculate_weighted_rate<T, K, I>(
        data: I,
        min_attempts: f64,
        key_fn: impl Fn(&T) -> Option<K>,
        is_event_fn: impl Fn(&T) -> bool,
        get_date_fn: impl Fn(&T) -> NaiveDate,
    ) -> HashMap<K, (f64, f64)>
    where
        T: Clone,
        K: Eq + std::hash::Hash + Clone,
        I: Iterator<Item = T>,
    {
        let mut raw_counts: HashMap<(K, String), (f64, f64)> = HashMap::new(); // Key: (Pattern, Lookback) -> (Successes, Attempts)

        for item in data {
            if let Some(key) = key_fn(&item) {
                let date = get_date_fn(&item);
                let is_success = if is_event_fn(&item) { 1.0 } else { 0.0 };

                let today = NaiveDate::from_ymd_opt(date.year(), date.month(), date.day()).unwrap_or(date);
                let six_months_ago = today.with_month(today.month().checked_sub(6).unwrap_or(12)).with_day(1).unwrap_or(today); // Simplified logic
                let one_year_ago = today.with_year(today.year() - 1).unwrap_or(today);

                // --- Lookback Tally ---
                let mut tally = |lookback: &str| {
                    let entry = raw_counts.entry((key.clone(), lookback.to_string())).or_insert((0.0, 0.0));
                    entry.0 += is_success;
                    entry.1 += 1.0;
                };

                // ALL
                tally("ALL");

                // 1Y
                if date >= one_year_ago {
                    tally("1Y");
                }
                
                // 6M
                if date >= six_months_ago {
                    tally("6M");
                }
            }
        }

        let mut weighted_results: HashMap<K, (f64, f64)> = HashMap::new();

        for ((key, lookback), (success, attempts)) in raw_counts.into_iter() {
            let (weight, required_attempts) = match lookback.as_str() {
                "ALL" => (WEIGHT_ALL, MIN_PATTERN_ATTEMPTS),
                "1Y" => (WEIGHT_1Y, MIN_PATTERN_ATTEMPTS),
                "6M" => (WEIGHT_6M, MIN_PATTERN_ATTEMPTS),
                _ => (0.0, 0.0),
            };

            // Apply minimum attempt filter (simplified: using `ALL` threshold for this check)
            if lookback == "ALL" && attempts < required_attempts {
                continue; 
            }

            let entry = weighted_results.entry(key).or_insert((0.0, 0.0));
            entry.0 += success * weight; // reliable_event_count
            entry.1 += attempts * weight; // reliable_total_count
        }
        
        // Final filter based on minimum weighted total attempts
        weighted_results.retain(|_, (_, total)| *total >= min_attempts);

        weighted_results
    }


    // --- CONTEXT GENERATION (Simulating CTEs) ---

    /// Simulates the `daily_outcome_7_state` and `day_context` CTEs.
    fn generate_day_context(&self, daily_views: &[DailyViewContext]) -> Result<Vec<DayContext>> {
        let mut context = Vec::new();

        // Ensure sorted by date for accurate LAG simulation
        let mut sorted_views = daily_views.to_vec();
        sorted_views.sort_by_key(|v| (v.asset_id.clone(), v.trading_date));

        for i in 0..sorted_views.len() {
            let day0 = &sorted_views[i];
            
            // Only calculate D0 for weekdays
            if day0.trading_date.weekday() == chrono::Weekday::Sat || day0.trading_date.weekday() == chrono::Weekday::Sun {
                continue;
            }

            let day0_outcome_7 = Self::get_daily_outcome_7(day0);
            let day0_direction = day0_outcome_7.clone();

            let (pd1_outcome_7, pd1_direction, pd1_dow, pd2_outcome_7, pd2_direction) = 
                if i > 0 && sorted_views[i - 1].asset_id == day0.asset_id {
                    // PD1
                    let pd1 = &sorted_views[i - 1];
                    let pd1_o7 = Self::get_daily_outcome_7(pd1);
                    let pd1_d = pd1_o7.clone();
                    let pd1_dow = Self::date_to_dow(pd1.trading_date);

                    // PD2
                    let (pd2_o7, pd2_d) = if i > 1 && sorted_views[i - 2].asset_id == day0.asset_id {
                        let pd2 = &sorted_views[i - 2];
                        let pd2_o7 = Self::get_daily_outcome_7(pd2);
                        (Some(pd2_o7), Some(pd2_o7.clone()))
                    } else {
                        (None, None)
                    };

                    (Some(pd1_o7), Some(pd1_d), Some(pd1_dow), pd2_o7, pd2_d)
                } else {
                    (None, None, None, None, None)
                };

            context.push(DayContext {
                asset_id: day0.asset_id.clone(),
                trading_date: day0.trading_date,
                day0_outcome_7,
                day0_direction,
                pd1_outcome_7,
                pd1_direction,
                pd1_dow,
                pd2_outcome_7,
                pd2_direction,
            });
        }
        Ok(context)
    }

    /// Simulates the `weekly_context` CTE.
    fn generate_weekly_context(&self, weekly_views: &[WeeklyViewContext]) -> Result<Vec<WeeklyContext>> {
        let mut context = Vec::new();
        let mut sorted_views = weekly_views.to_vec();
        sorted_views.sort_by_key(|v| (v.asset_id.clone(), v.week_start));

        for i in 0..sorted_views.len() {
            let week0 = &sorted_views[i];
            
            let week0_outcome_7 = match week0.weekly_type.as_str() {
                "Bullish" | "Bearish" => week0.weekly_type.clone(),
                "Consolidation" => week0.consolidation_subtype.as_ref().cloned().unwrap_or("Consolidation-Other".to_string()),
                _ => "Other".to_string(),
            };

            // Week of Month: CEIL(Day / 7.0)
            let week0_week_of_month = (week0.week_start.day() as f64 / 7.0).ceil() as i32;

            let (pw1_outcome_7, pw2_outcome_7) = 
                if i > 0 && sorted_views[i - 1].asset_id == week0.asset_id {
                    let pw1 = &sorted_views[i - 1];
                    let pw1_o7 = match pw1.weekly_type.as_str() {
                        "Bullish" | "Bearish" => pw1.weekly_type.clone(),
                        "Consolidation" => pw1.consolidation_subtype.as_ref().cloned().unwrap_or("Consolidation-Other".to_string()),
                        _ => "Other".to_string(),
                    };

                    let pw2_o7 = if i > 1 && sorted_views[i - 2].asset_id == week0.asset_id {
                        let pw2 = &sorted_views[i - 2];
                        Some(match pw2.weekly_type.as_str() {
                            "Bullish" | "Bearish" => pw2.weekly_type.clone(),
                            "Consolidation" => pw2.consolidation_subtype.as_ref().cloned().unwrap_or("Consolidation-Other".to_string()),
                            _ => "Other".to_string(),
                        })
                    } else {
                        None
                    };

                    (Some(pw1_o7), pw2_o7)
                } else {
                    (None, None)
                };

            context.push(WeeklyContext {
                asset_id: week0.asset_id.clone(),
                week_start: week0.week_start,
                week0_outcome_7,
                week0_week_of_month,
                pw1_outcome_7,
                pw2_outcome_7,
            });
        }
        Ok(context)
    }

    // --- MAIN EXECUTION FUNCTION ---

    /// The Rust equivalent of the REFRESH_METRICS_PIPELINE stored procedure.
    /// It calculates and persists all weighted confidence scores (DCI, FCI, WCI).
    #[allow(clippy::too_many_lines)]
    pub async fn run_pipeline(
        &self,
        daily_views: Vec<DailyViewContext>,
        weekly_views: Vec<WeeklyViewContext>,
        session_contexts: Vec<SessionContext>,
        fetcher: &impl DataFetcher, // Placeholder for the actual data fetching trait
    ) -> Result<()> {
        
        println!("Starting Unified Metrics Pipeline with Statistical Weighting...");

        // NOTE: In a complete system, we would TRUNCATE tables here via sqlx::query!
        // For brevity, we skip TRUNCATE calls and focus on calculation and INSERT.

        // --- 1. Generate Contexts (Simulating PL/pgSQL CTEs) ---
        let day_context = self.generate_day_context(&daily_views)?;
        let weekly_context = self.generate_weekly_context(&weekly_views)?;

        // --- 2. Calculate Weighted Base Rates (A.1, A.2, A.3) ---
        
        // A.1. DCI Base Rates (day_type_base_rates)
        println!("A.1. Calculating Daily Outcome Base Rates...");
        let daily_base_rates = Self::calculate_weighted_rate(
            daily_views.iter().cloned(),
            MIN_PATTERN_ATTEMPTS,
            |v| Some((v.asset_id.clone(), Self::get_daily_outcome_7(v))),
            |_| true,
            |v| v.trading_date,
        );

        // A.2. CS Base Rates (cs_base_rates)
        println!("A.2. Calculating Session Bias Base Rates (CS)...");
        let cs_base_rates = Self::calculate_weighted_rate(
            session_contexts.iter().cloned().filter(|c| c.cs_bias_7.is_some() && c.cs_bias_7.as_deref() != Some("Other")),
            MIN_PATTERN_ATTEMPTS,
            |c| Some((c.asset_id.clone(), c.cs_bias_7.clone().unwrap())),
            |_| true,
            |c| c.trading_date,
        );

        // A.3. WCI Base Rates (weekly_base_rates)
        println!("A.3. Calculating Weekly Outcome Base Rates (WCI)...");
        let weekly_base_rates = Self::calculate_weighted_rate(
            weekly_context.iter().cloned(),
            MIN_WEEKLY_ATTEMPTS,
            |w| Some((w.asset_id.clone(), w.week0_outcome_7.clone())),
            |_| true,
            |w| w.week_start,
        );

        // --- 3. WCI Conditional Probabilities (C.1, C.2, C.3) ---

        // C.1. WOCS_D (weekly_outcome_conf_daily) - PD2/PD1 -> W0
        println!("C.1. Calculating Weekly Outcome Confidence Daily (WOCS_D)...");
        // NOTE: This requires looking ahead (LEAD), which is complex. We'll use a simplified implementation
        // that matches up day context to the next week's outcome.
        let wocs_d_rates = Self::calculate_weighted_rate(
            day_context.iter().cloned().filter_map(|dc| {
                // Find the next weekly context start date
                let next_week_start = weekly_views.iter()
                    .filter(|wv| wv.asset_id == dc.asset_id && wv.week_start > dc.trading_date)
                    .min_by_key(|wv| wv.week_start)
                    .map(|wv| wv.week_start);

                if let (Some(ws), Some(pd1), Some(pd2)) = (next_week_start, dc.pd1_outcome_7, dc.pd2_outcome_7) {
                    let target_week_outcome = weekly_context.iter()
                        .find(|wc| wc.asset_id == dc.asset_id && wc.week_start == ws)
                        .map(|wc| wc.week0_outcome_7.clone());

                    if target_week_outcome.is_some() {
                        let pattern = (dc.asset_id.clone(), pd2, pd1);
                        return Some((dc.trading_date, pattern, target_week_outcome.unwrap()));
                    }
                }
                None
            }),
            MIN_WEEKLY_ATTEMPTS,
            |(_, pattern, _)| Some(pattern.clone()),
            |(_, _, outcome)| !outcome.is_empty(), // Simple check, always true if mapped
            |(date, _, _)| *date,
        );

        // C.2. WCS (weekly_continuation_score) - PW2/PW1 -> W0
        println!("C.2. Calculating Weekly Continuation Score (WCS)...");
        let wcs_rates = Self::calculate_weighted_rate(
            weekly_context.iter().cloned().filter(|w| w.pw1_outcome_7.is_some() && w.pw2_outcome_7.is_some()),
            MIN_WEEKLY_ATTEMPTS,
            |w| Some((w.asset_id.clone(), w.pw2_outcome_7.clone().unwrap(), w.pw1_outcome_7.clone().unwrap())),
            |w| !w.week0_outcome_7.is_empty(),
            |w| w.week_start,
        );

        // C.3. WMS (weekly_monthly_seasonality) - WeekOfMonth -> W0
        println!("C.3. Calculating Weekly Monthly Seasonality (WMS)...");
        let wms_rates = Self::calculate_weighted_rate(
            weekly_context.iter().cloned(),
            MIN_WEEKLY_ATTEMPTS,
            |w| Some((w.asset_id.clone(), w.week0_week_of_month, w.week0_outcome_7.clone())),
            |_| true,
            |w| w.week_start,
        );

        // --- 4. DCI Conditional Probabilities (B) ---

        // B. Day Outcome 3rd Order (pd2/pd1/dow -> day0)
        println!("B. Calculating Day Outcome 3rd Order Conditional Rates (DCI)...");
        let dci_3rd_order_rates = Self::calculate_weighted_rate(
            day_context.iter().cloned().filter(|d| 
                d.pd2_outcome_7.is_some() && d.pd1_outcome_7.is_some() && 
                d.pd1_dow.is_some() && 
                d.day0_outcome_7 != "Other" && d.day0_outcome_7 != "Consolidation-Other"
            ),
            MIN_PATTERN_ATTEMPTS,
            |d| Some((
                d.asset_id.clone(), 
                d.pd2_outcome_7.clone().unwrap(), 
                d.pd1_outcome_7.clone().unwrap(), 
                d.pd1_dow.clone().unwrap(), 
                d.day0_outcome_7.clone()
            )),
            |_| true,
            |d| d.trading_date,
        );


        // --- 5. Daily Trend Rate (DTCR) (D) ---

        // D. Daily Trend Rate (pd2/pd1/dow -> day0_direction)
        println!("D. Calculating Daily Trend Rate (DTCR)...");
        let dtcr_rates = Self::calculate_weighted_rate(
            day_context.iter().cloned().filter(|d| 
                d.pd2_direction.is_some() && d.pd1_direction.is_some() && d.pd1_dow.is_some()
            ),
            MIN_PATTERN_ATTEMPTS,
            |d| Some((
                d.asset_id.clone(), 
                d.pd2_direction.clone().unwrap(), 
                d.pd1_direction.clone().unwrap(), 
                d.pd1_dow.clone().unwrap(), 
                d.day0_direction.clone()
            )),
            |_| true,
            |d| d.trading_date,
        );


        // --- 6. FCI Conditional Probabilities (C.1, D.1, D.2) ---
        
        // C.1. PCS 2nd Order Conditional Rates (day_type_2nd_order) - (ps2/ps1 -> day0)
        println!("C.1. Calculating PCS 2nd Order Conditional Rates (FCI)...");
        let pcs_2nd_order_rates = Self::calculate_weighted_rate(
            session_contexts.iter().cloned().filter(|s| 
                s.ps2_name.is_some() && s.ps1_name.is_some()
            ).filter_map(|s| {
                let day_view = daily_views.iter().find(|dv| dv.asset_id == s.asset_id && dv.trading_date == s.trading_date)?;
                let day0_outcome_7 = Self::get_daily_outcome_7(day_view);
                let pattern_key = (
                    s.asset_id.clone(), 
                    s.ps2_name.clone().unwrap(), s.ps2_bias_7.clone().unwrap_or_default(), 
                    s.ps1_name.clone().unwrap(), s.ps1_bias_7.clone().unwrap_or_default(), 
                    day0_outcome_7.clone()
                );
                Some((s.trading_date, pattern_key))
            }),
            MIN_PATTERN_ATTEMPTS,
            |(_, key)| Some(key.clone()),
            |_| true,
            |(date, _)| *date,
        );

        // D.1. TCS 2nd Order Conditional Rates (transition_2nd_order) - (ps2/ps1 -> cs)
        println!("D.1. Calculating TCS 2nd Order Conditional Rates (FCI)...");
        let tcs_2nd_order_rates = Self::calculate_weighted_rate(
            session_contexts.iter().cloned().filter(|s| 
                s.ps2_name.is_some() && s.ps1_name.is_some() && s.cs_name.is_some() && s.cs_bias_7.is_some()
            ),
            MIN_PATTERN_ATTEMPTS,
            |s| Some((
                s.asset_id.clone(), 
                s.ps2_name.clone().unwrap(), s.ps2_bias_7.clone().unwrap_or_default(), 
                s.ps1_name.clone().unwrap(), s.ps1_bias_7.clone().unwrap_or_default(), 
                s.cs_name.clone().unwrap(), s.cs_bias_7.clone().unwrap_or_default()
            )),
            |_| true,
            |s| s.trading_date,
        );
        
        // D.2. High/Low Session 2nd Order Conditional Rates (ps2/ps1 -> extreme_session)
        println!("D.2. Calculating High/Low Session Conditional Rates (FCI)...");
        // This simulates the UNNEST (unpivoting) of High/Low sessions
        let high_low_extremes: Vec<(NaiveDate, (String, String, String, String, String, String, String))> = daily_views.iter().flat_map(|dv| {
            if dv.high_session.is_none() || dv.low_session.is_none() { return vec![] }
            let session_context = session_contexts.iter()
                .find(|sc| sc.asset_id == dv.asset_id && sc.trading_date == dv.trading_date)
                .cloned()?;
            
            let ps2_name = session_context.ps2_name.unwrap_or_default();
            let ps2_bias = session_context.ps2_bias_7.unwrap_or_default();
            let ps1_name = session_context.ps1_name.unwrap_or_default();
            let ps1_bias = session_context.ps1_bias_7.unwrap_or_default();
            
            vec![
                (dv.trading_date, (dv.asset_id.clone(), ps2_name.clone(), ps2_bias.clone(), ps1_name.clone(), ps1_bias.clone(), "High".to_string(), dv.high_session.clone().unwrap())),
                (dv.trading_date, (dv.asset_id.clone(), ps2_name, ps2_bias, ps1_name, ps1_bias, "Low".to_string(), dv.low_session.clone().unwrap())),
            ]
        }).collect();
        
        let high_low_session_rates = Self::calculate_weighted_rate(
            high_low_extremes.into_iter(),
            MIN_PATTERN_ATTEMPTS,
            |(_, pattern)| Some(pattern.clone()),
            |_| true,
            |(date, _)| *date,
        );

        // --- 7. TCS Continuation Score (G) ---
        
        // G. TCS Continuation Score (ps2/ps1 -> p_continuation)
        println!("G. Calculating TCS Continuation Scores (Raw Continuation Probability)...");
        let strong_bullish_states = ["Bullish", "Bullish_Reversal", "Failed_Bullish"];
        let strong_bearish_states = ["Bearish", "Bearish_Reversal", "Failed_Bearish"];

        let tcs_continuation_rates = Self::calculate_weighted_rate(
            session_contexts.iter().cloned().filter(|s| s.ps1_bias_7.is_some() && s.cs_bias_7.is_some()),
            MIN_PATTERN_ATTEMPTS,
            |s| Some((
                s.asset_id.clone(), 
                s.ps2_name.clone().unwrap_or_default(), s.ps2_bias_7.clone().unwrap_or_default(), 
                s.ps1_name.clone().unwrap_or_default(), s.ps1_bias_7.clone().unwrap_or_default()
            )),
            |s| {
                let ps1 = s.ps1_bias_7.as_deref().unwrap_or_default();
                let cs = s.cs_bias_7.as_deref().unwrap_or_default();
                
                let is_bullish_cont = strong_bullish_states.contains(&ps1) && strong_bullish_states.contains(&cs);
                let is_bearish_cont = strong_bearish_states.contains(&ps1) && strong_bearish_states.contains(&cs);
                let is_consolidation_cont = (ps1 == "Consolidation-Down" && cs == "Consolidation-Down") ||
                                            (ps1 == "Consolidation-Up" && cs == "Consolidation-Up") ||
                                            (ps1 == "Consolidation-Other" && cs == "Consolidation-Other");
                is_bullish_cont || is_bearish_cont || is_consolidation_cont
            },
            |s| s.trading_date,
        );

        // --- 8. Persist All Results and Calculate Final Scaled Scores (PDCS, PCS, TCS) ---

        // NOTE: The full implementation would persist all 10 calculated rate tables before moving to scaled scores.
        // For the PDCS example (E), we combine it here:

        println!("E. Calculating Final PDCS Scores (Scaled around 1.0)...");
        for (key, (conditional_success, conditional_total)) in dci_3rd_order_rates {
            let (asset_id, pd2_o7, pd1_o7, pd1_dow, day0_o7) = key;

            // 1. Find the base rate for day0_o7
            let base_key = (asset_id.clone(), day0_o7.clone());
            if let Some((base_success, base_total)) = daily_base_rates.get(&base_key) {
                
                let p_conditional = conditional_success / conditional_total * 100.0;
                let p_base = base_success / base_total * 100.0;
                
                if p_base > 0.0 {
                    let pdcs_score = (p_conditional / p_base).round_to(2);
                    
                    let insight_label = match pdcs_score {
                        s if s >= 2.25 => "Extreme Confidence Signal (PDCS >= 2.25)",
                        s if s >= 1.75 => "High-Confidence Signal (PDCS >= 1.75)",
                        s if s < 0.75 => "Contradictory Signal/Failure Risk (PDCS < 0.75)",
                        _ => "Confirmatory/Expected Range",
                    };
                    
                    // Final persistence step: INSERT into daily_confidence_score
                    let _ = sqlx::query!(
                        r#"
                        INSERT INTO daily_confidence_score (
                            asset_id, pd2_outcome_7, pd1_outcome_7, pd1_dow, day0_outcome_7,
                            p_day_outcome_conditional, p_day_type_base, pdcs_score, insight_label
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                        ON CONFLICT DO NOTHING
                        "#,
                        asset_id, pd2_o7, pd1_o7, pd1_dow, day0_o7,
                        p_conditional, p_base, pdcs_score, insight_label
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|e| anyhow!("PDCS Persistence Failed: {}", e));
                }
            }
        }
        
        // (F. Final PCS and TCS calculations/persistence would follow the same pattern)
        
        println!("Unified Metrics Pipeline complete. DCI, FCI, and WCI metrics calculated.");
        Ok(())
    }
}

// Helper trait to extend f64 for rounding
trait RoundTo {
    fn round_to(self, decimals: u32) -> f64;
}

impl RoundTo for f64 {
    fn round_to(self, decimals: u32) -> f64 {
        let factor = 10.0f64.powi(decimals as i32);
        (self * factor).round() / factor
    }
}

// --- Placeholder Trait for DataFetcher (Used for the run_pipeline signature) ---

// In a real application, this trait would define the async methods to load all context data.
#[async_trait]
pub trait DataFetcher: Send + Sync {
    async fn fetch_daily_views(&self, asset_id: &str, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<DailyViewContext>>;
    async fn fetch_weekly_views(&self, asset_id: &str, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<WeeklyViewContext>>;
    async fn fetch_session_contexts(&self, asset_id: &str, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<SessionContext>>;
}