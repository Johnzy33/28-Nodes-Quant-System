
use chrono::NaiveDate;
use std::collections::HashMap;

// --- 1. CORE DATA STRUCTURES (Inputs/Targets) ---

// The key used for all lookups related to a specific session transition pattern
pub type FciLookupKey = (NaiveDate, String, String, String, String, String); 
// (date, asset, ps2_name, ps1_name, ps2_bias, ps1_bias)

/// Placeholder for the raw 2nd-Order Transition data (P_conditional)
#[derive(Debug, Clone)]
pub struct Transition2ndOrder {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,
    pub cs_name: String,        // The target session (e.g., "NYL")
    pub cs_bias: String,        // The target bias (e.g., "Bullish")
    pub p_transition_conditional: f64, // P(CS=Bullish | PS2=Bearish, PS1=Bullish)
}

/// The calculated raw P_Continuation score
#[derive(Debug, Clone)]
pub struct TcsContinuationScore {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub ps2_name: String,
    pub ps1_name: String,
    pub ps2_bias: String,   // Crucial for the 6-element key
    pub ps1_bias: String,   // Crucial for the 6-element key
    pub p_continuation: f64, // The raw probability (P_Continuation)
}

/// The calculated TCS multiplier score
#[derive(Debug, Clone)]
pub struct Tcs2ndOrder {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub ps2_name: String,
    pub ps1_name: String,
    pub ps2_bias: String,   // Crucial for the 6-element key
    pub ps1_bias: String,   // Crucial for the 6-element key
    pub tcs_score: f64,     // The calculated TCS multiplier score (P_cond / P_base)
}

/// Placeholder for the raw historical log, defining the *target* pattern
#[derive(Debug, Clone)]
pub struct SessionContextData {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub ps2_name: String,
    pub ps1_name: String,
    pub ps2_bias: String,
    pub ps1_bias: String,
    pub cs_name: String, // The session being predicted (e.g., "NYL")
    pub cs_bias: String, // The actual outcome of CS (Target Variable)
}

/// Placeholder for the aggregated daily outcome (proxy for Model 1's output)
#[derive(Debug, Clone)]
pub struct DailyOutcomeValue {
    pub day_outcome: String, // e.g., "Bullish"
    pub day_confidence: f64, // e.g., 85.5
}


// --- 2. FCI LOOKUP STRUCTURES ---

/// Structure to hold the aggregated FCI metrics for a single pattern key (the Value in the HashMap).
#[derive(Debug, Clone)]
pub struct FciMetricsForSession {
    pub long_conditional_prob: f64, 
    pub short_conditional_prob: f64, 
    pub tcs_score: f64,
    pub continuation_prob: f64,
}


// --- 3. HELPER LOOKUP BUILDERS ---

/// Builds a lookup map for the TCS Continuation Score (P_Continuation)
fn build_continuation_lookup_map(
    scores: &Vec<TcsContinuationScore>
) -> HashMap<FciLookupKey, f64> {

    scores.iter()
        .map(|tcs| {
            let key = (
                tcs.trading_date,
                tcs.asset_id.clone(),
                tcs.ps2_name.clone(),
                tcs.ps1_name.clone(),
                tcs.ps2_bias.clone(),
                tcs.ps1_bias.clone(),
            );
            (key, tcs.p_continuation)
        })
        .collect()
}

/// Builds a lookup map for the raw TCS multiplier score
fn build_tcs_score_lookup_map(
    scores: &Vec<Tcs2ndOrder>
) -> HashMap<FciLookupKey, f64> {

    scores.iter()
        .map(|tcs| {
            let key = (
                tcs.trading_date,
                tcs.asset_id.clone(),
                tcs.ps2_name.clone(),
                tcs.ps1_name.clone(),
                tcs.ps2_bias.clone(),
                tcs.ps1_bias.clone(),
            );
            (key, tcs.tcs_score)
        })
        .collect()
}


// --- 4. CORE FCI LOOKUP BUILDER (MAIN JOIN LOGIC) ---

