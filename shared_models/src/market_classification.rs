
use std::f64;
use sqlx::{Type}; 
use strum_macros::{Display, EnumString};
use serde::{Deserialize, Serialize};

// --- System Thresholds (Constants) ---
const CONSOLIDATION_BODY_RATIO: f64 = 0.20;
const OPPOSING_WICK_PRESSURE: f64 = 0.40;
const REVERSAL_WICK_DOMINANCE: f64 = 2.0;
const BALANCED_WICK_RATIO: f64 = 0.50;
const EPSILON: f64 = 1e-8;
const SAFE_BODY_MIN: f64 = 1e-12; 



#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash,EnumString,Display)]
#[derive(Type)]
#[derive(Default)]
#[sqlx(type_name = "TEXT")]
pub enum MarketType {
    #[default]
    Other,
    Bullish,
    Bearish,
    FailedBearish,
    FailedBullish,
    BullishReversal,
    BearishReversal,
    PureIndecision,
}


#[derive(Debug, Clone, Serialize, Deserialize,PartialEq,)]
pub struct MarketRatios {
    pub body_ratio: f64,
    pub up_wick_ratio: f64,
    pub lo_wick_ratio: f64,
    pub total_wick_ratio: f64,
    pub is_bullish_candle: bool,
    pub range: f64,
    pub body_size: f64,
}

impl MarketRatios {

    pub fn new(open: f64, high: f64, low: f64, close: f64) -> Self {
        let range = (high - low).max(1e-10);
        let body_size = (close - open).abs();
        let real_body_high = close.max(open);
        let real_body_low = close.min(open);

        let up_wick_ratio = (high - real_body_high) / range;
        let lo_wick_ratio = (real_body_low - low) / range;

        Self {
            body_ratio: body_size / range,
            up_wick_ratio,
            lo_wick_ratio,
            total_wick_ratio: up_wick_ratio + lo_wick_ratio,
            is_bullish_candle: close > open,
            range,
            body_size,
        }
    }
}


impl MarketType {
    pub fn get_classification(r: &MarketRatios) -> Self {
        if r.range <= 1e-10 { return MarketType::Other; }

        // --- 1. THE 25% GATE ---
        if r.body_ratio >= 0.25 {
            // Path A: Large Body
            let dynamic_multiplier = if r.body_ratio < 0.35 { 1.5 } 
                                     else if r.body_ratio < 0.50 { 2.0 } 
                                     else { 2.5 };

            if r.total_wick_ratio > (r.body_ratio * dynamic_multiplier) {
                return MarketType::PureIndecision;
            }

            if r.is_bullish_candle {
                if r.up_wick_ratio >= 0.30 { return MarketType::FailedBullish; }
                return MarketType::Bullish;
            } else {
                if r.lo_wick_ratio >= 0.30 { return MarketType::FailedBearish; }
                return MarketType::Bearish;
            }
        } else {
            // Path B: Small Body
            if r.lo_wick_ratio >= 0.60 { return MarketType::BullishReversal; }
            if r.up_wick_ratio >= 0.60 { return MarketType::BearishReversal; }

            if (r.up_wick_ratio - r.lo_wick_ratio).abs() < 0.15 {
                return MarketType::PureIndecision;
            }

            if r.lo_wick_ratio > r.up_wick_ratio { MarketType::BullishReversal }
            else { MarketType::BearishReversal }
        }
    }

