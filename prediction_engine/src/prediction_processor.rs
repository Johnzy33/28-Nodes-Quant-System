use crate::data_fetch_hold::{PcsScoreResult, LatestPatternKey}; 

// --- Data Structure 3: Final Aggregated Prediction ---
#[derive(Debug, Clone, PartialEq)]
pub struct FinalPrediction {
    pub asset_id: String,
    pub strongest_signal: String,      
    pub highest_pcs_score: f64,
    pub lookback_period: String,       
    pub order_type: String,            
    pub confidence_label: String,      
}

struct CombinedResult {
    result: PcsScoreResult,
    order_type: String,
}

/// Analyzes and aggregates all fetched PCS scores to find the single most confident signal.
pub fn aggregate_prediction_scores(
    keys: &LatestPatternKey,
    pcs1_results: Vec<PcsScoreResult>,
    pcs2_results: Vec<PcsScoreResult>,
) -> Option<FinalPrediction> {
    
    let mut combined_results: Vec<CombinedResult> = Vec::new();

    // 1. Process 2nd Order Results (potentially higher confidence)
    for res in pcs2_results {
        if res.daily_outcome_7 == "Bullish" || res.daily_outcome_7 == "Bearish" {
            combined_results.push(CombinedResult {
                result: res,
                order_type: "2nd_Order".to_string(),
            });
        }
    }

    // 2. Process 1st Order Results
    for res in pcs1_results {
        if res.daily_outcome_7 == "Bullish" || res.daily_outcome_7 == "Bearish" {
            combined_results.push(CombinedResult {
                result: res,
                order_type: "1st_Order".to_string(),
            });
        }
    }

    // 3. Find the best result by maximizing the PCS score
    let best_result = combined_results.into_iter()
        .max_by(|a, b| a.result.pcs_score.partial_cmp(&b.result.pcs_score).unwrap_or(std::cmp::Ordering::Equal));
    
    match best_result {
        Some(combined) => {
            let res = combined.result;
            Some(FinalPrediction {
                asset_id: keys.asset_id.clone(),
                strongest_signal: res.daily_outcome_7,
                highest_pcs_score: res.pcs_score,
                lookback_period: res.lookback_period,
                order_type: combined.order_type,
                confidence_label: res.insight_label,
            })
        }
        None => None,
    }
}