/// Aggregates all Transition2ndOrder entries into a single FciMetricsForSession struct per pattern.
pub fn build_fci_lookup_map_for_session(
    transitions: &Vec<Transition2ndOrder>,
    tcs_scores: &Vec<Tcs2ndOrder>,
    continuation_scores: &Vec<TcsContinuationScore>,
) -> HashMap<FciLookupKey, FciMetricsForSession> {
    
    // 1. Create Lookup Maps for auxiliary data
    let tcs_map = build_tcs_score_lookup_map(tcs_scores);
    let cont_map = build_continuation_lookup_map(continuation_scores);

    // 2. Group Transition2ndOrder results by the unique pattern (6-element key)
    let mut grouped_results: HashMap<FciLookupKey, Vec<&Transition2ndOrder>> = HashMap::new();
    
    for t in transitions {
        let key = (
            t.trading_date,
            t.asset_id.clone(),
            t.ps2_name.clone(),
            t.ps1_name.clone(),
            t.ps2_bias.clone(),
            t.ps1_bias.clone(),
        );
        grouped_results.entry(key).or_insert_with(Vec::new).push(t);
    }
    
    // 3. Calculate MAX Long/Short P_cond and join auxiliary scores
    grouped_results.into_iter().filter_map(|(key, transitions)| {
        let mut max_long_p = 0.0;
        let mut max_short_p = 0.0;
        
        // Find MAX P_conditional for Long/Short groups based on CS bias
        for t in transitions {
            let p = t.p_transition_conditional;
            match t.cs_bias.as_str() {
                // Defines the Long Group (Bullish family)
                "Bullish" | "Bullish_Reversal" | "Consolidation-Up" | "Failed_Bearish" => max_long_p = max_long_p.max(p),
                // Defines the Short Group (Bearish family)
                "Bearish" | "Bearish_Reversal" | "Consolidation-Down" | "Failed_Bullish" => max_short_p = max_short_p.max(p),
                _ => {} // Ignore neutral/other biases for MAX calculation
            }
        }
        
        // 4. Look up TCS and P_Continuation using the same 6-element key
        if let Some(&tcs_score) = tcs_map.get(&key) {
            let continuation_prob = *cont_map.get(&key).unwrap_or(&0.0); // Default to 0.0 if P_Continuation is missing
            
            Some((key, FciMetricsForSession {
                long_conditional_prob: max_long_p,
                short_conditional_prob: max_short_p,
                tcs_score,
                continuation_prob,
            }))
        } else {
            // Drop the entry if we can't find the TCS score
            eprintln!("Warning: Dropping pattern {:?} due to missing TCS score.", key);
            None 
        }
    })
    .collect()
}


// --- 5. FINAL FEATURE VECTOR STRUCTURE AND GENERATION ---

/// The final feature vector for the ML model (Model 2: Session Momentum Predictor)
#[derive(Debug, Clone)]
pub struct SessionFeatureVector {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    
    // --- A. PATTERN CONTEXT (6-Element Key) ---
    pub ps2_name: String,        
    pub ps1_name: String,        
    pub ps2_bias: String,        
    pub ps1_bias: String,        
    
    // --- B. RAW PROBABILITIES & SCALED SCORES (FCI Signals) ---
    pub p_cs_long_cond: f64,     // MAX P(CS in Long Group | PS2, PS1)
    pub p_cs_short_cond: f64,    // MAX P(CS in Short Group | PS2, PS1)
    pub tcs_score: f64,          // The calculated TCS score
    pub p_tcs_continuation: f64, // P(Continuation) for this pattern
    
    // --- C. THE DAILY ANCHOR (Prediction from Model 1 Proxy) ---
    pub ml_daily_prediction: String, 
    pub ml_daily_score: f64,         

    // --- TARGET (Y) ---
    pub target_cs_bias: String,  // The known bias outcome of the next session (CS)
}

/// Orchestrates the final join to create the ML training dataset.
pub fn generate_session_feature_vector(
    session_context_data: Vec<SessionContextData>, // The raw historical session log
    fci_metrics_map: HashMap<FciLookupKey, FciMetricsForSession>,
    daily_outcome_map: HashMap<(NaiveDate, String), DailyOutcomeValue>, // ML Anchor Proxy
) -> Vec<SessionFeatureVector> {
    
    let mut feature_vectors = Vec::new();
    
    for session_context in session_context_data {
        // Construct the 6-element key to lookup the FCI metrics
        let key: FciLookupKey = (
            session_context.trading_date, 
            session_context.asset_id.clone(), 
            session_context.ps2_name.clone(), 
            session_context.ps1_name.clone(),
            session_context.ps2_bias.clone(), 
            session_context.ps1_bias.clone()
        );

        if let Some(fci_metrics) = fci_metrics_map.get(&key) {
            
            // Construct the 2-element key to lookup the Daily Anchor
            let daily_anchor_key = (session_context.trading_date, session_context.asset_id.clone());
            
            if let Some(daily_outcome) = daily_outcome_map.get(&daily_anchor_key) {
                
                let vector = SessionFeatureVector {
                    trading_date: session_context.trading_date,
                    asset_id: session_context.asset_id,
                    
                    ps2_name: session_context.ps2_name,
                    ps1_name: session_context.ps1_name,
                    ps2_bias: session_context.ps2_bias,
                    ps1_bias: session_context.ps1_bias,
                    
                    p_cs_long_cond: fci_metrics.long_conditional_prob,
                    p_cs_short_cond: fci_metrics.short_conditional_prob,
                    tcs_score: fci_metrics.tcs_score,
                    p_tcs_continuation: fci_metrics.continuation_prob,
                    
                    ml_daily_prediction: daily_outcome.day_outcome.clone(),
                    ml_daily_score: daily_outcome.day_confidence, 
                    
                    target_cs_bias: session_context.cs_bias, 
                };
                feature_vectors.push(vector);
            } else {
                eprintln!("Warning: Missing Daily Anchor for {:?}", daily_anchor_key);
            }
        } else {
            eprintln!("Warning: Missing FCI Metrics for pattern {:?}", key);
        }
    }

    feature_vectors
}

