use crate::signal_type::{MarketClassification, MarketSubtype, MarketType}; // Import from signal_type.rs
use std::f64;

// --- System Thresholds (Constants) ---
const CONSOLIDATION_BODY_RATIO: f64 = 0.20;
const OPPOSING_WICK_PRESSURE: f64 = 0.40;
const REVERSAL_WICK_DOMINANCE: f64 = 2.0;
const BALANCED_WICK_RATIO: f64 = 0.50;
const EPSILON: f64 = 1e-8;
const SAFE_BODY_MIN: f64 = 1e-12; 


// pub fn get_market_classification(
//     open_val: f64, 
//     high_val: f64, 
//     low_val: f64, 
//     close_val: f64
// ) -> MarketClassification {
    
//     let session_range = high_val - low_val;
    
//     //  Handle Edge Cases
//     if session_range <= EPSILON {
//         return MarketClassification { 
//             classification: MarketType::Consolidation, 
//             subtype: MarketSubtype::PureIndecision 
//         };
//     }

//     // Calculate Core Metrics
//     let body_size = (close_val - open_val).abs();
//     let real_body_high = close_val.max(open_val);
//     let real_body_low = close_val.min(open_val);
//     let upper_wick = high_val - real_body_high;
//     let lower_wick = real_body_low - low_val;

//     // Calculate Ratios 
//     let safe_range = session_range.max(EPSILON);
//     let body_range_ratio = body_size / safe_range;
//     let upper_wick_ratio = upper_wick / safe_range;
//     let lower_wick_ratio = lower_wick / safe_range;
    
//     // =======================================================
//     //  TREND CLASSIFICATION (BULLISH/BEARISH) - Immediate Return if Confirmed
//     // =======================================================

//     if close_val > open_val { 
    
//         if upper_wick_ratio < OPPOSING_WICK_PRESSURE && body_range_ratio >= CONSOLIDATION_BODY_RATIO {
//             return MarketClassification { 
//                 classification: MarketType::Bullish, 
//                 subtype: MarketSubtype::Confirmed 
//             };
//         }
//     } else if close_val < open_val { 
        
//         if lower_wick_ratio < OPPOSING_WICK_PRESSURE && body_range_ratio >= CONSOLIDATION_BODY_RATIO {
//             return MarketClassification { 
//                 classification: MarketType::Bearish, 
//                 subtype: MarketSubtype::Confirmed 
//             };
//         }
//     }

//     // =======================================================
//     //  CONSOLIDATION CLASSIFICATION (Small Body Check)
//     // =======================================================

    

//     if body_range_ratio < CONSOLIDATION_BODY_RATIO {
        
//         let safe_body = body_size.max(SAFE_BODY_MIN); 

        
//         if lower_wick >= REVERSAL_WICK_DOMINANCE * safe_body {
//             return MarketClassification { 
//                 classification: MarketType::Consolidation, 
//                 subtype: MarketSubtype::BullishReversal 
//             };
//         } else if upper_wick >= REVERSAL_WICK_DOMINANCE * safe_body {
//             return MarketClassification { 
//                 classification: MarketType::Consolidation, 
//                 subtype: MarketSubtype::BearishReversal 
//             };
//         }

//         // Pure indecision 
//         let min_wick = upper_wick.min(lower_wick);
//         let max_wick = upper_wick.max(lower_wick);
        
//         if min_wick >= BALANCED_WICK_RATIO * max_wick { 
//             return MarketClassification { 
//                 classification: MarketType::Consolidation, 
//                 subtype: MarketSubtype::PureIndecision 
//             };
//         }

//         // Fallback for small body
//         return MarketClassification { 
//             classification: MarketType::Consolidation, 
//             subtype: MarketSubtype::PureIndecision 
//         };
//     }

//     // =======================================================
//     //  FAILED TREND / VETOED TREND (Large Body, Large Opposing Wick)
//     // =======================================================
    
    
    