    pub fn inverse(&self) -> Self {
        match self {
            MarketType::Bullish => MarketType::Bullish,
            MarketType::Bearish => MarketType::Bearish,
            MarketType::FailedBullish => MarketType::Bearish,
            MarketType::FailedBearish => MarketType::Bullish,
            MarketType::BullishReversal => MarketType::Bullish,
            MarketType::BearishReversal => MarketType::Bearish,
            _ => MarketType::PureIndecision,
        }
    }
}
// // In your shared_models or wherever MarketType is defined
// impl MarketType {
//     pub fn inverse(&self) -> Self {
//         match self {
//             MarketType::Bullish => MarketType::Bearish,
//             MarketType::Bearish => MarketType::Bullish,
//             MarketType::FailedBullish => MarketType::Bullish, // Failure of a fail is a return to trend
//             MarketType::FailedBearish => MarketType::Bearish,
//             _ => MarketType::PureIndecision, // Indecision has no inverse
//         }
//     }
// }

// impl MarketType {
//     pub fn inverse(&self) -> Self {
//         match self {
//             // Standard Trend Flips
//             MarketType::Bullish => MarketType::Bearish,
//             MarketType::Bearish => MarketType::Bullish,

//             // Traps: The "Inverse" of a Failed Bullish move is a Bearish bias
//             MarketType::FailedBullish => MarketType::Bearish,
//             MarketType::FailedBearish => MarketType::Bullish,

//             // Reversals: These already signal the turn. 
//             // The "Inverse" of a BullishReversal (bottoming out) is a Bullish bias.
//             MarketType::BullishReversal => MarketType::Bullish,
//             MarketType::BearishReversal => MarketType::Bearish,

//             // Neutral remains Neutral
//             _ => MarketType::PureIndecision,
//         }
//     }
// }

// impl MarketType {
//     pub fn inverse(&self) -> Self {
//         match self {
//             // Healthy Trends: Bias remains in the direction of conviction
//             MarketType::Bullish => MarketType::Bullish, 
//             MarketType::Bearish => MarketType::Bearish,

//             // Traps: Bias FLIPS because the move failed
//             MarketType::FailedBullish => MarketType::Bearish,
//             MarketType::FailedBearish => MarketType::Bullish,

//             // Reversals: Bias is the NEW direction indicated by the wick
//             MarketType::BullishReversal => MarketType::Bullish,
//             MarketType::BearishReversal => MarketType::Bearish,

//             _ => MarketType::PureIndecision,
//         }
//     }

//     pub fn ratios(open: f64, high: f64, low: f64, close: f64) -> Self {
//         let range = (high - low).max(1e-10);
//         let body_size = (close - open).abs();
//         let real_body_high = close.max(open);
//         let real_body_low = close.min(open);

//         let up_wick_ratio = (high - real_body_high) / range;
//         let lo_wick_ratio = (real_body_low - low) / range;

//         Self {
//             body_ratio: body_size / range,
//             up_wick_ratio,
//             lo_wick_ratio,
//             total_wick_ratio: up_wick_ratio + lo_wick_ratio,
//             is_bullish_candle: close > open,
//             range,
//             body_size,
//         }
//     }

//     pub fn get_market_classification(
//         open: f64, 
//         high: f64, 
//         low: f64, 
//         close: f64
//     ) -> MarketType {
//         let session_range = high - low;
        
//         // 1. Initial State / Edge Case Check
//         if session_range <= EPSILON { 
//             return MarketType::Other; 
//         } 

//         let body_size = (close - open).abs();
//         let body_ratio = body_size / session_range;
//         let real_body_high = close.max(open);
//         let real_body_low = close.min(open);
        
//         let upper_wick_ratio = (high - real_body_high) / session_range;
//         let lower_wick_ratio = (real_body_low - low) / session_range;
//         let total_wick_ratio = upper_wick_ratio + lower_wick_ratio;

//         // --- 2. THE 25% GATE: IS THERE ENOUGH INTENT? ---
//         if body_ratio >= 0.25 {
//             // --- PATH A: LARGE BODY (Potential Trends or Traps) ---

//             // DYNAMIC MULTIPLIER: Judge how much "noise" (wicks) we allow 
//             // before a large body is considered "Indecision".
//             let dynamic_multiplier = if body_ratio < 0.35 {
//                 1.5 // Tight: Small-ish bodies are easily disrupted by wicks
//             } else if body_ratio < 0.50 {
//                 2.0 // Moderate: Medium bodies need significant wicks to be Indecision
//             } else {
//                 2.5 // Loose: Large bodies are very hard to override
//             };