// --- MOCK EXECUTION FOR DEMONSTRATION ---

fn main() {
    let date = NaiveDate::from_ymd_opt(2023, 10, 26).unwrap();
    let asset = "AUDCAD".to_string();
    let ps2_name = "LN".to_string();
    let ps1_name = "NYAM".to_string();
    let ps2_bias = "Bearish".to_string();
    let ps1_bias = "Bullish_Reversal".to_string();
    let cs_name = "NYL".to_string();
    let target_bias = "Bullish".to_string();


    // 1. Mock Transition Data (Inputs to FCI Lookup Builder)
    let transitions = vec![
        // Transition 1: Leads to Bullish CS (P_long)
        Transition2ndOrder { trading_date: date, asset_id: asset.clone(), ps2_name: ps2_name.clone(), ps2_bias: ps2_bias.clone(), ps1_name: ps1_name.clone(), ps1_bias: ps1_bias.clone(), cs_name: cs_name.clone(), cs_bias: "Bullish".to_string(), p_transition_conditional: 0.75 },
        // Transition 2: Leads to Bearish CS (P_short)
        Transition2ndOrder { trading_date: date, asset_id: asset.clone(), ps2_name: ps2_name.clone(), ps2_bias: ps2_bias.clone(), ps1_name: ps1_name.clone(), ps1_bias: ps1_bias.clone(), cs_name: cs_name.clone(), cs_bias: "Bearish".to_string(), p_transition_conditional: 0.20 },
    ];

    let tcs_scores = vec![
        Tcs2ndOrder { trading_date: date, asset_id: asset.clone(), ps2_name: ps2_name.clone(), ps1_name: ps1_name.clone(), ps2_bias: ps2_bias.clone(), ps1_bias: ps1_bias.clone(), tcs_score: 1.875 }, // P_cond(0.75)/P_base(0.4)
    ];

    let continuation_scores = vec![
        TcsContinuationScore { trading_date: date, asset_id: asset.clone(), ps2_name: ps2_name.clone(), ps1_name: ps1_name.clone(), ps2_bias: ps2_bias.clone(), ps1_bias: ps1_bias.clone(), p_continuation: 0.65 },
    ];

    // 2. Build the FCI Lookup Map
    let fci_metrics_map = build_fci_lookup_map_for_session(&transitions, &tcs_scores, &continuation_scores);
    
    println!("--- FCI Lookup Map Generated ---");
    for (key, metrics) in &fci_metrics_map {
        println!("Key: {:?} -> Metrics: {:?}", key, metrics);
    }
    println!("--------------------------------\n");


    // 3. Mock Session Context and Daily Anchor (Inputs to Final Generator)
    let session_context_data = vec![
        SessionContextData { 
            trading_date: date, 
            asset_id: asset.clone(), 
            ps2_name: ps2_name.clone(), 
            ps1_name: ps1_name.clone(), 
            ps2_bias: ps2_bias.clone(), 
            ps1_bias: ps1_bias.clone(), 
            cs_name, 
            cs_bias: target_bias, // Actual outcome
        }
    ];

    let mut daily_outcome_map = HashMap::new();
    daily_outcome_map.insert(
        (date, asset.clone()), 
        DailyOutcomeValue { 
            day_outcome: "Bullish".to_string(), 
            day_confidence: 85.5 
        }
    );


    // 4. Generate the Final Feature Vector
    let feature_vectors = generate_session_feature_vector(
        session_context_data, 
        fci_metrics_map, 
        daily_outcome_map
    );

    println!("--- FINAL SESSION FEATURE VECTOR (Model 2 Input) ---");
    for vector in feature_vectors {
        println!("{:#?}", vector);
    }
}


