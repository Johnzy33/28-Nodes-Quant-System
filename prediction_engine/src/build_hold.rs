
use chrono::NaiveDate;
use std::collections::HashMap;

use shared_models::models::{
    Transition2ndOrder,
    Tcs2ndOrder,
    TcsContinuationScore,
    FciMetricsForSession, HighLowSession2ndOrder
};

// The key used for all lookups related to a specific session transition pattern
pub type FciLookupKey = (NaiveDate, String, String, String, String, String); 
pub type PatternLookupKey = (String, String, String, String, String); 

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

pub type PatternKey5 = (String, String, String, String, String); 

/// Builds a lookup map for High/Low conditional probabilities.
/// Key: PatternKey5 -> HashMap<("High" or "Low", Session Name), Probability>
fn build_high_low_lookup_map(
    scores: &Vec<HighLowSession2ndOrder>
) -> HashMap<PatternKey5, HashMap<(String, String), f64>> {
    
    let mut map: HashMap<PatternKey5, HashMap<(String, String), f64>> = HashMap::new();

    for score in scores {
        // Use a 5-element key that excludes the dynamic Session_Extreme
        let pattern_key: PatternKey5 = (
            score.asset_id.clone(),
            score.ps2_name.clone(),
            score.ps2_bias.clone(),
            score.ps1_name.clone(),
            score.ps1_bias.clone(),
        );

        let extreme_key = (score.session_extreme.clone(), score.extreme_session_name.clone());
        let prob = score.p_extreme_session_conditional;

        map.entry(pattern_key)
           .or_insert_with(HashMap::new)
           .insert(extreme_key, prob);
    }
    map
}

#[derive(Debug, Clone)]
pub struct FlattenedExtremeProbs {
    // Unique identifier
    pub asset_id: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,

    // The 10 fixed features for High/Low session probabilities
    pub p_high_as: f64,
    pub p_high_ln: f64,
    pub p_high_nyam: f64,
    pub p_high_nyl: f64,
    pub p_high_nypm: f64,

    pub p_low_as: f64,
    pub p_low_ln: f64,
    pub p_low_nyam: f64,
    pub p_low_nyl: f64,
    pub p_low_nypm: f64,
}

pub fn flatten_high_low_probs(
    high_low_data: Vec<HighLowSession2ndOrder>,
) -> Vec<FlattenedExtremeProbs> {
    
    // 1. Organize data into a lookup map
    let pattern_prob_map = build_high_low_lookup_map(&high_low_data);

    // 2. Iterate through patterns and flatten the results
    let mut final_vectors = Vec::new();
    
    for (pattern_key, probs) in pattern_prob_map {
        let (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias) = pattern_key;
        
        let mut vector = FlattenedExtremeProbs {
            asset_id,
            ps2_name,
            ps2_bias,
            ps1_name,
            ps1_bias,
            // Initialize all 10 feature values to 0.0 (default)
            p_high_as: 0.0, p_high_ln: 0.0, p_high_nyam: 0.0, p_high_nyl: 0.0, p_high_nypm: 0.0,
            p_low_as: 0.0, p_low_ln: 0.0, p_low_nyam: 0.0, p_low_nyl: 0.0, p_low_nypm: 0.0,
        };

        // 3. Populate the fixed feature fields
        for ((extreme, name), prob) in probs {
            match (extreme.as_str(), name.as_str()) {
                ("High", "AS") => vector.p_high_as = prob,
                ("High", "LN") => vector.p_high_ln = prob,
                ("High", "NYAM") => vector.p_high_nyam = prob,
                ("High", "NYL") => vector.p_high_nyl = prob,
                ("High", "NYPM") => vector.p_high_nypm = prob,
                ("Low", "AS") => vector.p_low_as = prob,
                ("Low", "LN") => vector.p_low_ln = prob,
                ("Low", "NYAM") => vector.p_low_nyam = prob,
                ("Low", "NYL") => vector.p_low_nyl = prob,
                ("Low", "NYPM") => vector.p_low_nypm = prob,
                _ => {} // Handle unexpected Session Names gracefully
            }
        }
        final_vectors.push(vector);
    }
    
    final_vectors
}


// --- 4. CORE FCI LOOKUP BUILDER (MAIN JOIN LOGIC) ---

/// Aggregates all Transition2ndOrder entries into a single FciMetricsForSession struct per pattern.
// pub fn build_fci_lookup_map_for_session(
//     transitions: &Vec<Transition2ndOrder>,
//     tcs_scores: &Vec<Tcs2ndOrder>,
//     continuation_scores: &Vec<TcsContinuationScore>,
// ) -> HashMap<FciLookupKey, FciMetricsForSession> {
    
//     // 1. Create Lookup Maps for auxiliary (timeless) data
//     let tcs_map = build_tcs_score_lookup_map(tcs_scores);
//     let cont_map = build_continuation_lookup_map(continuation_scores);