//             // Check for Indecision/High-Wave noise
//             if total_wick_ratio > (body_ratio * dynamic_multiplier) {
//                 return MarketType::PureIndecision;
//             }

//             // Check for Directional Failure vs Healthy Trend
//             if close > open {
//                 // It's a Green Candle. Did sellers push back > 30% from the high?
//                 if upper_wick_ratio >= 0.30 {
//                     return MarketType::FailedBullish;
//                 }
//                 return MarketType::Bullish;
//             } else {
//                 // It's a Red Candle. Did buyers push back > 30% from the low?
//                 if lower_wick_ratio >= 0.30 {
//                     return MarketType::FailedBearish;
//                 }
//                 return MarketType::Bearish;
//             }
//         } else {
//             // --- PATH B: SMALL BODY (Reversals or Equilibrium) ---
            
//             // Priority 1: High Conviction Reversal (Wick owns 60%+ of the candle)
//             if lower_wick_ratio >= 0.60 { return MarketType::BullishReversal; }
//             if upper_wick_ratio >= 0.60 { return MarketType::BearishReversal; }

//             // Priority 2: Equilibrium (Small body, balanced wicks)
//             let wick_diff = (upper_wick_ratio - lower_wick_ratio).abs();
//             if wick_diff < 0.15 {
//                 return MarketType::PureIndecision;
//             }

//             // Priority 3: Soft Reversal (Body is small, but one side is clearly winning the wick war)
//             if lower_wick_ratio > upper_wick_ratio {
//                 return MarketType::BullishReversal;
//             } else {
//                 return MarketType::BearishReversal;
//             }
//         }
//     }
// }


#[derive(Debug, PartialEq, Clone, Copy)]
pub struct MarketClassification {
    pub classification: MarketType,
}

// pub fn get_market_classification(
//     open_val: f64, 
//     high_val: f64, 
//     low_val: f64, 
//     close_val: f64
// ) -> MarketType { // Changed return type to MarketType directly
    
//     let session_range = high_val - low_val;
    
//     //  Handle Edge Cases
//     if session_range <= EPSILON {
//         return MarketType::PureIndecision;
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
//     //  TREND CLASSIFICATION (BULLISH/BEARISH)
//     // =======================================================

//     if close_val > open_val { 
//         // Confirmed Bullish
//         if upper_wick_ratio < OPPOSING_WICK_PRESSURE && body_range_ratio >= CONSOLIDATION_BODY_RATIO {
//             return MarketType::Bullish; 
//         }
//     } else if close_val < open_val { 
//         // Confirmed Bearish
//         if lower_wick_ratio < OPPOSING_WICK_PRESSURE && body_range_ratio >= CONSOLIDATION_BODY_RATIO {
//             return MarketType::Bearish; 
//         }
//     }

//     // =======================================================
//     //  CONSOLIDATION / REVERSAL CLASSIFICATION (Small Body Check)
//     // =======================================================

//     if body_range_ratio < CONSOLIDATION_BODY_RATIO {
        
//         let safe_body = body_size.max(SAFE_BODY_MIN); 

//         // Bullish Reversal
//         if lower_wick >= REVERSAL_WICK_DOMINANCE * safe_body {
//             return MarketType::BullishReversal; 
//         } 
        
//         // Bearish Reversal
//         else if upper_wick >= REVERSAL_WICK_DOMINANCE * safe_body {
//             return MarketType::BearishReversal; 
//         }

//         // Pure indecision (Balanced Wicks)
//         let min_wick = upper_wick.min(lower_wick);
//         let max_wick = upper_wick.max(lower_wick);
        