// -------------------------------

use chrono::NaiveDate;
use std::collections::HashMap;

// --- 1. CORE DATA STRUCTURES (Inputs/Targets) ---

// Key for daily/asset/pattern-specific lookups (6 elements)
pub type FciLookupKey = (NaiveDate, String, String, String, String, String); 
// (date, asset, ps2_name, ps1_name, ps2_bias, ps1_bias)

// Key for timeless/pattern-specific lookups (5 elements - used for TCS/P_Cont)
pub type PatternLookupKey = (String, String, String, String, String); 
// (asset, ps2_name, ps1_name, ps2_bias, ps1_bias)

/// Placeholder for the raw 2nd-Order Transition data (P_conditional)
/// NOTE: This data *must* be keyed by date to be used for daily feature vectors.
#[derive(Debug, Clone)]
pub struct Transition2ndOrder {
    pub trading_date: NaiveDate, // Retained: Needed for daily aggregation
    pub asset_id: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,
    pub cs_name: String,        
    pub cs_bias: String,        
    pub p_transition_conditional: f64,
}

/// The calculated raw P_Continuation score (Timeless/Aggregated metric)
#[derive(Debug, Clone)]
pub struct TcsContinuationScore {
    pub asset_id: String,
    pub ps2_name: String,
    pub ps1_name: String,
    pub ps2_bias: String,   
    pub ps1_bias: String,   
    pub p_continuation: f64, // The raw probability (P_Continuation)
}

/// The calculated TCS multiplier score (Timeless/Aggregated metric)
#[derive(Debug, Clone)]
pub struct Tcs2ndOrder {
    pub asset_id: String,
    pub ps2_name: String,
    pub ps1_name: String,
    pub ps2_bias: String,   
    pub ps1_bias: String,   
    pub tcs_score: f64,     // The calculated TCS multiplier score (P_cond / P_base)
}

/// The core historical log structure (now using Option<String> for safety)
#[derive(Debug, Clone)]
pub struct SessionContextData {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    // Using Option<String> as per your original structure
    pub ps2_name: Option<String>,
    pub ps2_bias: Option<String>,
    pub ps1_name: Option<String>,
    pub ps1_bias: Option<String>,
    pub cs_name: Option<String>,
    pub cs_bias: Option<String>,
}

/// Placeholder for the aggregated daily outcome (proxy for Model 1's output)
#[derive(Debug, Clone)]
pub struct DailyOutcomeValue {
    pub day_outcome: String,
    pub day_confidence: f64,
}

// --- 2. FCI LOOKUP STRUCTURES ---

/// Structure to hold the aggregated FCI metrics for a single pattern key (the Value in the HashMap).
#[derive(Debug, Clone)]
pub struct FciMetricsForSession {
    pub long_conditional_prob: f64, 
    pub short_conditional_prob: f64, 
    pub tcs_score: f64,
    pub continuation_prob: f64,
}


// --- 3. HELPER LOOKUP BUILDERS ---

/// Builds a lookup map for the TCS Continuation Score (P_Continuation)
/// Uses the 5-element PatternLookupKey as these scores are *timeless* aggregates.
fn build_continuation_lookup_map(
    scores: &Vec<TcsContinuationScore>
) -> HashMap<PatternLookupKey, f64> {

    scores.iter()
        .map(|tcs| {
            let key = (
                tcs.asset_id.clone(),
                tcs.ps2_name.clone(),
                tcs.ps1_name.clone(),
                tcs.ps2_bias.clone(),
                tcs.ps1_bias.clone(),
            );
            (key, tcs.p_continuation)
        })
        .collect()
}

/// Builds a lookup map for the raw TCS multiplier score
/// Uses the 5-element PatternLookupKey as these scores are *timeless* aggregates.
fn build_tcs_score_lookup_map(
    scores: &Vec<Tcs2ndOrder>
) -> HashMap<PatternLookupKey, f64> {

    scores.iter()
        .map(|tcs| {
            let key = (
                tcs.asset_id.clone(),
                tcs.ps2_name.clone(),
                tcs.ps1_name.clone(),
                tcs.ps2_bias.clone(),
                tcs.ps1_bias.clone(),
            );
            (key, tcs.tcs_score)
        })
        .collect()
}


