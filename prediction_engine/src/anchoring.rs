

use shared_models::{AnchoredGrid, Candle, GridLayer, PriceLevel, SymmetricPriceGrid};
use super::tracker::CampaignTracker; 
use chrono::{TimeZone, Utc};


// Checks if a swing high or low constitutes an Initial Rejection (Rule 1).
fn is_initial_rejection(
    prev_spg: &SymmetricPriceGrid,
    current_high: f64,
    current_low: f64
) -> Option<GridLayer> { 
    let p1_25 = prev_spg.get_level_price(PriceLevel::MidPointUpper).unwrap();
    let p_0_25 = prev_spg.get_level_price(PriceLevel::MidPointLower).unwrap();

    // Check for High Rejection (R-Swing)
    if current_high > prev_spg.high && current_high < p1_25 {
        return Some(GridLayer::Critical); 
    }

    // Check for Low Rejection (L-Swing)
    if current_low < prev_spg.low && current_low > p_0_25 {
        return Some(GridLayer::MultiDay); 
    }

    None
}

// Checks if the price action since the range was anchored has achieved the minimum 40% retracement (completion signal).
fn has_multi_day_range_completed(
    prev_spg: &SymmetricPriceGrid,
    current_price_range: &[Candle]
) -> bool {
    let p0_4 = prev_spg.get_level_price(PriceLevel::DiscountLower).unwrap();
    let p0_6 = prev_spg.get_level_price(PriceLevel::DiscountUpper).unwrap();

    // Determine the direction of the previous swing (was P0 or P1 set last?)
    let last_swing_down = prev_spg.high > prev_spg.low; 

    if last_swing_down {
        current_price_range.iter().any(|c| c.high >= p0_4)
    } else {
        current_price_range.iter().any(|c| c.low <= p0_6)
    }
}


impl CampaignTracker {
    
    pub fn anchor_yearly_grid(
        &mut self,
        prev_year_candles: &[Candle]
    ) -> Result<(), String> {
        if prev_year_candles.is_empty() {
            return Err("Cannot anchor yearly grid: Input candle data is empty.".to_string());
        }

        // Determine High, Low, and Time Bounds
        let mut yearly_high = f64::MIN;
        let mut yearly_low = f64::MAX;
        
        let start_timestamp = prev_year_candles[0].timestamp;
        let end_timestamp = prev_year_candles.last().map(|c| c.timestamp).unwrap_or(start_timestamp);

        for candle in prev_year_candles {
            if candle.high > yearly_high {
                yearly_high = candle.high;
            }
            if candle.low < yearly_low {
                yearly_low = candle.low;
            }
        }

        //  Validate and Create the Grid
        if yearly_low >= yearly_high {
            return Err("Invalid yearly range: Low is not strictly less than High.".to_string());
        }

        let yearly_grid = match AnchoredGrid::new(
            GridLayer::Yearly, 
            yearly_high, 
            yearly_low, 
            start_timestamp, 
            end_timestamp
        ) {
            Ok(grid) => grid,
            Err(e) => return Err(format!("Failed to create Yearly SPG: {}", e)),
        };

        //  Store the Anchored Grid
        self.update_grid(yearly_grid);

        Ok(())
    }
    
    
    pub fn anchor_weekly_grid(
        &mut self,
        prev_week_candles: &[Candle]
    ) -> Result<(), String> {
        if prev_week_candles.is_empty() {
            return Err("Cannot anchor weekly grid: Input candle data is empty.".to_string());
        }

        //  Determine High, Low, and Time Bounds
        let mut weekly_high = f64::MIN;
        let mut weekly_low = f64::MAX;
        
        let start_timestamp = prev_week_candles[0].timestamp;
        let end_timestamp = prev_week_candles.last().map(|c| c.timestamp).unwrap_or(start_timestamp); 

        for candle in prev_week_candles {
            if candle.high > weekly_high {
                weekly_high = candle.high;
            }
            if candle.low < weekly_low {
                weekly_low = candle.low;
            }
        }

        // Validate and Create the Grid
        if weekly_low >= weekly_high {
            return Err("Invalid weekly range: Low is not strictly less than High.".to_string());
        }

        let weekly_grid = match AnchoredGrid::new(
            GridLayer::Weekly, 
            weekly_high, 
            weekly_low, 
            start_timestamp, 
            end_timestamp
        ) {
            Ok(grid) => grid,
            Err(e) => return Err(format!("Failed to create Weekly SPG: {}", e)),
        };

        // Store the Anchored Grid
        self.update_grid(weekly_grid);

        Ok(())
    }


