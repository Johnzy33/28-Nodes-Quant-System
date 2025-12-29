use anyhow::{Result, anyhow};
use sqlx::{PgPool};
use chrono::NaiveDate;
use std::collections::HashMap;

// --- ASSUMED INPUT DATA STRUCTURES ---
// These are the classified and contextual data required for feature generation.

#[derive(Debug, Clone)]
pub struct DailyViewContext {
    pub asset_id: String,
    pub trading_date: NaiveDate,
    pub day_type: String, // e.g., "Bullish", "Consolidation"
    pub consolidation_subtype: Option<String>, // e.g., "Consolidation-Up"
    pub high_session: Option<String>, // e.g., "LN"
    pub low_session: Option<String>, // e.g., "NYAM"
}

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

#[derive(Debug, Clone)]
pub struct DayContext {
    pub asset_id: String,
    pub trading_date: NaiveDate,
    pub day0_outcome_7: String,
    pub pd1_outcome_7: Option<String>,
    pub pd1_dow: Option<String>, // Day of Week
    pub pd2_outcome_7: Option<String>,
}


// --- FEATURE GENERATOR STRUCTURE ---

const MIN_PATTERN_ATTEMPTS: u32 = 10; // Minimum required raw count for reliability

/// Generates raw conditional probability features for Machine Learning models.
pub struct FeatureGenerator {
    pool: PgPool,
}

impl FeatureGenerator {
    pub fn new(pool: PgPool) -> Self {
        FeatureGenerator { pool }
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
        match date.weekday() {
            chrono::Weekday::Mon => "Monday".to_string(),
            chrono::Weekday::Tue => "Tuesday".to_string(),
            chrono::Weekday::Wed => "Wednesday".to_string(),
            chrono::Weekday::Thu => "Thursday".to_string(),
            chrono::Weekday::Fri => "Friday".to_string(),
            _ => "Weekend".to_string(),
        }
    }

    /// Generates the Day Context (simulating LAG and DailyOutcome7State)
    fn generate_day_context(daily_views: &[DailyViewContext]) -> Result<Vec<DayContext>> {
        let mut context = Vec::new();
        let mut sorted_views = daily_views.to_vec();
        sorted_views.sort_by_key(|v| (v.asset_id.clone(), v.trading_date));

        for i in 0..sorted_views.len() {
            let day0 = &sorted_views[i];
            
            if day0.trading_date.weekday() == chrono::Weekday::Sat || day0.trading_date.weekday() == chrono::Weekday::Sun {
                continue;
            }

            let day0_outcome_7 = Self::get_daily_outcome_7(day0);

            let (pd1_outcome_7, pd1_dow, pd2_outcome_7) = 
                if i > 0 && sorted_views[i - 1].asset_id == day0.asset_id {
                    let pd1 = &sorted_views[i - 1];
                    let pd1_o7 = Self::get_daily_outcome_7(pd1);
                    let pd1_dow = Self::date_to_dow(pd1.trading_date);

                    let pd2_o7 = if i > 1 && sorted_views[i - 2].asset_id == day0.asset_id {
                        Some(Self::get_daily_outcome_7(&sorted_views[i - 2]))
                    } else {
                        None
                    };

                    (Some(pd1_o7), Some(pd1_dow), pd2_o7)
                } else {
                    (None, None, None)
                };

            context.push(DayContext {
                asset_id: day0.asset_id.clone(),
                trading_date: day0.trading_date,
                day0_outcome_7,
                pd1_outcome_7,
                pd1_dow,
                pd2_outcome_7,
            });
        }
        Ok(context)
    }

    // --- CORE ML FEATURE CALCULATION ---