//     if upper_wick_ratio >= OPPOSING_WICK_PRESSURE {
//         // Bullish move was vetoed by the upper wick (sellers)
//         return MarketClassification { 
//             classification: MarketType::Consolidation, 
//             subtype: MarketSubtype::FailedBullish 
//         };
//     } else if lower_wick_ratio >= OPPOSING_WICK_PRESSURE {
//         // Bearish move was vetoed by the lower wick (buyers)
//         return MarketClassification { 
//             classification: MarketType::Consolidation, 
//             subtype: MarketSubtype::FailedBearish 
//         };
//     }
    
//     // Final fallback (Should theoretically be unreachable if logic is perfect, but kept for robustness)
//     MarketClassification { 
//         classification: MarketType::Consolidation, 
//         subtype: MarketSubtype::Other 
//     }
// }

pub fn get_market_classification(
    open_val: f64, 
    high_val: f64, 
    low_val: f64, 
    close_val: f64
) -> MarketType { // Changed return type to MarketType directly
    
    let session_range = high_val - low_val;
    
    //  Handle Edge Cases
    if session_range <= EPSILON {
        return MarketType::PureIndecision;
    }

    // Calculate Core Metrics
    let body_size = (close_val - open_val).abs();
    let real_body_high = close_val.max(open_val);
    let real_body_low = close_val.min(open_val);
    let upper_wick = high_val - real_body_high;
    let lower_wick = real_body_low - low_val;

    // Calculate Ratios 
    let safe_range = session_range.max(EPSILON);
    let body_range_ratio = body_size / safe_range;
    let upper_wick_ratio = upper_wick / safe_range;
    let lower_wick_ratio = lower_wick / safe_range;
    
    // =======================================================
    //  TREND CLASSIFICATION (BULLISH/BEARISH)
    // =======================================================

    if close_val > open_val { 
        // Confirmed Bullish
        if upper_wick_ratio < OPPOSING_WICK_PRESSURE && body_range_ratio >= CONSOLIDATION_BODY_RATIO {
            return MarketType::Bullish; 
        }
    } else if close_val < open_val { 
        // Confirmed Bearish
        if lower_wick_ratio < OPPOSING_WICK_PRESSURE && body_range_ratio >= CONSOLIDATION_BODY_RATIO {
            return MarketType::Bearish; 
        }
    }

    // =======================================================
    //  CONSOLIDATION / REVERSAL CLASSIFICATION (Small Body Check)
    // =======================================================

    if body_range_ratio < CONSOLIDATION_BODY_RATIO {
        
        let safe_body = body_size.max(SAFE_BODY_MIN); 

        // Bullish Reversal
        if lower_wick >= REVERSAL_WICK_DOMINANCE * safe_body {
            return MarketType::BullishReversal; 
        } 
        
        // Bearish Reversal
        else if upper_wick >= REVERSAL_WICK_DOMINANCE * safe_body {
            return MarketType::BearishReversal; 
        }

        // Pure indecision (Balanced Wicks)
        let min_wick = upper_wick.min(lower_wick);
        let max_wick = upper_wick.max(lower_wick);
        
        if min_wick >= BALANCED_WICK_RATIO * max_wick { 
            return MarketType::PureIndecision; 
        }

        // Fallback for small body
        return MarketType::PureIndecision; 
    }

    // =======================================================
    //  FAILED TREND / VETOED TREND (Large Body, Large Opposing Wick)
    // =======================================================
    
    
    if upper_wick_ratio >= OPPOSING_WICK_PRESSURE {
        // Bullish move was vetoed by the upper wick (sellers)
        return MarketType::FailedBullish; 
    } else if lower_wick_ratio >= OPPOSING_WICK_PRESSURE {
        // Bearish move was vetoed by the lower wick (buyers)
        return MarketType::FailedBearish; 
    }
    
    // Final fallback
    MarketType::Other
}