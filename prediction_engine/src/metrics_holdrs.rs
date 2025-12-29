


/// Calculates the final Predictive Daily Confidence Score (PDCS) by scaling
/// the 3rd-Order conditional probability against the Base Rate.
pub fn calculate_pdcs_scores(
    conditional_rates: Vec<DayOutcome3rdOrder>,
    base_rates: Vec<DayTypeBaseRate>,
) -> Vec<DailyConfidenceScore> {
    
    //  Prepare Base Rates for efficient lookup
    // Key: (Asset ID, Daily Outcome 7) -> P_Base
    let base_rate_map: HashMap<(String, String), f64> = base_rates.into_iter()
        .map(|b| ((b.asset_id, b.daily_bias), b.p_day_type_base))
        .collect();

    //  Iterate through Conditional Rates and calculate the PDCS score
    conditional_rates.into_iter()
        .filter_map(|p| {
            let key = (p.asset_id.clone(), p.c_day_bias.clone());
            
            // Look up the Base Rate
            if let Some(&p_day_type_base) = base_rate_map.get(&key) {
                
                // P_conditional and P_base are stored as 0.00-1.00 decimals
                let pdcs_score = p.p_day_bias_conditional / p_day_type_base;
                let rounded_score = (pdcs_score * 100.0).round() / 100.0;
                
                // Assign insight label based on your SQL logic
                let insight_label = match rounded_score {
                    s if s >= 2.25 => "Extreme Confidence Signal (PCS >= 2.25)".to_string(),
                    s if s >= 1.75 => "High-Confidence Signal (PCS >= 1.75)".to_string(),
                    s if s < 0.75 => "Contradictory Signal/Failure Risk (PCS < 0.75)".to_string(),
                    _ => "Confirmatory/Expected Range".to_string(),
                };

                Some(DailyConfidenceScore {
                    asset_id: p.asset_id,
                    pd2_outcome: p.pd2_bias,
                    pd1_outcome: p.pd1_bias,
                    pd1_dow: p.pd1_dow,
                    day0_outcome: p.c_day_bias,
                    p_day_outcome_conditional: p.p_day_bias_conditional,
                    p_day_type_base,
                    pdcs_score: rounded_score,
                    insight_label,
                })
            } else {
                // Base rate not found for this asset/outcome combination, skip
                None
            }
        })
        .collect()
}

type PatternKey = (String, String, String, String, String);

pub fn calculate_pcs_scores(
    conditional_rates: Vec<DayType2ndOrder>,
    base_rates: Vec<DayTypeBaseRate>,
) -> Vec<PcsScore2ndOrder> {
    
    // 1. Prepare Base Rates for efficient lookup
    // Key: (Asset ID, Day Type) -> P_Base
    let base_rate_map: HashMap<(String, String), f64> = base_rates.into_iter()
        .map(|b| ((b.asset_id, b.daily_bias), b.p_day_type_base))
        .collect();

    // 2. Aggregate PCS scores to find the MAXIMUM score per unique pattern
    // Key: PatternKey -> (Max PCS Score, Winning Day Type)
    let mut final_pcs_map: HashMap<PatternKey, (f64, String)> = HashMap::new();

    for cond in conditional_rates {
        let base_key = (cond.asset_id.clone(), cond.day_type.clone());
        
        if let Some(&p_base) = base_rate_map.get(&base_key) {
            
            // Calculate PCS Ratio: P_Conditional / P_Base
            let pcs_score = cond.p_day_type_conditional / p_base;

            // Construct the unique 5-element pattern key
            let pattern_key: PatternKey = (
                cond.asset_id.clone(),
                cond.ps2_name.clone(),
                cond.ps2_bias.clone(),
                cond.ps1_name.clone(),
                cond.ps1_bias.clone(),
            );
            
            // Find the highest PCS score for this specific pattern
            let entry = final_pcs_map.entry(pattern_key).or_insert((0.0, String::new()));
            
            // Update the entry if the current score is higher
            if pcs_score > entry.0 {
                entry.0 = pcs_score;
                entry.1 = cond.day_type;
            }
        }
    }
    
    // 3. Map the results into the final PcsScore2ndOrder structure
    final_pcs_map.into_iter()
        .map(|(key, (pcs_score, pcs_day_type))| {
            let (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias) = key;
            
            PcsScore2ndOrder {
                asset_id,
                ps2_name,
                ps2_bias,
                ps1_name,
                ps1_bias,
                pcs_day_type,
                // Rounding to two decimal places
                pcs_score: (pcs_score * 100.0).round() / 100.0,
            }
        })
        .collect()
}

