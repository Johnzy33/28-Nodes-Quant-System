
// shared_models/src/tcs_analysis_config.rs (Continued)

/// The set of constants defining the thresholds for interpreting raw TCS scores.
pub struct TcsThresholds;

impl TcsThresholds {
    // --- CONFIDENCE CATEGORIES (TCS > 100) ---
    pub const EXTREME_CONFIDENCE: f64 = 200.0; 
    pub const HIGH_CONFIDENCE: f64 = 150.0;
    pub const CONFIRMATORY: f64 = 120.0;
    
    // --- NEUTRAL/CONTRADICTORY CATEGORIES (TCS < 100) ---
    pub const NEUTRAL: f64 = 85.0; 
    pub const CONTRADICTORY: f64 = 75.0; 

    // --- COMPARISON THRESHOLDS ---
    pub const MIN_DELTA_FOR_SIGNIFICANCE: f64 = 10.0; 

    /// Converts a raw TCS score into a descriptive, narrative label.
    pub fn get_tcs_narrative_label(score: f64) -> &'static str {
        if score >= Self::EXTREME_CONFIDENCE {
            "🔴 Extreme Confidence Signal"
        } else if score >= Self::HIGH_CONFIDENCE {
            "🚀 High Confidence Signal"
        } else if score >= Self::CONFIRMATORY {
            "✅ Confirmatory Edge"
        } else if score >= 100.0 {
            "📈 Slightly Confirmatory"
        } else if score >= Self::NEUTRAL {
            "⚪ Neutral/Consolidation"
        } else if score >= Self::CONTRADICTORY {
            "📉 Weakly Contradictory"
        } else {
            "🛑 Strong Failure Risk (Veto)"
        }
    }
}