//         if min_wick >= BALANCED_WICK_RATIO * max_wick { 
//             return MarketType::PureIndecision; 
//         }

//         // Fallback for small body
//         return MarketType::PureIndecision; 
//     }

//     // =======================================================
//     //  FAILED TREND / VETOED TREND (Large Body, Large Opposing Wick)
//     // =======================================================
    
    
//     if upper_wick_ratio >= OPPOSING_WICK_PRESSURE {
//         // Bullish move was vetoed by the upper wick (sellers)
//         return MarketType::FailedBullish; 
//     } else if lower_wick_ratio >= OPPOSING_WICK_PRESSURE {
//         // Bearish move was vetoed by the lower wick (buyers)
//         return MarketType::FailedBearish; 
//     }
    
//     // Final fallback
//     MarketType::Other
// }


// pub fn get_market_classification(
//     open: f64, high: f64, low: f64, close: f64
// ) -> MarketType {
//     let session_range = high - low;
//     if session_range <= EPSILON { return MarketType::Other; } 

//     let body_size = (close - open).abs();
//     let body_ratio = body_size / session_range;
//     let real_body_high = close.max(open);
//     let real_body_low = close.min(open);
    
//     let upper_wick_ratio = (high - real_body_high) / session_range;
//     let lower_wick_ratio = (real_body_low - low) / session_range;

//     // --- 1. THE 25% GATE: DIRECTIONAL VS NON-DIRECTIONAL ---
    
//     if body_ratio >= 0.25 {
//         // --- PATH A: LARGE BODY (Strong Trends or Traps) ---
//         if close > open {
//             // It's Green. Does the upper wick "veto" it?
//             if upper_wick_ratio >= 0.30 {
//                 return MarketType::FailedBullish;
//             }
//             return MarketType::Bullish;
//         } else {
//             // It's Red. Does the lower wick "veto" it?
//             if lower_wick_ratio >= 0.30 {
//                 return MarketType::FailedBearish;
//             }
//             return MarketType::Bearish;
//         }
//     } else {
//         // --- PATH B: SMALL BODY (Reversals or Equilibrium) ---
        
//         // Priority 1: High Conviction Reversal (Wick > 60%)
//         if lower_wick_ratio >= 0.60 { return MarketType::BullishReversal; }
//         if upper_wick_ratio >= 0.60 { return MarketType::BearishReversal; }

//         // Priority 2: Equilibrium (Small body, balanced wicks)
//         let wick_diff = (upper_wick_ratio - lower_wick_ratio).abs();
//         if wick_diff < 0.15 {
//             return MarketType::PureIndecision;
//         }

//         // Priority 3: Soft Reversal (Body is small, but one side is clearly winning)
//         if lower_wick_ratio > upper_wick_ratio {
//             return MarketType::BullishReversal;
//         } else {
//             return MarketType::BearishReversal;
//         }
//     }
// }


// pub fn get_market_classification(
//     open: f64, high: f64, low: f64, close: f64
// ) -> MarketType {
//     let session_range = high - low;
//     if session_range <= EPSILON { return MarketType::Other; } 

//     let body_size = (close - open).abs();
//     let body_ratio = body_size / session_range;
//     let real_body_high = close.max(open);
//     let real_body_low = close.min(open);
    
//     let upper_wick_ratio = (high - real_body_high) / session_range;
//     let lower_wick_ratio = (real_body_low - low) / session_range;
//     let total_wick_ratio = upper_wick_ratio + lower_wick_ratio;

//     // --- 1. THE 25% GATE ---
//     if body_ratio >= 0.25 {
//         // --- PATH A: LARGE BODY (Trends or Traps) ---
        
//         // NEW: Dominance Check
//         // If wicks are 1.5x larger than the body, it's too "noisy" to be a trend.
//         if total_wick_ratio > (body_ratio * 1.5) {
//             return MarketType::PureIndecision;
//         }