//     // 2. Group Transition2ndOrder results by the unique pattern (6-element key)
//     let mut grouped_results: HashMap<FciLookupKey, Vec<&Transition2ndOrder>> = HashMap::new();
    
//     for t in transitions {
//         let key = (
//            // t.trading_date,
//             t.asset_id.clone(),
//             t.ps2_name.clone(),
//             t.ps1_name.clone(),
//             t.ps2_bias.clone(),
//             t.ps1_bias.clone(),
//         );
//         grouped_results.entry(key).or_insert_with(Vec::new).push(t);
//     }
    
//     // 3. Calculate MAX Long/Short P_cond and join auxiliary scores
//     grouped_results.into_iter().filter_map(|(fci_key, transitions)| {
//         let mut max_long_p = 0.0;
//         let mut max_short_p = 0.0;
        
//         // Find MAX P_conditional for Long/Short groups based on CS bias
//         for t in transitions {
//             let p = t.p_transition_conditional;
//             match t.cs_bias.as_str() {
//                 // Defines the Long Group (Bullish family)
//                 "Bullish" | "Bullish_Reversal" | "Consolidation-Up" | "Failed_Bearish" => max_long_p = f64::max(max_long_p, p), // FIX: Use f64::max
//                 // Defines the Short Group (Bearish family)
//                 "Bearish" | "Bearish_Reversal" | "Consolidation-Down" | "Failed_Bullish" => max_short_p = f64::max(max_short_p, p), // FIX: Use f64::max
//                 _ => {} // Ignore neutral/other biases for MAX calculation
//             }
//         }
        
//         // 4. Extract the 5-element key from the 6-element FCI key for timeless lookup
//         let (_, asset_id, ps2_name, ps1_name, ps2_bias, ps1_bias) = fci_key.clone();
//         let pattern_key: PatternLookupKey = (asset_id, ps2_name, ps1_name, ps2_bias, ps1_bias);

//         // 5. Look up TCS and P_Continuation using the 5-element key
//         if let Some(&tcs_score) = tcs_map.get(&pattern_key) {
//             let continuation_prob = *cont_map.get(&pattern_key).unwrap_or(&0.0);
            
//             Some((fci_key, FciMetricsForSession {
//                 long_conditional_prob: max_long_p,
//                 short_conditional_prob: max_short_p,
//                 tcs_score,
//                 continuation_prob,
//             }))
//         } else {
//             // Drop the entry if we can't find the TCS score (essential)
//             None 
//         }
//     })
//     .collect()
// }

// -----

pub fn build_fci_lookup_map_for_session(
    transitions: &Vec<Transition2ndOrder>,
    tcs_scores: &Vec<Tcs2ndOrder>,
    continuation_scores: &Vec<TcsContinuationScore>,
    all_dates: &Vec<NaiveDate>, 
) -> HashMap<FciLookupKey, FciMetricsForSession> {
    
    // 1. Create Lookup Maps for auxiliary (timeless) data (using 5-element key)
    let tcs_map = build_tcs_score_lookup_map(tcs_scores);
    let cont_map = build_continuation_lookup_map(continuation_scores);
    
    // 2. Group Transition2ndOrder results by the 5-element PatternLookupKey 
    // to calculate the MAX P_long and MAX P_short for each unique pattern.
    let mut pattern_metrics: HashMap<PatternLookupKey, (f64, f64)> = HashMap::new(); // (max_long_p, max_short_p)

    for t in transitions {
        let key: PatternLookupKey = (
            t.asset_id.clone(), t.ps2_name.clone(), t.ps1_name.clone(), t.ps2_bias.clone(), t.ps1_bias.clone()
        );
        
        let (max_long_p, max_short_p) = pattern_metrics.entry(key).or_insert((0.0, 0.0));
        let p = t.p_transition_conditional;

        // Using f64::max for reliable float comparison
        match t.cs_bias.as_str() {
            "Bullish" | "Bullish_Reversal" | "Consolidation-Up" | "Failed_Bearish" => 
                *max_long_p = f64::max(*max_long_p, p),
            "Bearish" | "Bearish_Reversal" | "Consolidation-Down" | "Failed_Bullish" => 
                *max_short_p = f64::max(*max_short_p, p),
            _ => {}
        }
    }
    
    // 3. Create the final 6-element FCI Lookup Map (Date-specific lookup)
    let mut fci_map: HashMap<FciLookupKey, FciMetricsForSession> = HashMap::new();

    for date in all_dates {
        for (pattern_key, (max_long_p, max_short_p)) in &pattern_metrics {
            
            // Extract components for timeless lookups
            let (asset_id, ps2_name, ps1_name, ps2_bias, ps1_bias) = pattern_key.clone();
            
            // Lookup TCS and Continuation (still using the 5-element key)
            if let Some(&tcs_score) = tcs_map.get(pattern_key) {
                let continuation_prob = *cont_map.get(pattern_key).unwrap_or(&0.0);
                
                // Create the final 6-element key for the output map
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

