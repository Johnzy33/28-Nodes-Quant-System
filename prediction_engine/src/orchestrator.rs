
use shared_models::{
    AnchoredGrid, Candle, GridLayer, PriceLevel, SignalQuality, TradeSignal, TradingBias
};
use std::collections::HashMap;

use super::tracker::CampaignTracker;
use crate::tracker::*;

use anyhow::Result; // Using anyhow for simplified error handling



/// Input container for all historical data needed by the prediction engine.
pub struct PredictionEngineInput {
    // Strategic Layers (L1, L2)
    pub prev_year_candles: Vec<Candle>,      // For Yearly (L1)
    pub critical_h8_candles: Vec<Candle>,    // For Critical (L2)
    
    // Campaign Layers (L3, L4, L5)
    pub prev_week_candles: Vec<Candle>,      // For Weekly (L3)
    pub multi_day_daily_candles: Vec<Candle>,// For Multi-Day (L4)
    pub prev_day_session_candles: Vec<Candle>, // For Previous Day (L5)

    // Execution Data
    pub current_price: f64,
    pub instrument_id: String,
    pub current_session_candles: Vec<Candle>, // For Inter-Session filter
}





/// Executes the full SPG prediction cycle: Anchoring all 5 layers, 
/// then generating the final weighted signal.
pub fn run_prediction_cycle(input: PredictionEngineInput) -> Result<TradeSignal> {
    
    // 1. Initialize the Campaign Tracker
    let mut tracker = CampaignTracker::new();

    // --- 2. Anchor Strategic Layers (L1, L2) ---
    // Note: We use the question mark operator (?) to propagate errors if anchoring fails
    
    tracker.anchor_yearly_grid(&input.prev_year_candles)
        .map_err(|e| anyhow::anyhow!("L1 Anchor Failure: {}", e))?;

    tracker.anchor_critical_grid(&input.critical_h8_candles)
        .map_err(|e| anyhow::anyhow!("L2 Anchor Failure: {}", e))?;

    // --- 3. Anchor Campaign Layers (L3, L4, L5) ---
    
    tracker.anchor_weekly_grid(&input.prev_week_candles)
        .map_err(|e| anyhow::anyhow!("L3 Anchor Failure: {}", e))?;
        
    tracker.anchor_multi_day_grid(&input.multi_day_daily_candles)
        .map_err(|e| anyhow::anyhow!("L4 Anchor Failure: {}", e))?;

    tracker.anchor_previous_day_grid(&input.prev_day_session_candles)
        .map_err(|e| anyhow::anyhow!("L5 Anchor Failure: {}", e))?;

    // --- 4. Generate Final Signal (Execution) ---

   let final_signal = tracker.generate_trade_signal(
        input.current_price,
        &input.instrument_id,
        &input.current_session_candles,
    )
    // FIX: Use .map_err() to convert the String error into an anyhow::Error
    .map_err(|e| anyhow::anyhow!("Signal Generation Failure: {}", e))?;

    // Log all active grids for debugging purposes (optional but highly recommended)
    // println!("Active Grids: {:?}", tracker.active_grids);

    Ok(final_signal)
}