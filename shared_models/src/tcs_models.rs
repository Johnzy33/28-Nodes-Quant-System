use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use std::collections::HashMap;


// --- Helper struct to represent a single key prediction point ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionPoint {
    pub bias: String,
    pub score: f64,
    pub label: String,
    pub conditional_prob: f64,
    pub base_prob: f64, // Added P_base for better context
}

// --- TCS 1ST ORDER DATA (Includes Session Names) ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tcs1stOrderData {
    // Context Info
    pub current_ps1_session: String,  // e.g., "NYAM"
    pub current_ps1_bias: String,     // e.g., "Bearish"
    pub current_cs_session: String,   // e.g., "NYL"
    
    // Prediction Details
    pub predicted_cs_bias: String,
    pub lookback_period: String,
    pub p_transition_conditional: f64,
    pub p_cs_base: f64,
    pub tcs_score: f64,
}

impl Tcs1stOrderData {
    /// Maps a tokio_postgres::Row to Tcs1stOrderData.
    pub fn from_row(row: &Row) -> Self {
        Tcs1stOrderData {
            current_ps1_session: row.get("current_ps1_session"),
            current_ps1_bias: row.get("current_ps1_bias"),
            current_cs_session: row.get("current_cs_session"),
            predicted_cs_bias: row.get("predicted_cs_bias"),
            lookback_period: row.get("lookback_period"),
            p_transition_conditional: row.get("p_transition_conditional"),
            p_cs_base: row.get("p_cs_base"),
            tcs_score: row.get("tcs_score"),
        }
    }
}


// --- TCS 2ND ORDER DATA (Includes Session Names) ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tcs2ndOrderData {
    // Context Info
    pub current_ps2_session: String,  // e.g., "LN"
    pub current_ps2_bias: String,     // e.g., "Bullish"
    pub current_ps1_session: String,  // e.g., "NYAM"
    pub current_ps1_bias: String,     // e.g., "Bearish"
    pub current_cs_session: String,   // e.g., "NYL"
    
    // Prediction Details
    pub predicted_cs_bias: String,
    pub lookback_period: String,
    pub p_transition_conditional: f64,
    pub p_cs_base: f64,
    pub tcs_score: f64,
}

impl Tcs2ndOrderData {
    /// Maps a tokio_postgres::Row to Tcs2ndOrderData.
    pub fn from_row(row: &Row) -> Self {
        Tcs2ndOrderData {
            current_ps2_session: row.get("current_ps2_session"),
            current_ps2_bias: row.get("current_ps2_bias"),
            current_ps1_session: row.get("current_ps1_session"),
            current_ps1_bias: row.get("current_ps1_bias"),
            current_cs_session: row.get("current_cs_session"),
            predicted_cs_bias: row.get("predicted_cs_bias"),
            lookback_period: row.get("lookback_period"),
            p_transition_conditional: row.get("p_transition_conditional"),
            p_cs_base: row.get("p_cs_base"),
            tcs_score: row.get("tcs_score"),
        }
    }
}

// Struct to hold the summary report for one TCS metric (e.g., 1st Order, 1Y lookback)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcsLookbackSummary {
    pub lookback_period: String,
    pub strongest_edge: PredictionPoint,
    pub most_likely: PredictionPoint,
    pub strongest_veto: PredictionPoint,
}

// Struct to hold the comparison result between TCS1 and TCS2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossOrderComparison {
    pub lookback_period: String,
    pub predicted_cs_bias: String,
    
    pub tcs_1st_order: f64,
    pub tcs_2nd_order: f64,
    pub delta: f64, // TCS_2nd - TCS_1st
    
    // Dynamic Session Info for Narrative
    pub ps1_name: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    
    // Narrative Interpretation
    pub narrative_label: String,
    pub explanation: String,
}

// The comprehensive final analysis report container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcsFinalReport {
    pub symbol: String,
    pub analysis_date: String,
    
    // Structured Data Summaries
    pub tcs1_summaries: HashMap<String, TcsLookbackSummary>, 
    pub tcs2_summaries: HashMap<String, TcsLookbackSummary>, 
    pub cross_order_comparisons: Vec<CrossOrderComparison>,
    
    // The final narrative output (Markdown)
    pub final_markdown_report: String,
}