// --- 4. CORE FCI LOOKUP BUILDER (MAIN JOIN LOGIC) ---

/// Aggregates all Transition2ndOrder entries into a single FciMetricsForSession struct per pattern.
/// NOTE: This function now combines data from daily (transitions) and timeless (TCS/Continuation) sources.
pub fn build_fci_lookup_map_for_session(
    transitions: &Vec<Transition2ndOrder>,
    tcs_scores: &Vec<Tcs2ndOrder>,
    continuation_scores: &Vec<TcsContinuationScore>,
) -> HashMap<FciLookupKey, FciMetricsForSession> {
    
    // 1. Create Lookup Maps for auxiliary (timeless) data
    let tcs_map = build_tcs_score_lookup_map(tcs_scores);
    let cont_map = build_continuation_lookup_map(continuation_scores);

    // 2. Group Transition2ndOrder results by the unique pattern (6-element key)
    let mut grouped_results: HashMap<FciLookupKey, Vec<&Transition2ndOrder>> = HashMap::new();
    
    for t in transitions {
        let key = (
            t.trading_date,
            t.asset_id.clone(),
            t.ps2_name.clone(),
            t.ps1_name.clone(),
            t.ps2_bias.clone(),
            t.ps1_bias.clone(),
        );
        grouped_results.entry(key).or_insert_with(Vec::new).push(t);
    }
    
    // 3. Calculate MAX Long/Short P_cond and join auxiliary scores
    grouped_results.into_iter().filter_map(|(fci_key, transitions)| {
        let mut max_long_p = 0.0;
        let mut max_short_p = 0.0;
        
        // Find MAX P_conditional for Long/Short groups based on CS bias
        for t in transitions {
            let p = t.p_transition_conditional;
            match t.cs_bias.as_str() {
                // Defines the Long Group (Bullish family)
                "Bullish" | "Bullish_Reversal" | "Consolidation-Up" | "Failed_Bearish" => max_long_p = f64::max(max_long_p, p), // FIX: Use f64::max
                // Defines the Short Group (Bearish family)
                "Bearish" | "Bearish_Reversal" | "Consolidation-Down" | "Failed_Bullish" => max_short_p = f64::max(max_short_p, p), // FIX: Use f64::max
                _ => {} // Ignore neutral/other biases for MAX calculation
            }
        }
        
        // 4. Extract the 5-element key from the 6-element FCI key for timeless lookup
        let (_, asset_id, ps2_name, ps1_name, ps2_bias, ps1_bias) = fci_key.clone();
        let pattern_key: PatternLookupKey = (asset_id, ps2_name, ps1_name, ps2_bias, ps1_bias);

        // 5. Look up TCS and P_Continuation using the 5-element key
        if let Some(&tcs_score) = tcs_map.get(&pattern_key) {
            let continuation_prob = *cont_map.get(&pattern_key).unwrap_or(&0.0);
            
            Some((fci_key, FciMetricsForSession {
                long_conditional_prob: max_long_p,
                short_conditional_prob: max_short_p,
                tcs_score,
                continuation_prob,
            }))
        } else {
            // Drop the entry if we can't find the TCS score (essential)
            None 
        }
    })
    .collect()
}


// --- 5. FINAL FEATURE VECTOR STRUCTURE AND GENERATION ---

/// The final feature vector for the ML model (Model 2: Session Momentum Predictor)
#[derive(Debug, Clone)]
pub struct SessionFeatureVector {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    
    // --- A. PATTERN CONTEXT (6-Element Key) ---
    pub ps2_name: String,        
    pub ps1_name: String,        
    pub ps2_bias: String,        
    pub ps1_bias: String,        
    
    // --- B. RAW PROBABILITIES & SCALED SCORES (FCI Signals) ---
    pub p_cs_long_cond: f64,
    pub p_cs_short_cond: f64,
    pub tcs_score: f64,
    pub p_tcs_continuation: f64,
    
    // --- C. THE DAILY ANCHOR (Prediction from Model 1 Proxy) ---
    pub ml_daily_prediction: String, 
    pub ml_daily_score: f64,         

    // --- TARGET (Y) ---
    pub target_cs_bias: String,
}