    pub fn anchor_critical_grid(
        &mut self,
        h8_candles: &[Candle]
    ) -> Result<(), String> {
        if h8_candles.is_empty() {
            return Err("Cannot anchor critical grid: Input candle data is empty.".to_string());
        }

        let start_ts = h8_candles.first().unwrap().timestamp;
        let end_ts = h8_candles.last().unwrap().timestamp;
        
        let potential_high = h8_candles.iter().map(|c| c.high).fold(f64::MIN, f64::max);
        let potential_low = h8_candles.iter().map(|c| c.low).fold(f64::MAX, f64::min);
        
        let mut new_high = potential_high;
        let mut new_low = potential_low;
        let mut new_start_ts = start_ts;
        
        // ---  Check for Initial Grid Creation ---
        if self.get_grid(GridLayer::Critical).is_none() {
            let initial_grid = AnchoredGrid::new(
                GridLayer::Critical, potential_high, potential_low, start_ts, end_ts
            )?;
            self.update_grid(initial_grid);
            return Ok(());
        }

        // Check against Previous Grid (R_N-1) ---
        let r_n_minus_1 = self.get_grid(GridLayer::Critical).unwrap();
        let spg_prev = &r_n_minus_1.spg;
        
        // --- Rule 3: Range Satisfaction (The highest priority check) ---
        let exit_upper = spg_prev.get_level_price(PriceLevel::ExitUpper).unwrap_or(f64::MAX);
        let exit_lower = spg_prev.get_level_price(PriceLevel::ExitLower).unwrap_or(f64::MIN);
        
        if potential_high >= exit_upper || potential_low <= exit_lower {
            new_start_ts = end_ts;  
        } 
        
        // --- Rule 1 & 2: Initial Rejection and Mid-Point Validation ---
        else if let Some(rejection_type) = is_initial_rejection(spg_prev, potential_high, potential_low) {
            
            // Check Rule 2: Did the price pullback far enough to confirm the rejection?
            
            let is_mid_point_pullback_met = true; 
            
            if is_mid_point_pullback_met {
                match rejection_type {
                    GridLayer::Critical => { 
                        new_high = potential_high;
                        new_low = r_n_minus_1.spg.low;
                        new_start_ts = r_n_minus_1.start_timestamp; 
                    },
                    GridLayer::MultiDay => { 
                        new_high = r_n_minus_1.spg.high;
                        new_low = potential_low;
                        new_start_ts = r_n_minus_1.start_timestamp;
                    },
                    _ => unreachable!(),
                }
            } else {

                return Ok(());
            }
        } else {

            return Ok(());
        }

        //  Create and Store the New Anchored Grid
        let final_grid = AnchoredGrid::new(
            GridLayer::Critical, new_high, new_low, new_start_ts, end_ts
        )?;
        self.update_grid(final_grid);

        Ok(())
    }

    pub fn anchor_multi_day_grid(
        &mut self,
        daily_candles: &[Candle]
    ) -> Result<(), String> {
        if daily_candles.is_empty() {
            return Err("Cannot anchor multi-day grid: Input candle data is empty.".to_string());
        }

        let end_ts = daily_candles.last().unwrap().timestamp;
        let mut new_high = f64::MIN;
        let mut new_low = f64::MAX;
        let mut high_ts = 0;
        let mut low_ts = 0;
        
        let prev_grid = self.get_grid(GridLayer::MultiDay).cloned();

        //  Check for Range Completion of R_N-1 (if a previous grid exists)
        if let Some(r_n_minus_1) = prev_grid.clone() {
            // Price action since the last anchor was set
            let subsequent_candles: Vec<Candle> = daily_candles.iter()
                .filter(|c| c.timestamp >= r_n_minus_1.end_timestamp)
                .cloned()
                .collect();

            if subsequent_candles.is_empty() {
                return Ok(()); 
            }

            if has_multi_day_range_completed(&r_n_minus_1.spg, &subsequent_candles) {

            } else {
                // R_N-1 is NOT completed. The grid remains anchored to R_N-1.
                self.update_grid(r_n_minus_1);
                return Ok(());
            }
        }
        
        //  Search for the widest, valid Multi-Day Swing
        for (i, candle) in daily_candles.iter().rev().enumerate() {
            
            if candle.high >= new_high {
                new_high = candle.high;
                high_ts = candle.timestamp;
            }

            if candle.low <= new_low {
                new_low = candle.low;
                low_ts = candle.timestamp;
            }
        }

        //  Apply the "Different Days" Constraint
        let high_day = Utc.timestamp_millis(high_ts as i64).date_naive();
        let low_day = Utc.timestamp_millis(low_ts as i64).date_naive();

        if high_day == low_day {
            return Err("Multi-Day anchor failed: High and Low set on the same day.".to_string());
        }

        // Create and Store the New Anchored Grid
        let start_ts = if high_ts < low_ts { high_ts } else { low_ts };
        
        let final_grid = AnchoredGrid::new(
            GridLayer::MultiDay, new_high, new_low, start_ts, end_ts
        )?;
        self.update_grid(final_grid);

        Ok(())
    }

    pub fn anchor_previous_day_grid(
        &mut self,
        prev_day_candles: &[Candle]
    ) -> Result<(), String> {
        if prev_day_candles.is_empty() {
            return Err("Cannot anchor previous day grid: Input candle data is empty.".to_string());
        }

        //  Determine High, Low, and Time Bounds
        let mut daily_high = f64::MIN;
        let mut daily_low = f64::MAX;
        
        // Time bounds are set precisely by the first and last candle of the custom day
        let start_timestamp = prev_day_candles[0].timestamp;
        let end_timestamp = prev_day_candles.last().map(|c| c.timestamp).unwrap_or(start_timestamp);

        for candle in prev_day_candles {
            if candle.high > daily_high {
                daily_high = candle.high;
            }
            if candle.low < daily_low {
                daily_low = candle.low;
            }
        }

        // Validate and Create the Grid
        if daily_low >= daily_high {
            return Err("Invalid previous day range: Low is not strictly less than High.".to_string());
        }

        let daily_grid = match AnchoredGrid::new(
            GridLayer::PreviousDay, 
            daily_high, 
            daily_low, 
            start_timestamp, 
            end_timestamp
        ) {
            Ok(grid) => grid,
            Err(e) => return Err(format!("Failed to create Previous Day SPG: {}", e)),
        };

        //  Store the Anchored Grid
        self.update_grid(daily_grid);

        Ok(())
    }
   
}