pub fn calculate_weekly_continuation_score(
    weekly_contexts: &[WeeklyContext],
    intervals: &[LookbackInterval],
    min_attempts: f64,
) -> Vec<WeeklyContinuationScore> {
    
    // WeeklyContext implements CountableContext for this exact pattern
    let results = aggregate_weighted_counts(
        weekly_contexts, 
        intervals, 
        min_attempts
    );

    results.into_iter().map(|(key, (success, total))| {
        let (asset_id, pattern_key, week0_outcome_7) = key;
        let (pw2_outcome_7, pw1_outcome_7) = pattern_key;

        let wcs_score = success / total;
        
        WeeklyContinuationScore {
            asset_id,
            pw2_outcome: pw2_outcome_7,
            pw1_outcome: pw1_outcome_7,
            week0_outcome: week0_outcome_7,
            wcs_score: (wcs_score * 100.0).round() / 100.0,
        }
    })
    .collect()
}


/// Calculates the Weekly Outcome Conditional on Daily Score P(W0 | PD2, PD1).
pub fn calculate_wocs_d_score(
    wocs_contexts: &[WocsContext],
    intervals: &[LookbackInterval],
    min_attempts: f64,
) -> Vec<WocsDailyScore> {
    
    let results = aggregate_weighted_counts(
        wocs_contexts, 
        intervals, 
        min_attempts
    );

    results.into_iter().map(|(key, (success, total))| {
        let (asset_id, pattern_key, week0_outcome_7) = key;
        let (pd2_outcome_7, pd1_outcome_7) = pattern_key;

        let wocs_d_score = success / total;
        
        WocsDailyScore {
            asset_id,
            pd2_outcome: pd2_outcome_7,
            pd1_outcome: pd1_outcome_7,
            week0_outcome: week0_outcome_7,
            wocs_d_score: (wocs_d_score * 100.0).round() / 100.0,
        }
    })
    .collect()
}

/// Calculates the Weekly Monthly Seasonality Score P(W0 | Week of Month).
pub fn calculate_weekly_monthly_seasonality(
    weekly_contexts: &[WeeklyContext],
    intervals: &[LookbackInterval],
    min_attempts: f64,
) -> Vec<WeeklyMonthlySeasonality> {
    
    // 1. Wrap the contexts for the specialized WMS calculation
    let wms_contexts: Vec<WmsContext> = weekly_contexts.iter()
        .cloned()
        .map(|w| WmsContext { inner: w })
        .collect();

    // 2. Run the weighted aggregation
    let results = aggregate_weighted_counts(
        &wms_contexts, 
        intervals, 
        min_attempts
    );

    // 3. Map the results
    results.into_iter().map(|(key, (success, total))| {
        let (asset_id, week0_week_of_month, week0_outcome_7) = key;

        let wms_score = success / total;
        
        WeeklyMonthlySeasonality {
            asset_id,
            week0_week_of_month,
            week0_outcome: week0_outcome_7,
            wms_score: (wms_score * 100.0).round() / 100.0,
        }
    })
    .collect()
}

/// Calculates the final Transition Confidence Score (TCS) by scaling
/// the 2nd-Order transition probability against the CS Base Rate.
pub fn calculate_tcs_scores(
    conditional_rates: Vec<Transition2ndOrder>,
    base_rates: Vec<CsBaseRate>,
) -> Vec<Tcs2ndOrder> {
    
    //  Prepare Base Rates for efficient lookup
    // Key: (Asset ID, CS Bias) -> P_Base
    let base_rate_map: HashMap<(String, String), f64> = base_rates.into_iter()
        .map(|b| ((b.asset_id, b.cs_bias), b.p_base))
        .collect();

    // Iterate through Conditional Rates and calculate the TCS score
    conditional_rates.into_iter()
        .filter_map(|t| {
            let key = (t.asset_id.clone(), t.cs_bias.clone());
            
            if let Some(&p_cs_base) = base_rate_map.get(&key) {
                
                let tcs_score = t.p_transition_conditional / p_cs_base;
                let rounded_score = (tcs_score * 100.0).round() / 100.0;
                
                Some(Tcs2ndOrder {
                    asset_id: t.asset_id,
                    ps2_name: t.ps2_name, 
                    ps2_bias: t.ps2_bias,
                    ps1_name: t.ps1_name, 
                    ps1_bias: t.ps1_bias,
                    cs_name: t.cs_name,
                    cs_bias: t.cs_bias,
                    p_transition_conditional: t.p_transition_conditional,
                    p_cs_base,
                    tcs_score: rounded_score,
                })
            } else {
                None
            }
        })
        .collect()
}