/// Orchestrates the final join to create the ML training dataset.
pub fn generate_session_feature_vector(
    session_context_data: Vec<SessionContextData>, // The raw historical session log
    fci_metrics_map: HashMap<FciLookupKey, FciMetricsForSession>,
    daily_outcome_map: HashMap<(NaiveDate, String), DailyOutcomeValue>, // ML Anchor Proxy
) -> Vec<SessionFeatureVector> {
    
    let mut feature_vectors = Vec::new();
    
    for session_context in session_context_data {
        // Safely unwrap all Optional fields using unwrap_or_default()
        let ps2_name = session_context.ps2_name.clone().unwrap_or_default();
        let ps1_name = session_context.ps1_name.clone().unwrap_or_default();
        let ps2_bias = session_context.ps2_bias.clone().unwrap_or_default();
        let ps1_bias = session_context.ps1_bias.clone().unwrap_or_default();
        let target_cs_bias = session_context.cs_bias.clone().unwrap_or_default();

        // Construct the 6-element key to lookup the FCI metrics
        let key: FciLookupKey = (
            session_context.trading_date, 
            session_context.asset_id.clone(),
            ps2_name.clone(),
            ps1_name.clone(),
            ps2_bias.clone(), 
            ps1_bias.clone()
        );

        if let Some(fci_metrics) = fci_metrics_map.get(&key) {
            
            // Construct the 2-element key to lookup the Daily Anchor
            let daily_anchor_key = (session_context.trading_date, session_context.asset_id.clone());
            
            if let Some(daily_outcome) = daily_outcome_map.get(&daily_anchor_key) {
                
                // Only push the vector if all key components are present (handled by unwrap_or_default())
                // and the lookups succeeded.
                let vector = SessionFeatureVector {
                    trading_date: session_context.trading_date,
                    asset_id: session_context.asset_id,
                    
                    ps2_name,
                    ps1_name,
                    ps2_bias,
                    ps1_bias,
                    
                    p_cs_long_cond: fci_metrics.long_conditional_prob,
                    p_cs_short_cond: fci_metrics.short_conditional_prob,
                    tcs_score: fci_metrics.tcs_score,
                    p_tcs_continuation: fci_metrics.continuation_prob,
                    
                    ml_daily_prediction: daily_outcome.day_outcome.clone(),
                    ml_daily_score: daily_outcome.day_confidence, 
                    
                    target_cs_bias, 
                };
                feature_vectors.push(vector);
            }
        }
    }

    feature_vectors
}

// ------------------------

use chrono::NaiveDate;
use std::collections::HashMap;

// --- 1. CORE DATA STRUCTURES (Inputs/Targets) ---

// Key for daily/asset/pattern-specific lookups (6 elements)
pub type FciLookupKey = (NaiveDate, String, String, String, String, String); 
// (date, asset, ps2_name, ps1_name, ps2_bias, ps1_bias)

// Key for timeless/pattern-specific lookups (5 elements)
pub type PatternLookupKey = (String, String, String, String, String); 
// (asset, ps2_name, ps1_name, ps2_bias, ps1_bias)

/// The core historical log structure (using Option<String> for safety)
#[derive(Debug, Clone)]
pub struct SessionContextData {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub ps2_name: Option<String>,
    pub ps2_bias: Option<String>,
    pub ps1_name: Option<String>,
    pub ps1_bias: Option<String>,
    pub cs_name: Option<String>,
    pub cs_bias: Option<String>,
}

/// Placeholder for the aggregated daily outcome (proxy for Model 1's output)
#[derive(Debug, Clone)]
pub struct DailyOutcomeValue {
    pub day_outcome: String,
    pub day_confidence: f64,
}


// --- 2. TIMELESS/AGGREGATED METRIC STRUCTS (Inputs to ML Pipeline) ---

/// P(CS Bias | PS2 Bias, PS1 Bias) - Timeless Aggregate
#[derive(Debug, Clone)]
pub struct Transition2ndOrder {
    pub asset_id: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,
    pub cs_bias: String,        
    pub p_transition_conditional: f64,
}

/// The calculated raw P_Continuation score - Timeless Aggregate
#[derive(Debug, Clone)]
pub struct TcsContinuationScore {
    pub asset_id: String,
    pub ps2_name: String,
    pub ps1_name: String,
    pub ps2_bias: String,   
    pub ps1_bias: String,   
    pub p_continuation: f64, 
}

/// The calculated TCS multiplier score - Timeless Aggregate
#[derive(Debug, Clone)]
pub struct Tcs2ndOrder {
    pub asset_id: String,
    pub ps2_name: String,
    pub ps1_name: String,
    pub ps2_bias: String,   
    pub ps1_bias: String,   
    pub tcs_score: f64,     
}


// --- 3. Mock Calculation Function (Adapted to use Timeless Struct) ---

