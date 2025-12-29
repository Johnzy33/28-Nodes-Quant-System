
use chrono::NaiveDate;
use std::collections::HashMap;
use shared_models::models::{SessionFeatureVector, SessionContextData,FciMetricsForSession, DailyOutcomeValue, PcsScore2ndOrder};

// The key used for all lookups related to a specific session transition pattern
pub type FciLookupKey = (NaiveDate, String, String, String, String, String); 
pub type PatternKey5 = (String, String, String, String, String);

#[derive(Debug, Clone)]
pub struct FlattenedExtremeProbs {
    // Note: The rest of the fields are the 10 p_high/p_low features
    pub p_high_as: f64, pub p_high_ln: f64, pub p_high_nyam: f64, pub p_high_nyl: f64, pub p_high_nypm: f64,
    pub p_low_as: f64, pub p_low_ln: f64, pub p_low_nyam: f64, pub p_low_nyl: f64, pub p_low_nypm: f64,
}

#[derive(Debug, Clone)]
pub struct SessionFeatureVectorML {
    pub date: NaiveDate,
    pub asset_id: String,
    
    // The ML model features (X) go here. We'll use a Vec<f64> for simplicity.
    pub feature_vector: Vec<f64>,
    
    // The ML target label (Y) goes here (Label Encoded 0-6).
    pub target_label: i32, 
}

// Defines all 7 mutually exclusive bias states for OHE
const ALL_BIASES: [&str; 7] = [
    "Bullish", "Bearish", "BullishReversal", "BearishReversal", 
    "FailedBearish", "FailedBullish", "PureConsolidation"
];

// Helper function for OHE (based on your implementation)
fn one_hot_encode(value: &str, prefix: &str) -> Vec<f64> {
    let mut encoded_vec = Vec::with_capacity(ALL_BIASES.len());
    for state in ALL_BIASES.iter() {
        let is_match = if value == *state { 1.0 } else { 0.0 };
        encoded_vec.push(is_match);
    }
    encoded_vec
}

// pub fn generate_session_feature_vector(
//     session_context_data: Vec<SessionContextData>, // The raw historical session log
//     fci_metrics_map: HashMap<FciLookupKey, FciMetricsForSession>,
//     daily_outcome_map: HashMap<(NaiveDate, String), DailyOutcomeValue>, // ML Anchor Proxy
// ) -> Vec<SessionFeatureVector> {
    
//     let mut feature_vectors = Vec::new();
    
//     for session_context in session_context_data {
//         // Construct the 6-element key to lookup the FCI metrics
//         let key: FciLookupKey = (
//             session_context.trading_date, 
//             session_context.asset_id.clone(),
//             session_context.ps2_name.clone().unwrap_or_default(),
//             session_context.ps1_name.clone().unwrap_or_default(),
//             session_context.ps2_bias.clone().unwrap_or_default(), 
//             session_context.ps1_bias.clone().unwrap_or_default()
//         );

//         if let Some(fci_metrics) = fci_metrics_map.get(&key) {
            
//             // Construct the 2-element key to lookup the Daily Anchor
//             let daily_anchor_key = (session_context.trading_date, session_context.asset_id.clone());
            
//             if let Some(daily_outcome) = daily_outcome_map.get(&daily_anchor_key) {
                
//                 let vector = SessionFeatureVector {
//                     trading_date: session_context.trading_date,
//                     asset_id: session_context.asset_id,
//                     ps2_name: session_context.ps2_name.clone().unwrap_or_default(),
//                     ps1_name: session_context.ps1_name.clone().unwrap_or_default(),
//                     ps2_bias: session_context.ps2_bias.clone().unwrap_or_default(),
//                     ps1_bias: session_context.ps1_bias.clone().unwrap_or_default(),
                    
//                     p_cs_long_cond: fci_metrics.long_conditional_prob,
//                     p_cs_short_cond: fci_metrics.short_conditional_prob,
//                     tcs_score: fci_metrics.tcs_score,
//                     p_tcs_continuation: fci_metrics.continuation_prob,
                    
//                     ml_daily_prediction: daily_outcome.day_outcome.clone(),
//                     ml_daily_score: daily_outcome.day_confidence, 
                    
//                     target_cs_bias: session_context.cs_bias.clone().unwrap_or_default(),
//                 };
//                 feature_vectors.push(vector);
//             } else {
//                 eprintln!("Warning: Missing Daily Anchor for {:?}", daily_anchor_key);
//             }
//         } else {
//             eprintln!("Warning: Missing FCI Metrics for pattern {:?}", key);
//         }
//     }

//     feature_vectors
// }