// Calculates the Daily Trend Rate Conditional Probability P(Day0 Direction | PD2 Direction, PD1 Direction, DOW).
pub fn calculate_daily_trend_rate(
    day_contexts: &[DayContext],
    intervals: &[LookbackInterval],
    min_attempts: f64,
) -> Vec<DailyTrendRate> {
    
    // Convert DayContexts to the specialized TrendRateContext for aggregation
    let trend_contexts: Vec<TrendRateContext> = day_contexts.iter()
        .cloned()
        .map(|d| TrendRateContext { inner: d })
        .collect();

    //  Run the weighted aggregation
    let results = aggregate_weighted_counts(
        &trend_contexts, 
        intervals, 
        min_attempts
    );

    //  Map the results to the final Vec<DailyTrendRate> struct
    results.into_iter().map(|(key, (success, total))| {
        let (asset_id, pattern_key, day0_direction) = key;
        let (pd2_direction, pd1_direction, pd1_dow) = pattern_key;

        let p_trend_rate = (success / total) * 100.0;
        
        DailyTrendRate {
            asset_id,
            pd2_direction,
            pd1_direction,
            pd1_dow,
            day0_direction,
            p_trend_rate: p_trend_rate.round() / 100.0, 
        }
    })
    .collect()
}

// Calculates the 3rd-Order Day Outcome Conditional Probability P(Day0 Type | PD2, PD1, DOW). This is the core engine of the Daily Confidence Index (DCI).
pub fn calculate_day_outcome_3rd_order(
    day_contexts: &[DayContext],
    intervals: &[LookbackInterval],
    min_attempts: f64,
) -> Vec<DayOutcome3rdOrder> {
    
    // Run the weighted aggregation using the DayContext implementation
    let results = aggregate_weighted_counts(
        day_contexts, 
        intervals, 
        min_attempts
    );

    // 2. Map the HashMap results to the final Vec<DayOutcome3rdOrder> struct
    results.into_iter().map(|(key, (success, total))| {
        let (asset_id, pattern_key, day0_outcome_7) = key;
        let (pd2_outcome_7, pd1_outcome_7, pd1_dow) = pattern_key;

        let p_conditional = (success / total) * 100.0;
        
        DayOutcome3rdOrder {
            asset_id,
            pd2_bias: pd2_outcome_7,
            pd1_bias: pd1_outcome_7,
            pd1_dow,
            c_day_bias: day0_outcome_7,
            reliable_total_attempts: total.round(),
            reliable_success_count: success.round(),
            p_day_bias_conditional: p_conditional.round() / 100.0, // Storing as a percentage
        }
    })
    .collect()
}

/// Calculates the High/Low Session Conditional Probability P(Session Name | PS2 Bias, PS1 Bias, High/Low).
pub fn calculate_high_low_session_2nd_order(
    high_low_contexts: &[HighLowContext],
    intervals: &[LookbackInterval],
    min_attempts: f64,
) -> Vec<HighLowSession2ndOrder> {
    
    let results = aggregate_weighted_counts(high_low_contexts, intervals, min_attempts);

    results.into_iter().map(|(key, (success, total))| {
        let (asset_id, pattern_key, extreme_session_name) = key;
        
        let (ps2_name, ps2_bias, ps1_name, ps1_bias, session_extreme) = pattern_key;

        let p_conditional = success / total;
        
        HighLowSession2ndOrder {
            asset_id,
            ps2_bias,
            ps1_bias,
            ps2_name,
            ps1_name, 
            session_extreme,
            extreme_session_name,
            reliable_total_attempts: total.round(),
            reliable_success_count: success.round(),
            p_extreme_session_conditional: (p_conditional * 100.0).round() / 100.0,
        }
    })
    .collect()
}

