
use crate::spg_tracker::SpgTracker;
use crate::anchored_grid::AnchoredGrid;
use crate::price_grid::GridLayer;
use crate::price_grid::PriceLevel;
use crate::signal_type::{TradingBias, SignalQuality, TradeSignal};
use shared_models::candle::Candle;


// --- Helper Functions ---

/// Determines the bias for a single layer based on price relative to P0.5 (Mid-Point).
fn get_layer_bias(grid: &AnchoredGrid, current_price: f64) -> TradingBias {
    if let Some(mid_point) = grid.spg.get_level_price(PriceLevel::MidPoint) {
        if current_price > mid_point {
            TradingBias::Sell 
        } else if current_price < mid_point {
            TradingBias::Buy 
        } else {
            TradingBias::Neutral 
        }
    } else {
        TradingBias::Neutral
    }
}

/// Implements the Weighted Hierarchical Veto System (Quality Scoring),  The immediate higher layer (MultiDay, L4) carries the most weight (+2).
fn calculate_signal_quality(
    tracker: &SpgTracker, 
    base_bias: TradingBias,
    current_price: f64
) -> SignalQuality {
    if base_bias == TradingBias::Neutral { return SignalQuality::Medium; } 

    let mut alignment_score = 0;
    
    // Check against the Multi-Day (L4) - IMMEDIATE HIGHER LAYER (Weight: 2)
    if let Some(multi_day_grid) = tracker.get_grid(GridLayer::MultiDay) {
        if get_layer_bias(multi_day_grid, current_price) == base_bias {
            alignment_score += 2; 
        }
    }

    // Check against the Weekly (L3) (Weight: 1)
    if let Some(weekly_grid) = tracker.get_grid(GridLayer::PW) {
        if get_layer_bias(weekly_grid, current_price) == base_bias {
            alignment_score += 1;
        }
    }

    // Check against the Yearly (L1) - STRATEGIC VETO (Weight: 1)
    if let Some(yearly_grid) = tracker.get_grid(GridLayer::Yearly) {
        if get_layer_bias(yearly_grid, current_price) == base_bias {
            alignment_score += 1;
        }
    }

    // Determine quality based on alignment score
    match alignment_score {
        x if x >= 3 => SignalQuality::High,   
        x if x >= 1 => SignalQuality::Medium, 
        _           => SignalQuality::Low,    
    }
}


// --- Main Execution Logic ---

impl SpgTracker {
    
    pub fn generate_trade_signal(
        &self, 
        current_price: f64,
        instrument_id: &str,
        session_candles: &[Candle] 
    ) -> Result<TradeSignal, String> {
        
        let daily_grid = self.get_grid(GridLayer::PreviousDay)
            .ok_or_else(|| "Previous Day Grid (L5) not anchored. Cannot generate signal.".to_string())?;

        // Determine BASE Bias (L5 Bias)
        let base_bias = get_layer_bias(daily_grid, current_price);
        
        if base_bias == TradingBias::Neutral {
             return Ok(TradeSignal {
                instrument: instrument_id.to_string(),
                current_price,
                bias: TradingBias::Neutral,
                quality: SignalQuality::Low,
                entry_level: current_price,
                target_price: current_price,
                stop_loss: current_price,
                reason: "NEUTRAL: Price is at P0.5 (Equilibrium Line).".to_string(),
            });
        }
        
        //  Apply Veto System to get Quality
        let quality = calculate_signal_quality(self, base_bias, current_price);

        //  Apply Inter-Session Liquidity Filter (Confirmation/Target Setting)
        let (confirmed, entry_level, target_price) = 
            self.apply_inter_session_rules(daily_grid, base_bias, current_price, session_candles)?;

        if !confirmed {
            return Ok(TradeSignal {
                instrument: instrument_id.to_string(),
                current_price,
                bias: base_bias,
                quality: SignalQuality::Low, // Downgrade quality due to lack of confirmation
                entry_level: current_price,
                target_price: current_price,
                stop_loss: current_price,
                reason: format!("Unconfirmed {:?} Signal: L5 action failed liquidity check.", base_bias),
            });
        }
        
        // Construct Final Signal
        Ok(TradeSignal {
            instrument: instrument_id.to_string(),
            current_price,
            bias: base_bias,
            quality,
            entry_level,
            target_price,
            stop_loss: daily_grid.spg.get_level_price(PriceLevel::MidPoint).unwrap_or(current_price),
            reason: format!("{:?} Signal Confirmed. Quality: {:?}. Failure at L5 Exit.", base_bias, quality),
        })
    }


    
    // This logic determines the high-confidence entry and target.
    fn apply_inter_session_rules(
        &self,
        daily_grid: &AnchoredGrid,
        base_bias: TradingBias,
        current_price: f64,
        session_candles: &[Candle]
    ) -> Result<(bool, f64, f64), String> {
        
        let current_session_high = session_candles.iter().map(|c| c.high).fold(f64::MIN, f64::max);
        let current_session_low = session_candles.iter().map(|c| c.low).fold(f64::MAX, f64::min);
        
        // --- Key Levels from Previous Day (L5) ---
        let p0_0 = daily_grid.spg.get_level_price(PriceLevel::Zero).unwrap();
        let p1_0 = daily_grid.spg.get_level_price(PriceLevel::One).unwrap();
        let p1_5 = daily_grid.spg.get_level_price(PriceLevel::ExitUpper).unwrap();
        let p_0_5 = daily_grid.spg.get_level_price(PriceLevel::ExitLower).unwrap();
        let p0_6 = daily_grid.spg.get_level_price(PriceLevel::DiscountUpper).unwrap();
        let p0_4 = daily_grid.spg.get_level_price(PriceLevel::DiscountLower).unwrap();
        
        // --- SELL SCENARIO CONFIRMATION ---
        // Conditions: 1. Price failed near P1.5, AND 2. Price cleared P0.0.
        let confirmed_sell = (current_session_high < p1_5) && (current_price < p0_0);

        if confirmed_sell && base_bias == TradingBias::Sell {
            let entry = p0_6; 
            let target = p_0_5;
            return Ok((true, entry, target));
        }

        // --- BUY SCENARIO CONFIRMATION ---
        // Conditions: 1. Price failed near P-0.5, AND 2. Price cleared P1.0.
        let confirmed_buy = (current_session_low > p_0_5) && (current_price > p1_0);

        if confirmed_buy && base_bias == TradingBias::Buy {
            let entry = p0_4;
            let target = p1_5;
            return Ok((true, entry, target));
        }

        Ok((false, current_price, current_price))
    }
}