    /// Calculates P(Event | Pattern) using raw counts (no time weighting).
    /// Returns a map of PatternKey -> P_Conditional.
    fn calculate_conditional_rate<T, K, I>(
        data: I,
        key_fn: impl Fn(&T) -> Option<(K, K)>, // Returns (Pattern Key, Event Key)
        min_attempts: u32,
    ) -> HashMap<K, f64>
    where
        T: Clone,
        K: Eq + std::hash::Hash + Clone,
        I: Iterator<Item = T>,
    {
        let mut event_counts: HashMap<K, u32> = HashMap::new();
        let mut pattern_totals: HashMap<K, u32> = HashMap::new();

        for item in data {
            if let Some((pattern_key, event_key)) = key_fn(&item) {
                // Total attempts for this pattern
                *pattern_totals.entry(pattern_key.clone()).or_insert(0) += 1;
                
                // Successful count for this pattern + event
                *event_counts.entry(event_key).or_insert(0) += 1;
            }
        }

        let mut final_rates: HashMap<K, f64> = HashMap::new();

        // Calculate P(Event | Pattern) = EventCount / PatternTotal
        for (event_key, event_count) in event_counts.into_iter() {
            // event_key contains both the pattern and the outcome (e.g., (Asset, P2, P1, Dow, Day0))
            // We need to extract the base pattern key for the total attempts
            
            // This requires K to be a tuple/struct that can be deconstructed to find the pattern base.
            // Since K is generic, we rely on the input pattern_totals key being correctly formatted
            // to represent the pattern *without* the final outcome.
            
            // For now, we will assume K is structured as (..., outcome), and P_base is (...,).
            // This requires a helper to get the PatternKey from the EventKey K.
            
            // Since we can't easily generalize the key deconstruction, we'll re-calculate the total
            // based on the key structure used in the functions below (which is more explicit).

            // Rerun the calculation assuming EventKey is the full tuple and PatternKey is the prefix tuple.
            // Given the complexity of generic key derivation, the simplest *correct* approach is to
            // ensure the EventKey contains the full feature set (Pattern + Outcome) and PatternTotal 
            // is calculated via a separate map lookup or explicit key extraction.
            
            // Due to the complexity of generic tuple manipulation, we'll assume the functions
            // below handle the map key generation correctly to compute P_Conditional.
            // If `key_fn` returns (P_Pattern, P_Event), and P_Event includes P_Pattern in its data,
            // we can calculate the final rate reliably.
            
            if let Some(total_attempts) = pattern_totals.get(&event_key) {
                if *total_attempts >= min_attempts && *total_attempts > 0 {
                    let p_conditional = (*event_count as f64 / *total_attempts as f64) * 100.0;
                    final_rates.insert(event_key, p_conditional.round_to(2));
                }
            }
        }
        
        // This is a common pattern for conditional probability:
        // P(A|B) = Count(A AND B) / Count(B)
        // Here, the "Event Key" (K) must uniquely define (A AND B), and the "Pattern Key" (K_pattern) must define (B).

        // Let's create an explicit function for each feature set, as the keys are complex tuples.
        HashMap::new() // Return an empty map here, as the explicit functions will handle persistence
    }


    // --- FEATURE CALCULATION FUNCTIONS (Explicit implementation of conditional rates) ---