//         if close > open {
//             if upper_wick_ratio >= 0.30 { return MarketType::FailedBullish; }
//             return MarketType::Bullish;
//         } else {
//             if lower_wick_ratio >= 0.30 { return MarketType::FailedBearish; }
//             return MarketType::Bearish;
//         }
//     } else {
//         // --- PATH B: SMALL BODY (Reversals or Equilibrium) ---
//         if lower_wick_ratio >= 0.60 { return MarketType::BullishReversal; }
//         if upper_wick_ratio >= 0.60 { return MarketType::BearishReversal; }

//         let wick_diff = (upper_wick_ratio - lower_wick_ratio).abs();
//         if wick_diff < 0.15 { return MarketType::PureIndecision; }

//         if lower_wick_ratio > upper_wick_ratio { return MarketType::BullishReversal; }
//         else { return MarketType::BearishReversal; }
//     }
// }

// pub fn get_market_classification(
//     open: f64, 
//     high: f64, 
//     low: f64, 
//     close: f64
// ) -> MarketType {
//     let session_range = high - low;
    
//     // 1. Initial State / Edge Case Check
//     if session_range <= EPSILON { 
//         return MarketType::Other; 
//     } 

//     let body_size = (close - open).abs();
//     let body_ratio = body_size / session_range;
//     let real_body_high = close.max(open);
//     let real_body_low = close.min(open);
    
//     let upper_wick_ratio = (high - real_body_high) / session_range;
//     let lower_wick_ratio = (real_body_low - low) / session_range;
//     let total_wick_ratio = upper_wick_ratio + lower_wick_ratio;

//     // --- 2. THE 25% GATE: IS THERE ENOUGH INTENT? ---
//     if body_ratio >= 0.25 {
//         // --- PATH A: LARGE BODY (Potential Trends or Traps) ---

//         // DYNAMIC MULTIPLIER: Judge how much "noise" (wicks) we allow 
//         // before a large body is considered "Indecision".
//         let dynamic_multiplier = if body_ratio < 0.35 {
//             1.5 // Tight: Small-ish bodies are easily disrupted by wicks
//         } else if body_ratio < 0.50 {
//             2.0 // Moderate: Medium bodies need significant wicks to be Indecision
//         } else {
//             2.5 // Loose: Large bodies are very hard to override
//         };

//         // Check for Indecision/High-Wave noise
//         if total_wick_ratio > (body_ratio * dynamic_multiplier) {
//             return MarketType::PureIndecision;
//         }

//         // Check for Directional Failure vs Healthy Trend
//         if close > open {
//             // It's a Green Candle. Did sellers push back > 30% from the high?
//             if upper_wick_ratio >= 0.30 {
//                 return MarketType::FailedBullish;
//             }
//             return MarketType::Bullish;
//         } else {
//             // It's a Red Candle. Did buyers push back > 30% from the low?
//             if lower_wick_ratio >= 0.30 {
//                 return MarketType::FailedBearish;
//             }
//             return MarketType::Bearish;
//         }
//     } else {
//         // --- PATH B: SMALL BODY (Reversals or Equilibrium) ---
        
//         // Priority 1: High Conviction Reversal (Wick owns 60%+ of the candle)
//         if lower_wick_ratio >= 0.60 { return MarketType::BullishReversal; }
//         if upper_wick_ratio >= 0.60 { return MarketType::BearishReversal; }

//         // Priority 2: Equilibrium (Small body, balanced wicks)
//         let wick_diff = (upper_wick_ratio - lower_wick_ratio).abs();
//         if wick_diff < 0.15 {
//             return MarketType::PureIndecision;
//         }

//         // Priority 3: Soft Reversal (Body is small, but one side is clearly winning the wick war)
//         if lower_wick_ratio > upper_wick_ratio {
//             return MarketType::BullishReversal;
//         } else {
//             return MarketType::BearishReversal;
//         }
//     }
// }