// Calculates the overall probability (Base Rate) for each Weekly Outcome Type.
pub fn calculate_weekly_base_rates(
    weekly_contexts: &[WeeklyContext],
    intervals: &[LookbackInterval],
) -> Vec<WeeklyBaseRate> {
    
    let base_contexts: Vec<BaseRateWeeklyContext> = weekly_contexts.iter()
        .cloned()
        .map(|w| BaseRateWeeklyContext { inner: w })
        .collect();

    let results = aggregate_weighted_counts(
        &base_contexts, 
        intervals, 
        10.0
    );

    results.into_iter().map(|(key, (success, total))| {
        let (asset_id, _, week_bias) = key;
        
        let weekly_base_rate = (success / total) * 100.0;
        
        WeeklyBaseRate {
            asset_id,
            week_bias,
            weekly_base_rate: weekly_base_rate.round() / 100.0,
        }
    })
    .collect()
}

/// Calculates the 2nd-Order Transition Conditional Probability P(CS Bias | PS2 Bias, PS1 Bias).
pub fn calculate_transition_2nd_order(
    session_contexts: &[SessionContextData],
    intervals: &[LookbackInterval],
    min_attempts: f64,
) -> Vec<Transition2ndOrder> {
    
    // Aggregation now uses the expanded 4-tuple key
    let results = aggregate_weighted_counts(session_contexts, intervals, min_attempts);

    results.into_iter().map(|(key, (success, total))| {
        let (asset_id, pattern_key, cs_bias) = key;
        
        let (ps2_name, ps2_bias, ps1_name, ps1_bias) = pattern_key; 
        
        let p_conditional = success / total;
        
        // Note: cs_name still needs a lookup or derivation, but not part of the pattern
        let cs_name = format!("{}_Session", cs_bias); 
        
        Transition2ndOrder {
            asset_id,
            ps2_bias,
            ps1_bias,
            cs_bias,
            ps2_name, 
            ps1_name, 
            cs_name,  
            reliable_total_attempts: total.round(),
            reliable_success_count: success.round(),
            p_transition_conditional: (p_conditional * 100.0).round() / 100.0,
        }
    })
    .collect()
}

/// Calculates the 2nd-Order Transition Conditional Probability P(CS Bias | PS2 Bias, PS1 Bias).
pub fn calculate_transition_2nd_order(
    session_contexts: &[SessionContextData],
    intervals: &[LookbackInterval],
    min_attempts: f64,
) -> Vec<Transition2ndOrder> {
    
    // Aggregation now uses the expanded 4-tuple key
    let results = aggregate_weighted_counts(session_contexts, intervals, min_attempts);

    results.into_iter().map(|(key, (success, total))| {
        let (asset_id, pattern_key, cs_bias) = key;
        
        let (ps2_name, ps2_bias, ps1_name, ps1_bias) = pattern_key; 
        
        let p_conditional = success / total;
        
        // Note: cs_name still needs a lookup or derivation, but not part of the pattern
        let cs_name = format!("{}_Session", cs_bias); 
        
        Transition2ndOrder {
            asset_id,
            ps2_bias,
            ps1_bias,
            cs_bias,
            ps2_name, 
            ps1_name, 
            cs_name,  
            reliable_total_attempts: total.round(),
            reliable_success_count: success.round(),
            p_transition_conditional: (p_conditional * 100.0).round() / 100.0,
        }
    })
    .collect()
}


    pub fn calculate_high_low_session_2nd_order(
        &self,
    high_low_contexts: &[HighLowContext],
    intervals: &[LookbackInterval],
    min_attempts: f64,
) -> Vec<HighLowSession2ndOrder> {
    
    // The aggregation call uses the Trait implementation on HighLowContext
    let results = aggregate_weighted_counts(
        high_low_contexts, 
        intervals, 
        min_attempts
    );

    results.into_iter().map(|(key, (success, total))| {
        let (asset_id, pattern_key, extreme_session_name) = key;
        
        let (ps2_name, ps2_bias, ps1_name, ps1_bias, session_extreme) = pattern_key;

        let p_conditional = success / total;
        let p_conditional_rounded = round_to_decimal_places(p_conditional, 4); // Round to 4 decimal places

        HighLowSession2ndOrder {
            asset_id,
            ps2_bias,
            ps1_bias,
            ps2_name,
            ps1_name, 
            session_extreme,
            extreme_session_name,
            reliable_total_attempts: total.round(),
            reliable_success_count: success.round(),
            p_extreme_session_conditional: p_conditional_rounded,
        }
    })
    .collect()
}


    pub async fn fetch_tcs_input_data(
        &self,
        asset_id: &str,
    ) -> Result<TcsData> {
        
        // 1. Fetch 2nd Order Transitions
        let transitions_query = r#"
            SELECT 
                asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, 
                reliable_total_attempts, reliable_success_count, p_transition_conditional
            FROM transition_2nd_order
            WHERE asset_id = $1
        "#;
        let transitions = sqlx::query_as::<_, Transition2ndOrder>(transitions_query)
            .bind(asset_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to fetch 2nd Order Transitions for TCS: {}", e))?;

        // 2. Fetch CS Base Rates (Denominator)
        let base_rates_query = r#"
            SELECT asset_id, cs_bias, p_base
            FROM cs_base_rates_ml
            WHERE asset_id = $1
        "#;
        let base_rates = sqlx::query_as::<_, CsBaseRate>(base_rates_query)
            .bind(asset_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to fetch CS Base Rates for TCS: {}", e))?;

        Ok(TcsData { transitions, base_rates })
    }



    // --- Specialized Context for Base Rate Calculation (P(Outcome)) ---
#[derive(Debug, Clone)]
pub struct BaseRateDayContext {
    pub inner: DayContext,
}

impl MetricsContext for BaseRateDayContext {
    // The pattern key is constant, representing 'no specific pattern'
    type PatternKey = bool; 
    type OutcomeKey = String; 

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        Some(true)
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        if self.inner.c_day_bias != "Other" && self.inner.c_day_bias != "Consolidation-Other" {
            Some(self.inner.c_day_bias.clone())
        } else {
            None
        }
    }
}






#[derive(Debug, Clone)]
pub struct BaseRateWeeklyContext {
    pub inner: WeeklyContext,
}

impl MetricsContext for BaseRateWeeklyContext {
    type PatternKey = bool;
    type OutcomeKey = String;

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.week_start }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        Some(true)
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        Some(self.inner.week_bias.clone())
    }
}