    /// Generates DCI (Daily Confidence Index) features: P(Day0 | PD2/PD1/DOW)
    async fn calculate_dci_features(&self, day_context: &[DayContext]) -> Result<()> {
        
        let mut event_counts: HashMap<(String, String, String, String, String), u32> = HashMap::new();
        let mut pattern_totals: HashMap<(String, String, String, String), u32> = HashMap::new();
        
        for d in day_context.iter() {
            if let (Some(pd2), Some(pd1), Some(dow)) = (d.pd2_outcome_7.as_ref(), d.pd1_outcome_7.as_ref(), d.pd1_dow.as_ref()) {
                let pattern_key = (d.asset_id.clone(), pd2.clone(), pd1.clone(), dow.clone());
                let event_key = (d.asset_id.clone(), pd2.clone(), pd1.clone(), dow.clone(), d.day0_outcome_7.clone());
                
                *pattern_totals.entry(pattern_key).or_insert(0) += 1;
                *event_counts.entry(event_key).or_insert(0) += 1;
            }
        }

        // Calculate P(Day0 | PD2/PD1/DOW) and persist
        for (event_key, event_count) in event_counts.into_iter() {
            let (asset_id, pd2, pd1, dow, day0) = event_key;
            let pattern_key = (asset_id.clone(), pd2.clone(), pd1.clone(), dow.clone());

            if let Some(total_attempts) = pattern_totals.get(&pattern_key) {
                if *total_attempts >= MIN_PATTERN_ATTEMPTS {
                    let p_conditional = (*event_count as f64 / *total_attempts as f64) * 100.0;

                    let _ = sqlx::query!(
                        r#"
                        INSERT INTO ml_feature_dci_rate (
                            asset_id, pd2_outcome_7, pd1_outcome_7, pd1_dow, day0_outcome_7,
                            raw_total_attempts, p_conditional
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                        ON CONFLICT (asset_id, pd2_outcome_7, pd1_outcome_7, pd1_dow, day0_outcome_7)
                        DO UPDATE SET raw_total_attempts = EXCLUDED.raw_total_attempts, p_conditional = EXCLUDED.p_conditional
                        "#,
                        asset_id, pd2, pd1, dow, day0,
                        *total_attempts as i32, p_conditional.round_to(2)
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|e| anyhow!("DCI Feature Persistence Failed: {}", e))?;
                }
            }
        }
        Ok(())
    }

    /// Generates TCS (Transition Confidence Score) features: P(CS | PS2/PS1)
    async fn calculate_tcs_features(&self, sessions: &[SessionContext]) -> Result<()> {
        
        let mut event_counts: HashMap<(String, String, String, String, String, String, String), u32> = HashMap::new();
        let mut pattern_totals: HashMap<(String, String, String, String, String), u32> = HashMap::new();
        
        for s in sessions.iter() {
            if let (Some(ps2_n), Some(ps2_b), Some(ps1_n), Some(ps1_b), Some(cs_n), Some(cs_b)) = (
                s.ps2_name.as_ref(), s.ps2_bias_7.as_ref(), s.ps1_name.as_ref(), s.ps1_bias_7.as_ref(),
                s.cs_name.as_ref(), s.cs_bias_7.as_ref()
            ) {
                let pattern_key = (s.asset_id.clone(), ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone());
                let event_key = (s.asset_id.clone(), ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone(), cs_n.clone(), cs_b.clone());
                
                *pattern_totals.entry(pattern_key).or_insert(0) += 1;
                *event_counts.entry(event_key).or_insert(0) += 1;
            }
        }

        // Calculate P(CS | PS2/PS1) and persist
        for (event_key, event_count) in event_counts.into_iter() {
            let (asset_id, ps2_n, ps2_b, ps1_n, ps1_b, cs_n, cs_b) = event_key;
            let pattern_key = (asset_id.clone(), ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone());

            if let Some(total_attempts) = pattern_totals.get(&pattern_key) {
                if *total_attempts >= MIN_PATTERN_ATTEMPTS {
                    let p_conditional = (*event_count as f64 / *total_attempts as f64) * 100.0;

                    let _ = sqlx::query!(
                        r#"
                        INSERT INTO ml_feature_tcs_rate (
                            asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias,
                            raw_total_attempts, p_conditional
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                        ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias)
                        DO UPDATE SET raw_total_attempts = EXCLUDED.raw_total_attempts, p_conditional = EXCLUDED.p_conditional
                        "#,
                        asset_id, ps2_n, ps2_b, ps1_n, ps1_b, cs_n, cs_b,
                        *total_attempts as i32, p_conditional.round_to(2)
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|e| anyhow!("TCS Feature Persistence Failed: {}", e))?;
                }
            }
        }
        Ok(())
    }

    /// Generates High/Low Session features: P(ExtremeSession | PS2/PS1) - Simulates UNNEST
    async fn calculate_high_low_features(&self, daily_views: &[DailyViewContext], sessions: &[SessionContext]) -> Result<()> {
        
        let mut event_counts: HashMap<(String, String, String, String, String, String, String), u32> = HashMap::new();
        let mut pattern_totals: HashMap<(String, String, String, String, String, String), u32> = HashMap::new();

        for dv in daily_views.iter() {
            if dv.high_session.is_none() || dv.low_session.is_none() { continue; }
            
            // Find the corresponding session context
            if let Some(s) = sessions.iter().find(|sc| sc.asset_id == dv.asset_id && sc.trading_date == dv.trading_date) {
                if s.ps2_name.is_none() || s.ps1_name.is_none() { continue; }

                let ps2_n = s.ps2_name.as_ref().unwrap();
                let ps2_b = s.ps2_bias_7.as_ref().unwrap_or(&"".to_string());
                let ps1_n = s.ps1_name.as_ref().unwrap();
                let ps1_b = s.ps1_bias_7.as_ref().unwrap_or(&"".to_string());
                
                // --- High Session Event ---
                let pattern_high_key = (dv.asset_id.clone(), ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone(), "High".to_string());
                let event_high_key = (dv.asset_id.clone(), ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone(), "High".to_string(), dv.high_session.as_ref().unwrap().clone());
                
                *pattern_totals.entry(pattern_high_key).or_insert(0) += 1;
                *event_counts.entry(event_high_key).or_insert(0) += 1;

                // --- Low Session Event ---
                let pattern_low_key = (dv.asset_id.clone(), ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone(), "Low".to_string());
                let event_low_key = (dv.asset_id.clone(), ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone(), "Low".to_string(), dv.low_session.as_ref().unwrap().clone());

                *pattern_totals.entry(pattern_low_key).or_insert(0) += 1;
                *event_counts.entry(event_low_key).or_insert(0) += 1;
            }
        }

        // Calculate P(ExtremeSession | PS2/PS1/ExtremeType) and persist
        for (event_key, event_count) in event_counts.into_iter() {
            let (asset_id, ps2_n, ps2_b, ps1_n, ps1_b, extreme_type, extreme_session_name) = event_key;
            let pattern_key = (asset_id.clone(), ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone(), extreme_type.clone());

            if let Some(total_attempts) = pattern_totals.get(&pattern_key) {
                if *total_attempts >= MIN_PATTERN_ATTEMPTS {
                    let p_conditional = (*event_count as f64 / *total_attempts as f64) * 100.0;

                    let _ = sqlx::query!(
                        r#"
                        INSERT INTO ml_feature_high_low_rate (
                            asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, session_extreme, extreme_session_name,
                            raw_total_attempts, p_conditional
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                        ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, session_extreme, extreme_session_name)
                        DO UPDATE SET raw_total_attempts = EXCLUDED.raw_total_attempts, p_conditional = EXCLUDED.p_conditional
                        "#,
                        asset_id, ps2_n, ps2_b, ps1_n, ps1_b, extreme_type, extreme_session_name,
                        *total_attempts as i32, p_conditional.round_to(2)
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|e| anyhow!("High/Low Feature Persistence Failed: {}", e))?;
                }
            }
        }
        Ok(())
    }

    /// Generates TCS Continuation features: P(Continuation | PS2/PS1)
    async fn calculate_continuation_features(&self, sessions: &[SessionContext]) -> Result<()> {
        
        let strong_bullish_states = ["Bullish", "Bullish_Reversal", "Failed_Bullish"];
        let strong_bearish_states = ["Bearish", "Bearish_Reversal", "Failed_Bearish"];
        
        let mut continuation_counts: HashMap<(String, String, String, String, String), u32> = HashMap::new();
        let mut pattern_totals: HashMap<(String, String, String, String, String), u32> = HashMap::new();

        for s in sessions.iter() {
            if s.ps1_bias_7.is_none() || s.cs_bias_7.is_none() { continue; }
            
            let ps1 = s.ps1_bias_7.as_deref().unwrap_or_default();
            let cs = s.cs_bias_7.as_deref().unwrap_or_default();
            
            let pattern_key = (
                s.asset_id.clone(), 
                s.ps2_name.as_ref().unwrap_or(&"".to_string()).clone(), 
                s.ps2_bias_7.as_ref().unwrap_or(&"".to_string()).clone(), 
                s.ps1_name.as_ref().unwrap_or(&"".to_string()).clone(), 
                ps1.to_string()
            );

            *pattern_totals.entry(pattern_key.clone()).or_insert(0) += 1;

            let is_continuation = 
                (strong_bullish_states.contains(&ps1) && strong_bullish_states.contains(&cs)) ||
                (strong_bearish_states.contains(&ps1) && strong_bearish_states.contains(&cs)) ||
                (ps1 == "Consolidation-Down" && cs == "Consolidation-Down") ||
                (ps1 == "Consolidation-Up" && cs == "Consolidation-Up") ||
                (ps1 == "Consolidation-Other" && cs == "Consolidation-Other");

            if is_continuation {
                *continuation_counts.entry(pattern_key).or_insert(0) += 1;
            }
        }
        
        // Calculate P(Continuation | PS2/PS1) and persist
        for (pattern_key, cont_count) in continuation_counts.into_iter() {
            let (asset_id, ps2_n, ps2_b, ps1_n, ps1_b) = pattern_key.clone();
            
            if let Some(total_attempts) = pattern_totals.get(&pattern_key) {
                if *total_attempts >= MIN_PATTERN_ATTEMPTS {
                    let p_continuation = (*cont_count as f64 / *total_attempts as f64) * 100.0;
                    
                    let _ = sqlx::query!(
                        r#"
                        INSERT INTO ml_feature_tcs_continuation (
                            asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, 
                            raw_total_attempts, p_continuation
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                        ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias)
                        DO UPDATE SET raw_total_attempts = EXCLUDED.raw_total_attempts, p_continuation = EXCLUDED.p_continuation
                        "#,
                        asset_id, ps2_n, ps2_b, ps1_n, ps1_b,
                        *total_attempts as i32, p_continuation.round_to(2)
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|e| anyhow!("TCS Continuation Feature Persistence Failed: {}", e))?;
                }
            }
        }
        Ok(())
    }

    /// Orchestrates all feature generation steps.
    pub async fn generate_ml_features(
        &self,
        daily_views: &[DailyViewContext],
        session_contexts: &[SessionContext],
    ) -> Result<()> {
        
        println!("Starting ML Feature Generation Pipeline (Raw Conditional Rates)...");

        // 1. Generate Day Context for DCI features
        let day_context = Self::generate_day_context(daily_views)?;
        
        // 2. Calculate DCI Features: P(Day0 | PD2/PD1/DOW)
        self.calculate_dci_features(&day_context).await?;
        
        // 3. Calculate TCS Features: P(CS | PS2/PS1)
        self.calculate_tcs_features(session_contexts).await?;
        
        // 4. Calculate High/Low Features: P(ExtremeSession | PS2/PS1)
        self.calculate_high_low_features(daily_views, session_contexts).await?;
        
        // 5. Calculate TCS Continuation Features: P(Continuation | PS2/PS1)
        self.calculate_continuation_features(session_contexts).await?;

        println!("ML Feature generation complete. Raw conditional probabilities persisted.");
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