pub fn generate_session_feature_vector(
    session_context_data: Vec<SessionContextData>, 
    fci_metrics_map: HashMap<FciLookupKey, FciMetricsForSession>,
    daily_outcome_map: HashMap<(NaiveDate, String), DailyOutcomeValue>, 
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
                    
                    ml_daily_prediction: daily_outcome.day_bias.clone(),
                    ml_daily_score: daily_outcome.day_confidence, 
                    
                    target_cs_bias, 
                };
                feature_vectors.push(vector);
            }
        }
    }

    feature_vectors
}


pub fn generate_ml_training_vectors(
    session_context_data: Vec<SessionContextData>, 
    fci_metrics_map: HashMap<FciLookupKey, FciMetricsForSession>,
    pcs_scores_map: HashMap<PatternKey5, PcsScore2ndOrder>,
    extreme_probs_map: HashMap<PatternKey5, FlattenedExtremeProbs>,
    daily_outcome_map: HashMap<(NaiveDate, String), DailyOutcomeValue>, 
) -> Vec<SessionFeatureVectorML> {
    
    let mut ml_vectors = Vec::new();

    // Map the target CS bias string to a numerical label (Label Encoding)
    let bias_to_label: HashMap<&str, i32> = ALL_BIASES.iter().enumerate()
        .map(|(i, &s)| (s, i as i32))
        .collect();
    
    for sc in session_context_data {
        // Safely unwrap all Optional fields and provide defaults
        let ps2_name = sc.ps2_name.clone().unwrap_or_default();
        let ps1_name = sc.ps1_name.clone().unwrap_or_default();
        let ps2_bias = sc.ps2_bias.clone().unwrap_or_default();
        let ps1_bias = sc.ps1_bias.clone().unwrap_or_default();
        let target_cs_bias = sc.cs_bias.clone().unwrap_or_default();
        
        // --- 1. Define Lookup Keys ---
        let fci_key: FciLookupKey = (sc.trading_date, sc.asset_id.clone(), ps2_name.clone(), ps2_bias.clone(), ps1_name.clone(), ps1_bias.clone());
        let pattern_key: PatternKey5 = (sc.asset_id.clone(), ps2_name.clone(), ps2_bias.clone(), ps1_name.clone(), ps1_bias.clone());
        let daily_key = (sc.trading_date, sc.asset_id.clone());

        // --- 2. Perform Lookups & Filter for Completeness ---
        if let (
            Some(fci_metrics), 
            Some(pcs_scores),
            Some(extreme_probs), 
            Some(daily_outcome),
            Some(target_label) // Must have a valid target bias
        ) = (
            fci_metrics_map.get(&fci_key),
            pcs_scores_map.get(&pattern_key),
            extreme_probs_map.get(&pattern_key),
            daily_outcome_map.get(&daily_key),
            bias_to_label.get(target_cs_bias.as_str()).cloned()
        ) {
            
            // --- 3. Assemble the Feature Vector (X) ---
            let mut feature_vec: Vec<f64> = Vec::new();

            // A. Add Core FCI/TCS Numeric Scores
            feature_vec.push(fci_metrics.long_conditional_prob);
            feature_vec.push(fci_metrics.short_conditional_prob);
            feature_vec.push(fci_metrics.tcs_score);
            feature_vec.push(fci_metrics.continuation_prob);

            // B. Add PCS (Dynamic Day Anchor) Numeric Scores
            feature_vec.push(pcs_scores.pcs_score);

            // C. Add Flattened High/Low Numeric Scores (10 features)
            feature_vec.push(extreme_probs.p_high_as);
            // ... push 8 more p_high/p_low features ...
            feature_vec.push(extreme_probs.p_low_nypm);

            // D. Add Daily Anchor Numeric Score
            feature_vec.push(daily_outcome.day_confidence);

            // E. Add OHE Categorical Features (PS2/PS1 Bias & PCS Day Type)
            // Note: You would perform OHE for PSX_Name here as well.
            feature_vec.extend(one_hot_encode(&ps2_bias, "ps2_bias")); // 7 features
            feature_vec.extend(one_hot_encode(&ps1_bias, "ps1_bias")); // 7 features
           // feature_vec.extend(one_hot_encode(&pcs_scores.pcs_day_type, "pcs_day_type")); // 7 features
            feature_vec.extend(one_hot_encode(&daily_outcome.day_bias, "ml_daily_prediction")); // 7 features

            // --- 4. Push the Final ML Vector ---
            ml_vectors.push(SessionFeatureVectorML {
                date: sc.trading_date,
                asset_id: sc.asset_id,
                feature_vector: feature_vec,
                target_label: target_label, 
            });
        }
    }

    ml_vectors
}