impl MetricsContext for DayContext {
    // Pattern is the input key: (PD2_Outcome, PD1_Outcome, PD1_DOW)
    type PatternKey = (String, String, String);

    fn get_asset_id(&self) -> String {
        self.asset_id.clone()
    }

    // Outcome is the resulting key: (Day_Outcome)
    type OutcomeKey = String;

    fn get_date(&self) -> NaiveDate {
        self.trading_date
    }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        
        if let (Some(pd2), Some(pd1), Some(dow)) = (
            &self.pd2_bias, 
            &self.pd1_bias, 
            &self.pd1_dow
        ) {
            Some((pd2.clone(), pd1.clone(), dow.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        if self.c_day_bias != "Other" && self.c_day_bias != "Consolidation-Other" {
            Some(self.c_day_bias.clone())
        } else {
            None
        }
    }

}


// --- For WCS (Weekly Continuation Score) ---
impl MetricsContext for WeeklyContext {
    // Pattern is the input key: (PW2_Outcome, PW1_Outcome)
    type PatternKey = (String, String);
    
    // Outcome is the resulting key: (Week_Outcome)
    type OutcomeKey = String;

    fn get_asset_id(&self) -> String {
        self.asset_id.clone()
    }
    
    fn get_date(&self) -> NaiveDate {
        self.week_start
    }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        
        if let (Some(pw2), Some(pw1)) = (
            &self.pw2_bias, 
            &self.pw1_bias
        ) {
            Some((pw2.clone(), pw1.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        Some(self.week_bias.clone())
    }
}


// --- For WMS (Weekly Monthly Seasonality) ---
// Since WMS uses a different PatternKey (Week of Month), we need a separate
// struct or helper function, but we can stick to the primary trait for WCS 
// for simplicity now and treat WMS as a special case function call, or
// define a new trait implementation based on the `week0_week_of_month` field.

// Let's create a specialized struct for WMS to handle the different key:
#[derive(Debug, Clone)]
pub struct WmsContext {
    pub inner: WeeklyContext,
}

impl MetricsContext for WmsContext {
    // Pattern is the input key: (Week0_Week_of_Month)
    type PatternKey = i32; // Week of Month (1-4/5)
    
    // Outcome is the resulting key: (Week0_Outcome_7)
    type OutcomeKey = String;

    fn get_asset_id(&self) -> String {
        self.inner.asset_id.clone()
    }

    fn get_date(&self) -> NaiveDate {
        self.inner.week_start
    }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        // Pattern is simply the week of month
        Some(self.inner.week_of_month)
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        Some(self.inner.week_bias.clone())
    }
}

// --- Specialized Context for Trend Rate Calculation ---

// A temporary struct wrapper to tell the aggregator to use Direction fields
#[derive(Debug, Clone)]
pub struct TrendRateContext {
    pub inner: DayContext,
}

impl MetricsContext for TrendRateContext {
    type PatternKey = (String, String, String); // (PD2_bias, PD1_bias, PD1_DOW)
    type OutcomeKey = String; // c_day_bias

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        if let (Some(pd2), Some(pd1), Some(dow)) = (
            &self.inner.pd2_bias, 
            &self.inner.pd1_bias, 
            &self.inner.pd1_dow
        ) {
            Some((pd2.clone(), pd1.clone(), dow.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        Some(self.inner.day0_direction.clone())
    }
}

// --- Specialized Context for Continuation Rate Calculation ---


#[derive(Debug, Clone)]
pub struct WocsContext {
    pub trading_date: NaiveDate, // Date of the PD1/PD2 pattern
    pub asset_id: String,
    pub pd2_outcome: String,
    pub pd1_outcome: String,
    pub week0_outcome: String, // The weekly outcome observed after the daily pattern
}

impl MetricsContext for WocsContext {
    // Pattern is the input key: (PD2_Outcome_7, PD1_Outcome_7)
    type PatternKey = (String, String); 
    // Outcome is the resulting key: (Week0_Outcome_7)
    type OutcomeKey = String; 

    fn get_asset_id(&self) -> String { self.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        Some((self.pd2_outcome.clone(), self.pd1_outcome.clone()))
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        Some(self.week0_outcome.clone())
    }
}

// The unique key for the PS2 -> PS1 pattern (4 elements)
type SessionPatternKey = (String, String, String, String); 
// The outcome we want to predict is the Daily Bias
type DailyOutcomeKey = String; 

#[derive(Debug, Clone)]
pub struct PcsContext {
    pub inner: SessionContextData,
    pub actual_daily_bias: String, // Must be joined from DayContext or similar source
}

impl MetricsContext for PcsContext {
    // Pattern is the input key: (PS2_Name, PS2_Bias, PS1_Name, PS1_Bias)
    type PatternKey = (String, String, String, String);
    
    // Outcome is the resulting key: (Actual Daily Bias)
    type OutcomeKey = String;

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        
        // This is the 4-element pattern that precedes the CS session
        if let (Some(ps2_n), Some(ps2_b), Some(ps1_n), Some(ps1_b)) = (
            &self.inner.ps2_name, 
            &self.inner.ps2_bias, 
            &self.inner.ps1_name, 
            &self.inner.ps1_bias
        ) {
            Some((ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        // We ensure the daily bias is one of the valid 7 states (not 'Other')
        if self.actual_daily_bias != "Other" && self.actual_daily_bias != "Consolidation-Other" {
            Some(self.actual_daily_bias.clone())
        } else {
            None
        }
    }
}

// Conceptual function to prepare the data for PCS calculation
pub fn prepare_pcs_contexts(
    session_contexts: &[SessionContextData],
    day_contexts: &[DayContext],
) -> Vec<PcsContext> {
    
    // Create a lookup map for the Daily Context's outcome
    let daily_map: HashMap<(NaiveDate, String), &DayContext> = day_contexts.iter()
        .map(|d| ((d.trading_date, d.asset_id.clone()), d))
        .collect();

    session_contexts.iter()
        .filter_map(|sc| {
            let key = (sc.trading_date, sc.asset_id.clone());
            
            // 1. Find the corresponding Day Context
            let dc = daily_map.get(&key)?;
            
            // 2. Ensure we have the necessary pattern elements for the key
            sc.ps2_name.as_ref()?;
            sc.ps2_bias.as_ref()?;
            sc.ps1_name.as_ref()?;
            sc.ps1_bias.as_ref()?;

            // 3. Create the PcsContext
            Some(PcsContext {
                inner: sc.clone(),
                actual_daily_bias: dc.c_day_bias.clone(), // Use the actual daily outcome
            })
        })
        .collect()
}