/// MOCK: Represents the result of an aggregation function (like your aggregate_weighted_counts)
/// The resulting vector is now a timeless statistic.
pub fn calculate_transition_2nd_order(
    // session_contexts: &[SessionContextData], // Assuming these are used inside aggregate_weighted_counts
    // intervals: &[LookbackInterval], // Assuming these are used inside aggregate_weighted_counts
    // min_attempts: f64, // Assuming these are used inside aggregate_weighted_counts
    mock_results: Vec<(PatternLookupKey, String, f64, f64)> // (pattern_key, cs_bias, success, total)
) -> Vec<Transition2ndOrder> {
    
    mock_results.into_iter().map(|(pattern_key, cs_bias, success, total)| {
        
        let (asset_id, ps2_name, ps1_name, ps2_bias, ps1_bias) = pattern_key; 
        
        let p_conditional = success / total;
        
        Transition2ndOrder {
            asset_id,
            ps2_bias,
            ps1_bias,
            cs_bias,
            ps2_name, 
            ps1_name,
            p_transition_conditional: (p_conditional * 100.0).round() / 100.0,
        }
    })
    .collect()
}


// --- 4. FCI LOOKUP STRUCTURES AND BUILDERS (The Joining Logic) ---

/// Structure to hold the aggregated FCI metrics for a single pattern key (the Value in the HashMap).
#[derive(Debug, Clone)]
pub struct FciMetricsForSession {
    pub long_conditional_prob: f64, 
    pub short_conditional_prob: f64, 
    pub tcs_score: f64,
    pub continuation_prob: f64,
}

/// Helper: Builds a lookup map for the TCS Continuation Score (P_Continuation)
fn build_continuation_lookup_map(
    scores: &Vec<TcsContinuationScore>
) -> HashMap<PatternLookupKey, f64> {

    scores.iter()
        .map(|tcs| {
            let key = (
                tcs.asset_id.clone(),
                tcs.ps2_name.clone(),
                tcs.ps1_name.clone(),
                tcs.ps2_bias.clone(),
                tcs.ps1_bias.clone(),
            );
            (key, tcs.p_continuation)
        })
        .collect()
}

/// Helper: Builds a lookup map for the raw TCS multiplier score
fn build_tcs_score_lookup_map(
    scores: &Vec<Tcs2ndOrder>
) -> HashMap<PatternLookupKey, f64> {

    scores.iter()
        .map(|tcs| {
            let key = (
                tcs.asset_id.clone(),
                tcs.ps2_name.clone(),
                tcs.ps1_name.clone(),
                tcs.ps2_bias.clone(),
                tcs.ps1_bias.clone(),
            );
            (key, tcs.tcs_score)
        })
        .collect()
}


/// Aggregates all Transition2ndOrder entries into a single FciMetricsForSession struct per pattern.
pub fn build_fci_lookup_map_for_session(
    transitions: &Vec<Transition2ndOrder>,
    tcs_scores: &Vec<Tcs2ndOrder>,
    continuation_scores: &Vec<TcsContinuationScore>,
    all_dates: &Vec<NaiveDate>, // Add all dates for cartesian product lookup
) -> HashMap<FciLookupKey, FciMetricsForSession> {
    
    // 1. Create Lookup Maps for auxiliary (timeless) data
    let tcs_map = build_tcs_score_lookup_map(tcs_scores);
    let cont_map = build_continuation_lookup_map(continuation_scores);
    
    // 2. Group Transition2ndOrder results by the 5-element PatternLookupKey
    let mut pattern_metrics: HashMap<PatternLookupKey, (f64, f64)> = HashMap::new(); // (max_long_p, max_short_p)

    for t in transitions {
        let key: PatternLookupKey = (
            t.asset_id.clone(), t.ps2_name.clone(), t.ps1_name.clone(), t.ps2_bias.clone(), t.ps1_bias.clone()
        );
        
        let (max_long_p, max_short_p) = pattern_metrics.entry(key).or_insert((0.0, 0.0));
        let p = t.p_transition_conditional;

        match t.cs_bias.as_str() {
            "Bullish" | "Bullish_Reversal" | "Consolidation-Up" | "Failed_Bearish" => 
                *max_long_p = f64::max(*max_long_p, p),
            "Bearish" | "Bearish_Reversal" | "Consolidation-Down" | "Failed_Bullish" => 
                *max_short_p = f64::max(*max_short_p, p),
            _ => {}
        }
    }
    
    // 3. Create the final 6-element FCI Lookup Map (Date-specific lookup)
    // This assumes the timeless metric is valid for all dates in the provided 'all_dates' list.
    let mut fci_map: HashMap<FciLookupKey, FciMetricsForSession> = HashMap::new();

    for date in all_dates {
        for (pattern_key, (max_long_p, max_short_p)) in &pattern_metrics {
            
            // Extract components for timeless lookups
            let (asset_id, ps2_name, ps1_name, ps2_bias, ps1_bias) = pattern_key.clone();
            
            // Lookup TCS and Continuation (still using the 5-element key)
            if let Some(&tcs_score) = tcs_map.get(pattern_key) {
                let continuation_prob = *cont_map.get(pattern_key).unwrap_or(&0.0);
                
                // Create the final 6-element key
                let fci_key: FciLookupKey = (
                    *date, asset_id.clone(), ps2_name.clone(), ps1_name.clone(), ps2_bias.clone(), ps1_bias.clone()
                );

                fci_map.insert(fci_key, FciMetricsForSession {
                    long_conditional_prob: *max_long_p,
                    short_conditional_prob: *max_short_p,
                    tcs_score,
                    continuation_prob,
                });
            }
        }
    }

    fci_map
}


