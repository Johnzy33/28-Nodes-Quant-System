// shared_models/src/models.rs

use chrono::{NaiveDate, DateTime, Utc};
use sqlx::FromRow;
use serde::{Deserialize, Serialize};

// Matches the 'assets' table schema
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, PartialEq)]
pub struct AssetInfo {
    pub id: String,
    pub symbol: String,
    pub timezone: String,
    pub source: String,
    pub name: Option<String>,
    pub asset_class: Option<String>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub active: Option<bool>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct AssetSeed {
    pub symbol: String,
    pub timezone: String,
    pub name: Option<String>,
    pub asset_class: Option<String>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub source: String,
    // Note: 'active' is omitted as it's not in the JSON and will be hardcoded to true/Some(true) during insertion
}

// Matches the output of get_trading_signal and get_signal_for_backtest_pattern
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct FciSignalOutput {
    pub asset_id: Option<String>,
    pub trading_date: Option<NaiveDate>,
    pub ps2_name: Option<String>,
    pub ps1_name: Option<String>,
    pub ps2_bias: Option<String>,
    pub ps1_bias: Option<String>,
    pub cs_name: String,
    pub predicted_day_type: String,
    pub signal_direction: String,
    pub fci_score: f64,
    pub signal_confidence: String,
    pub is_vetoed: bool,
    pub pcs_anchor_score: f64,
    pub tcs_multiplier: f64,
    pub p_continuation_raw: Option<f64>,
}
#[derive(Debug, Clone)]
pub struct FciMetricsForSession {
    pub long_conditional_prob: f64, 
    pub short_conditional_prob: f64, 
    pub tcs_score: f64,
    pub continuation_prob: f64,
}

// Matches the output of the daily_composite_score table/query
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DcsScore {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub dcs: Option<f64>,
    pub dcs_classification: Option<String>,
    pub f_reversal: Option<f64>,
    pub f_dm1_factor: Option<f64>,
    pub f_commitment: Option<f64>,
    pub f_sustainability: Option<f64>,
}


// --- 1. Intermediate Context Structs (replacing CTEs) ---

// A temporary struct to hold raw counts before weighting
#[derive(Debug)]
pub struct RawCount {
    pub event_count: f64,
    pub total_attempts: f64,
}


#[derive(Debug)]
pub struct LookbackInterval {
    pub name: &'static str,
    pub weight: f64,
    pub start_date: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct DayContext {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub c_day_bias: String,
    pub day0_direction: String,
    pub pd1_bias: Option<String>,
    pub pd1_direction: Option<String>,
    pub pd1_dow: Option<String>,
    pub pd2_bias: Option<String>,
    pub pd2_direction: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DailyContextData {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub day_type: Option<String>,
    //pub consolidation_subtype: Option<String>,
    pub high_session: String,
    pub low_session: String,
    pub high_bar: i32, 
    pub low_bar: i32,

}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct SessionToBarMetrics {
    pub asset_id: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,
    pub target_bar_num: i32,    // The "Easy check" field
    pub target_bar_bias: String, // The actual outcome key
    pub total_attempts: f64,
    pub success_count: f64,
    pub probability: f64,
}

#[derive(Debug, Clone)]
pub struct DailyOutcome {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub day_bias: String,
    pub week_bias: String,   
    pub day_bias_group: String,
}

#[derive(Debug, Clone)]
pub struct DailyOutcomeValue {
    pub day_bias: String, // e.g., "Bullish"
    pub day_confidence: f64, // e.g., 85.5
}
#[derive(Debug, Clone)]
pub struct WeeklyContext {
    pub week_start: NaiveDate,
    pub asset_id: String,
    pub week_bias: String,
    pub week_of_month: i32, // INTEGER
    pub pw1_bias: Option<String>,
    pub pw2_bias: Option<String>,
}

// --- 2. Input Data Structs (Data fetched from DB) ---

#[derive(Debug, Clone, FromRow)]
pub struct SessionContextData {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    // Names (e.g., NYAM) and Biases (e.g., Bullish_Reversal)
    pub ps2_name: Option<String>,
    pub ps2_bias: Option<String>,
    pub ps1_name: Option<String>,
    pub ps1_bias: Option<String>,
    pub cs_name: Option<String>,
    pub cs_bias: Option<String>,
    pub session_end_ts: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct EightContextData {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    // Names (e.g., NYAM) and Biases (e.g., Bullish_Reversal)
    pub pb2_num: Option<i32>,
    pub pb2_bias: Option<String>,
    pub pb1_num: Option<i32>,
    pub pb1_bias: Option<String>,
    pub cb_num: Option<i32>,
    pub cb_bias: Option<String>,
    pub session_end_ts: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DailyViewExtremes {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub high_session: String,
    pub low_session: String,
}

// --- 3. Output Metrics Structs (Final Tables) ---

// Metrics A.1, A.2, A.3 (Base Rates)
#[derive(Debug, Clone)]
pub struct DayTypeBaseRate {
    pub asset_id: String,
    pub daily_bias: String,
    pub p_day_type_base: f64,
}
#[derive(Debug, Clone, FromRow)]
pub struct CsBaseRate {
    pub asset_id: String,
    pub cs_name: String,
    pub cs_bias: String,
    pub p_base: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CbBaseRate {
    pub asset_id: String,
    pub cb_num: Option<i32>, // Important for STCS mapping
    pub cb_bias: String,
    pub p_base: f64,
}
#[derive(Debug, Clone)]
pub struct WeeklyBaseRate {
    pub asset_id: String,
    pub week_bias: String,
    pub weekly_base_rate: f64,
}

// Metric B (3rd Order Conditional - DCI Core)
#[derive(Debug, Clone)]
pub struct DayOutcome3rdOrder {
    pub asset_id: String,
    pub pd2_bias: String,
    pub pd1_bias: String,
    pub pd1_dow: String,
    pub c_day_bias: String,
    pub reliable_total_attempts: f64,
    pub reliable_success_count: f64,
    pub p_day_bias_conditional: f64,
}

#[derive(Debug, Clone)]
pub struct DailyTrendContinuationRate {
    pub asset_id: String,
    pub pd2_bias: String,
    pub pd1_bias: String,
    pub pd1_dow: String,
    pub c_day_bias: String, // <-- NEW FIELD for the unique key
    pub reliable_total_attempts: f64,
    pub reliable_success_count: f64,
    pub p_continuation_conditional: f64,
}
// Metric D (Trend Rate - DCI Momentum)
#[derive(Debug, Clone)]
pub struct DailyTrendRate {
    pub asset_id: String,
    pub pd2_direction: String,
    pub pd1_direction: String,
    pub pd1_dow: String,
    pub day0_direction: String,
    pub p_trend_rate: f64,
}

// WCI Metrics (C.1, C.2, C.3)
#[derive(Debug, Clone)]
pub struct WocsDailyScore {
    pub asset_id: String,
    pub pd2_outcome: String,
    pub pd1_outcome: String,
    pub week0_outcome: String,
    pub wocs_d_score: f64,
}
#[derive(Debug, Clone)]
pub struct WeeklyContinuationScore {
    pub asset_id: String,
    pub pw2_outcome: String,
    pub pw1_outcome: String,
    pub week0_outcome: String,
    pub wcs_score: f64,
}
#[derive(Debug, Clone)]
pub struct WeeklyMonthlySeasonality {
    pub asset_id: String,
    pub week0_week_of_month: i32,
    pub week0_outcome: String,
    pub wms_score: f64,
}

// FCI Metrics (D.1, D.2, G)
#[derive(Debug, Clone, FromRow)]
pub struct Transition2ndOrder {
    // pub trading_date: NaiveDate,
    pub asset_id: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,
    pub cs_name: String,
    pub cs_bias: String,
    pub reliable_total_attempts: f64,
    pub reliable_success_count: f64,
    pub p_transition_conditional: f64,
}


#[derive(Debug, Clone, FromRow)]
pub struct BarTransition2ndOrder {
    pub asset_id: String,
    pub pb2_bias: String,
    pub pb2_num: i32,
    pub pb1_bias: String,
    pub pb1_num: i32,
    pub cb_bias: String, 
    pub cb_num: i32,
    pub reliable_total_attempts: f64,
    pub reliable_success_count: f64,
    pub p_transition_conditional: f64,
}

#[derive(Debug, Clone)]
pub struct TrendRateContext {
    pub inner: DayContext,
}
#[derive(Debug, Clone)]
pub struct HighLowSession2ndOrder {
    pub asset_id: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,
    pub session_extreme: String,
    pub extreme_session_name: String,
    pub reliable_total_attempts: f64,
    pub reliable_success_count: f64,
    pub p_extreme_session_conditional: f64,
}
#[derive(Debug, Clone)]
pub struct TcsContinuationScore {
   // pub trading_date: NaiveDate,
    pub asset_id: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,
    pub p_continuation: f64,
}

#[derive(Debug, Clone)]
pub struct DcsInputData {
    pub conditionals: Vec<DayOutcome3rdOrder>,
    pub base_rates: Vec<DayTypeBaseRate>,
}

#[derive(Debug, Clone)]
pub struct StcsInputData {
    pub conditionals: Vec<SessionToBarMetrics>, // The metrics we just built
    pub base_rates: Vec<CbBaseRate>,           // Historical base rates for Bars 1, 2, 3
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionToBarStcs {
    pub asset_id: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,
    pub target_bar_num: i32,
    pub target_bar_bias: String,
    pub probability: f64,        // P(Conditional)
    pub p_bar_base: f64,         // P(Base)
    pub stcs_score: f64,         // The Lift
}

// Final Scores (E, F)
#[derive(Debug, Clone)]
pub struct Dcs3rdOrder { // PDCS
    pub asset_id: String,
    pub pd2_bias: String,
    pub pd1_bias: String,
    pub pd1_dow: String,
    pub c_day_bias: String,
    pub p_day_bias_conditional: f64,
    pub p_day_type_base: f64,
    pub dcs_score: f64,
    //pub insight_label: String,
}
#[derive(Debug, Clone)]
pub struct Tcs2ndOrder { // TCS
   // pub trading_date: NaiveDate,
    pub asset_id: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,
    pub cs_name: String,
    pub cs_bias: String,
    pub p_transition_conditional: f64,
    pub p_cs_base: f64,
    pub tcs_score: f64,
}



#[derive(Debug, Clone)]
pub struct Btcs2ndOrder { // TCS
   // pub trading_date: NaiveDate,
    pub asset_id: String,
    pub pb2_bias: String,
    pub pb2_num: i32,
    pub pb1_bias: String,
    pub pb1_num: i32,
    pub cb_num: i32,
    pub cb_bias: String,
    pub p_transition_conditional: f64,
    pub p_cb_base: f64,
    pub btcs_score: f64,
}


#[derive(Debug, Clone, FromRow)]
pub struct TcsData {
    pub transitions: Vec<Transition2ndOrder>,
    pub base_rates: Vec<CsBaseRate>,
}

#[derive(Debug, Clone, FromRow)]
pub struct BtcsInputData {
    pub transitions: Vec<BarTransition2ndOrder>,
    pub base_rates: Vec<CbBaseRate>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PcsInputData {
    pub conditionals: Vec<DayType2ndOrder>,
    pub base_rates: Vec<DayTypeBaseRate>,
}

#[derive(Debug, Clone, FromRow)]
pub struct BpcsInputData {
    pub conditionals: Vec<BarDayType2ndOrder>,
    pub base_rates: Vec<DayTypeBaseRate>,
}


#[derive(Debug, Clone)]
pub struct DayType2ndOrder {
    pub asset_id: String,
    pub ps2_name: String,        // Added Name
    pub ps2_bias: String,
    pub ps1_name: String,        // Added Name
    pub ps1_bias: String,
    pub day_type: String, 
    pub reliable_total_attempts:f64,
    pub reliable_success_count:f64,  // One of the 7 Daily Outcomes
    pub p_day_type_conditional: f64,
}

#[derive(Debug, Clone)]
pub struct BarDayType2ndOrder {
    pub asset_id: String,
    pub pb2_num: i32,        // Added Name
    pub pb2_bias: String,
    pub pb1_num: i32,        // Added Name
    pub pb1_bias: String,
    pub day_type: String, 
    pub reliable_total_attempts:f64,
    pub reliable_success_count:f64,  // One of the 7 Daily Outcomes
    pub p_day_type_conditional: f64,
}

// Final output of the PCS calculation (the highest confidence signal for a pattern)
#[derive(Debug, Clone)]
pub struct PcsScore2ndOrder {
    pub asset_id: String,
    pub ps2_name: String,        // Must be part of the final feature key
    pub ps2_bias: String,
    pub ps1_name: String,        // Must be part of the final feature key
    pub ps1_bias: String,
    pub daily_bias: String, // The Day Type with the max score
    pub p_day_type_conditional: f64,
    pub p_day_type_base: f64,
    pub pcs_score: f64,       // The max PCS ratio score
}

#[derive(Debug, Clone)]
pub struct BpcsScore2ndOrder {
    pub asset_id: String,
    pub pb2_num: i32,        // Must be part of the final feature key
    pub pb2_bias: String,
    pub pb1_num: i32,        // Must be part of the final feature key
    pub pb1_bias: String,
    pub daily_bias: String, // The Day Type with the max score
    pub p_day_type_conditional: f64,
    pub p_day_type_base: f64,
    pub pcs_score: f64,       // The max PCS ratio score
}


#[derive(Debug, Clone)]
pub struct SessionFeatureVector {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    
    // --- CONTEXT (The exact pattern that just completed) ---
    pub ps2_name: String,        // e.g., "LN"
    pub ps1_name: String,        // e.g., "NYAM"
    pub ps2_bias: String,        // The bias of the PS2 session
    pub ps1_bias: String,        // The bias of the PS1 session
    
    // --- FCI METRICS (Calculated for the PS2/PS1 pattern above) ---
    // The conditional probability for a Long-aligned CS outcome given this specific pattern
    pub p_cs_long_cond: f64,     
    // The conditional probability for a Short-aligned CS outcome given this specific pattern
    pub p_cs_short_cond: f64,    
    pub tcs_score: f64,          // The calculated TCS score for the highest probability outcome
    pub p_tcs_continuation: f64, // P(Continuation) for this specific PS2/PS1 pattern %
    
    // --- DAILY ANCHOR (The ML Prediction from the previous day) ---
    // This is the output of the 'MultiTimeframeFeatureVector' ML Model.
    pub ml_daily_prediction: String, // e.g., "Bullish" or "Bearish"
    pub ml_daily_score: f64,         // Confidence score of the daily prediction (e.g., 85.5)

    // --- TARGET (The actual outcome for the CS session) ---
    pub target_cs_bias: String,  // The known bias outcome of the next session (CS)
}

#[derive(Debug, Clone)]
pub struct SessionFeatureVectorML {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    
    // --- CONTEXT (Unchanged) ---
    pub ps2_name: String,        
    pub ps1_name: String,        
    pub ps2_bias: String,        
    pub ps1_bias: String,        
    
    // --- FCI METRICS (Unchanged) ---
    pub p_cs_long_cond: f64,     
    pub p_cs_short_cond: f64,    
    pub tcs_score: f64,          
    pub p_tcs_continuation: f64, 
    
    // --- NEW: PCS DAY ANCHOR ---
    pub pcs_day_type: String,     // The Day Type with the max PCS score (e.g., "BearishReversal")
    pub pcs_score: f64,           // The maximum PCS ratio score
    
    // --- NEW: FLATTENED HIGH/LOW PROBABILITIES (10 FEATURES) ---
    pub p_high_as: f64,
    pub p_high_ln: f64,
    // ... 8 more p_high/p_low fields ...
    
    // --- DAILY ANCHOR (Unchanged) ---
    pub ml_daily_prediction: String, 
    pub ml_daily_score: f64,         

    // --- TARGET (Unchanged) ---
    pub target_cs_bias: String,  
}

#[derive(Debug, Clone)]
pub struct MultiTimeframeFeatureVector {
    pub trading_date: NaiveDate,
    pub asset_id: String,

    // --- DAILY CONFIDENCE INDEX (DCI) SIGNALS ---
    pub dci_long_max_score: f64,        // Max DCI score for Long outcomes
    pub dci_short_max_score: f64,       // Max DCI score for Short outcomes
    pub pdcs_anchor_max_score: f64,     // Max raw PDCS score (P_cond / P_base)
    pub p_dci_conditional_max: f64,     // Max P(D0 | PD2, PD1, DOW) %
    pub p_day_type_base_for_max: f64,   // P(D0) % for the outcome above
    pub p_daily_trend_rate_raw: f64,    // Max P(Trend | PD2_Dir, PD1_Dir) %

    // --- FLOW CONFIDENCE INDEX (FCI) SIGNALS ---
    pub fci_long_max_score: f64,        // Max FCI score for Long-aligned CS Biases
    pub fci_short_max_score: f64,       // Max FCI score for Short-aligned CS Biases
    pub tcs_anchor_max_score: f64,      // Max raw TCS score (P_cond / P_base)
    
    // **FIXED: DUAL RAW CS CONDITIONAL PROBABILITIES**
    pub p_cs_long_cond_max: f64,        // Max P(CS in Long Group | PS2, PS1) %
    pub p_cs_short_cond_max: f64,       // Max P(CS in Short Group | PS2, PS1) %
    
    pub p_tcs_continuation_raw: f64,    // P(Continuation | PS2, PS1) %
    pub p_high_session_cond_max: f64,   // Max P(High Session Name | ...) %
    pub p_low_session_cond_max: f64,    // Max P(Low Session Name | ...) %

    // --- WEEKLY CONFIDENCE INDEX (WCI) SIGNALS ---
    pub wci_long_max_score: f64,
    pub wci_short_max_score: f64,
    pub wocs_d_anchor_max: f64,         // Max raw WOCS-D score
    pub wcs_continuation_max: f64,      // Max raw WCS score
    pub wms_seasonality_max: f64,       // Max raw WMS score
    
    // --- TARGETS (The known outcome for this date) ---
    pub target_day_outcome: String,
    pub target_week_outcome: String,
}