// --- 5. FINAL FEATURE VECTOR STRUCTURE AND GENERATION ---

/// The final feature vector for the ML model (Model 2: Session Momentum Predictor)
#[derive(Debug, Clone)]
pub struct SessionFeatureVector {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    
    // --- A. PATTERN CONTEXT (6-Element Key) ---
    pub ps2_name: String,        
    pub ps1_name: String,        
    pub ps2_bias: String,        
    pub ps1_bias: String,        
    
    // --- B. RAW PROBABILITIES & SCALED SCORES (FCI Signals) ---
    pub p_cs_long_cond: f64,
    pub p_cs_short_cond: f64,
    pub tcs_score: f64,
    pub p_tcs_continuation: f64,
    
    // --- C. THE DAILY ANCHOR (Prediction from Model 1 Proxy) ---
    pub ml_daily_prediction: String, 
    pub ml_daily_score: f64,         

    // --- TARGET (Y) ---
    pub target_cs_bias: String,
}

/// Orchestrates the final join to create the ML training dataset.
pub fn generate_session_feature_vector(
    session_context_data: Vec<SessionContextData>, // The raw historical session log
    fci_metrics_map: HashMap<FciLookupKey, FciMetricsForSession>,
    daily_outcome_map: HashMap<(NaiveDate, String), DailyOutcomeValue>, // ML Anchor Proxy
) -> Vec<SessionFeatureVector> {
    
    let mut feature_vectors = Vec::new();
    
    for session_context in session_context_data {
        // Safely unwrap all Optional fields using unwrap_or_default()
        let ps2_name = session_context.ps2_name.clone().unwrap_or_default();
        let ps1_name = session_context.ps1_name.clone().unwrap_or_default();
        let ps2_bias = session_context.ps2_bias.clone().unwrap_or_default();
        let ps1_bias = session_context.ps1_bias.clone().unwrap_or_default();
        let target_cs_bias = session_context.cs_bias.clone().unwrap_or_default();

        // Construct the 6-element key to lookup the FCI metrics
        let key: FciLookupKey = (
            session_context.trading_date, 
            session_context.asset_id.clone(),
            ps2_name.clone(),
            ps1_name.clone(),
            ps2_bias.clone(), 
            ps1_bias.clone()
        );

        if let Some(fci_metrics) = fci_metrics_map.get(&key) {
            
            // Construct the 2-element key to lookup the Daily Anchor
            let daily_anchor_key = (session_context.trading_date, session_context.asset_id.clone());
            
            if let Some(daily_outcome) = daily_outcome_map.get(&daily_anchor_key) {
                
                let vector = SessionFeatureVector {
                    trading_date: session_context.trading_date,
                    asset_id: session_context.asset_id,
                    
                    ps2_name,
                    ps1_name,
                    ps2_bias,
                    ps1_bias,
                    
                    p_cs_long_cond: fci_metrics.long_conditional_prob,
                    p_cs_short_cond: fci_metrics.short_conditional_prob,
                    tcs_score: fci_metrics.tcs_score,
                    p_tcs_continuation: fci_metrics.continuation_prob,
                    
                    ml_daily_prediction: daily_outcome.day_outcome.clone(),
                    ml_daily_score: daily_outcome.day_confidence, 
                    
                    target_cs_bias, 
                };
                feature_vectors.push(vector);
            }
        }
    }

    feature_vectors
}