
use crate::price_grid::PriceLevel;
use crate::symmetric_grid::SymmetricPriceGrid;
use crate::anchored_grid::AnchoredGrid;
use shared_models::candle::Candle;
use crate::price_grid::GridLayer;
use std::collections::HashMap;
use data_engine::data_model as dm;
use chrono::{TimeZone, Utc};

#[derive(Debug, Clone)]
pub struct SpgTracker {
    // Stores the five active grids, keyed by their layer type
    pub active_grids: HashMap<GridLayer, AnchoredGrid>,
}

impl SpgTracker {
//     /// Initializes the CampaignTracker with placeholder or initial (e.g., Yearly) grids.


//         pub fn anchor_yearly_grid(
//         &mut self,
//         prev_year: &[Candle]
//     ) -> Result<(), String> {
//         if prev_year.is_empty() {
//             return Err("Cannot anchor yearly grid: Input candle data is empty.".to_string());
//         }

//         // Determine High, Low, and Time Bounds
//         let mut yearly_high = f64::MIN;
//         let mut yearly_low = f64::MAX;
        
//         let start_timestamp = prev_year[0].timestamp;
//         let end_timestamp = prev_year.last().map(|c| c.timestamp).unwrap_or(start_timestamp);

//         for candle in prev_year {
//             if candle.high > yearly_high {
//                 yearly_high = candle.high;
//             }
//             if candle.low < yearly_low {
//                 yearly_low = candle.low;
//             }
//         }

//         //  Validate and Create the Grid
//         if yearly_low >= yearly_high {
//             return Err("Invalid yearly range: Low is not strictly less than High.".to_string());
//         }

//         let yearly_grid = match AnchoredGrid::new(
//             GridLayer::Yearly, 
//             yearly_high, 
//             yearly_low, 
//             start_timestamp, 
//             end_timestamp
//         ) {
//             Ok(grid) => grid,
//             Err(e) => return Err(format!("Failed to create Yearly SPG: {}", e)),
//         };

//         //  Store the Anchored Grid
//         self.update_grid(yearly_grid);

//         Ok(())
//     }
    
    
//     pub fn anchor_weekly_grid(
//         &mut self,
//         prev_week: &[Candle]
//     ) -> Result<(), String> {
//         if prev_week.is_empty() {
//             return Err("Cannot anchor weekly grid: Input candle data is empty.".to_string());
//         }

//         //  Determine High, Low, and Time Bounds
//         let mut weekly_high = f64::MIN;
//         let mut weekly_low = f64::MAX;
        
//         let start_timestamp = prev_week[0].timestamp;
//         let end_timestamp = prev_week.last().map(|c| c.timestamp).unwrap_or(start_timestamp); 

//         for candle in prev_week {
//             if candle.high > weekly_high {
//                 weekly_high = candle.high;
//             }
//             if candle.low < weekly_low {
//                 weekly_low = candle.low;
//             }
//         }

//         // Validate and Create the Grid
//         if weekly_low >= weekly_high {
//             return Err("Invalid weekly range: Low is not strictly less than High.".to_string());
//         }

//         let weekly_grid = match AnchoredGrid::new(
//             GridLayer::Weekly, 
//             weekly_high, 
//             weekly_low, 
//             start_timestamp, 
//             end_timestamp
//         ) {
//             Ok(grid) => grid,
//             Err(e) => return Err(format!("Failed to create Weekly SPG: {}", e)),
//         };

//         // Store the Anchored Grid
//         self.update_grid(weekly_grid);

//         Ok(())
//     }


//     pub fn anchor_critical_grid(
//         &mut self,
//         h8_data: &[Candle]
//     ) -> Result<(), String> {
//         if h8_data.is_empty() {
//             return Err("Cannot anchor critical grid: Input candle data is empty.".to_string());
//         }

//         let start_ts = h8_data.first().unwrap().timestamp;
//         let end_ts = h8_data.last().unwrap().timestamp;
        
//         let potential_high = h8_data.iter().map(|c| c.high).fold(f64::MIN, f64::max);
//         let potential_low = h8_data.iter().map(|c| c.low).fold(f64::MAX, f64::min);
        
//         let mut new_high = potential_high;
//         let mut new_low = potential_low;
//         let mut new_start_ts = start_ts;
        
//         // ---  Check for Initial Grid Creation ---
//         if self.get_grid(GridLayer::Critical).is_none() {
//             let initial_grid = AnchoredGrid::new(
//                 GridLayer::Critical, potential_high, potential_low, start_ts, end_ts
//             )?;
//             self.update_grid(initial_grid);
//             return Ok(());
//         }

//         // Check against Previous Grid (R_N-1) ---
//         let r_n_minus_1 = self.get_grid(GridLayer::Critical).unwrap();
//         let spg_prev = &r_n_minus_1.spg;
        
//         // --- Rule 3: Range Satisfaction (The highest priority check) ---
//         let exit_upper = spg_prev.get_level_price(PriceLevel::ExitUpper).unwrap_or(f64::MAX);
//         let exit_lower = spg_prev.get_level_price(PriceLevel::ExitLower).unwrap_or(f64::MIN);
        
//         if potential_high >= exit_upper || potential_low <= exit_lower {
//             new_start_ts = end_ts;  
//         } 
        
//         // --- Rule 1 & 2: Initial Rejection and Mid-Point Validation ---
//         else if let Some(rejection_type) = is_initial_rejection(spg_prev, potential_high, potential_low) {
            
//             // Check Rule 2: Did the price pullback far enough to confirm the rejection?
            
//             let is_mid_point_pullback_met = true; 
            
//             if is_mid_point_pullback_met {
//                 match rejection_type {
//                     GridLayer::Critical => { 
//                         new_high = potential_high;
//                         new_low = r_n_minus_1.spg.low;
//                         new_start_ts = r_n_minus_1.start_timestamp; 
//                     },
//                     GridLayer::MultiDay => { 
//                         new_high = r_n_minus_1.spg.high;
//                         new_low = potential_low;
//                         new_start_ts = r_n_minus_1.start_timestamp;
//                     },
//                     _ => unreachable!(),
//                 }
//             } else {

//                 return Ok(());
//             }
//         } else {

//             return Ok(());
//         }

//         //  Create and Store the New Anchored Grid
//         let final_grid = AnchoredGrid::new(
//             GridLayer::Critical, new_high, new_low, new_start_ts, end_ts
//         )?;
//         self.update_grid(final_grid);

//         Ok(())
//     }

//     pub fn anchor_multi_day_grid(
//         &mut self,
//         daily: &[Candle]
//     ) -> Result<(), String> {
//         if daily.is_empty() {
//             return Err("Cannot anchor multi-day grid: Input candle data is empty.".to_string());
//         }

//         let end_ts = daily.last().unwrap().timestamp;
//         let mut new_high = f64::MIN;
//         let mut new_low = f64::MAX;
//         let mut high_ts = 0;
//         let mut low_ts = 0;
        
//         let prev_grid = self.get_grid(GridLayer::MultiDay).cloned();

//         //  Check for Range Completion of R_N-1 (if a previous grid exists)
//         if let Some(r_n_minus_1) = prev_grid.clone() {
//             // Price action since the last anchor was set
//             let subsequent_candles: Vec<Candle> = daily.iter()
//                 .filter(|c| c.timestamp >= r_n_minus_1.end_timestamp)
//                 .cloned()
//                 .collect();

//             if subsequent_candles.is_empty() {
//                 return Ok(()); 
//             }

//             if has_multi_day_range_completed(&r_n_minus_1.spg, &subsequent_candles) {

//             } else {
//                 // R_N-1 is NOT completed. The grid remains anchored to R_N-1.
//                 self.update_grid(r_n_minus_1);
//                 return Ok(());
//             }
//         }
        
//         //  Search for the widest, valid Multi-Day Swing
//         for (i, candle) in daily.iter().rev().enumerate() {
            
//             if candle.high >= new_high {
//                 new_high = candle.high;
//                 high_ts = candle.timestamp;
//             }

//             if candle.low <= new_low {
//                 new_low = candle.low;
//                 low_ts = candle.timestamp;
//             }
//         }

//         //  Apply the "Different Days" Constraint
//         let high_day = Utc.timestamp_millis(high_ts as i64).date_naive();
//         let low_day = Utc.timestamp_millis(low_ts as i64).date_naive();

//         if high_day == low_day {
//             return Err("Multi-Day anchor failed: High and Low set on the same day.".to_string());
//         }

//         // Create and Store the New Anchored Grid
//         let start_ts = if high_ts < low_ts { high_ts } else { low_ts };
        
//         let final_grid = AnchoredGrid::new(
//             GridLayer::MultiDay, new_high, new_low, start_ts, end_ts
//         )?;
//         self.update_grid(final_grid);

//         Ok(())
//     }

//     pub fn anchor_previous_day_grid(
//         &mut self,
//         prev_day: &[Candle]
//     ) -> Result<(), String> {
//         if prev_day.is_empty() {
//             return Err("Cannot anchor previous day grid: Input candle data is empty.".to_string());
//         }

//         //  Determine High, Low, and Time Bounds
//         let mut daily_high = f64::MIN;
//         let mut daily_low = f64::MAX;
        
//         // Time bounds are set precisely by the first and last candle of the custom day
//         let start_timestamp = prev_day[0].timestamp;
//         let end_timestamp = prev_day.last().map(|c| c.timestamp).unwrap_or(start_timestamp);

//         for data in prev_day {
//             if data.high > daily_high {
//                 daily_high = data.high;
//             }
//             if data.low < daily_low {
//                 daily_low = data.low;
//             }
//         }

//         // Validate and Create the Grid
//         if daily_low >= daily_high {
//             return Err("Invalid previous day range: Low is not strictly less than High.".to_string());
//         }

//         let daily_grid = match AnchoredGrid::new(
//             GridLayer::PreviousDay, 
//             daily_high, 
//             daily_low, 
//             start_timestamp, 
//             end_timestamp
//         ) {
//             Ok(grid) => grid,
//             Err(e) => return Err(format!("Failed to create Previous Day SPG: {}", e)),
//         };

//         //  Store the Anchored Grid
//         self.update_grid(daily_grid);

//         Ok(())
//     }

}






// // Checks if a swing high or low constitutes an Initial Rejection (Rule 1).
// fn is_initial_rejection(
//     prev_spg: &SymmetricPriceGrid,
//     current_high: f64,
//     current_low: f64
// ) -> Option<GridLayer> { 
//     let p1_25 = prev_spg.get_level_price(PriceLevel::MidPointUpper).unwrap();
//     let p_0_25 = prev_spg.get_level_price(PriceLevel::MidPointLower).unwrap();

//     // Check for High Rejection (R-Swing)
//     if current_high > prev_spg.high && current_high < p1_25 {
//         return Some(GridLayer::Critical); 
//     }

//     // Check for Low Rejection (L-Swing)
//     if current_low < prev_spg.low && current_low > p_0_25 {
//         return Some(GridLayer::MultiDay); 
//     }

//     None
// }

// // Checks if the price action since the range was anchored has achieved the minimum 40% retracement (completion signal).
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


// ------------

// impl SpgTracker {
//     pub fn new() -> Self {
//         SpgTracker { active_grids: HashMap::new() }
//     }

//     /// L1: Yearly Anchor (from dm::ClassifiedYearlyView)
//     pub fn anchor_yearly(&mut self, view: &dm::ClassifiedYearlyView) -> Result<(), String> {
//         let grid = AnchoredGrid::new(
//             GridLayer::Yearly,
//             view.high,
//             view.low,
//             view.year_start as u64,
//             view.last_update_ts as u64,
//         )?;
//         self.active_grids.insert(GridLayer::Yearly, grid);
//         Ok(())
//     }

//     /// L3: Weekly Anchor (from dm::ClassifiedWeeklyView)
//     pub fn anchor_weekly(&mut self, view: &dm::ClassifiedWeeklyView) -> Result<(), String> {
//         let grid = AnchoredGrid::new(
//             GridLayer::Weekly,
//             view.high,
//             view.low,
//             view.week_start_ts as u64,
//             view.last_update_ts as u64,
//         )?;
//         self.active_grids.insert(GridLayer::Weekly, grid);
//         Ok(())
//     }

//     /// L5: Previous Day Anchor (from dm::ClassifiedDailyView)
//     pub fn anchor_previous_day(&mut self, view: &dm::ClassifiedDailyView) -> Result<(), String> {
//         let grid = AnchoredGrid::new(
//             GridLayer::PreviousDay,
//             view.high,
//             view.low,
//             view.open_ts as u64,
//             view.close_ts as u64,
//         )?;
//         self.active_grids.insert(GridLayer::PreviousDay, grid);
//         Ok(())
//     }

//     // Helper to get grids for your Veto system
//     pub fn get_grid(&self, layer: GridLayer) -> Option<&AnchoredGrid> {
//         self.active_grids.get(&layer)
//     }

//         pub fn anchor_critical_from_etl(
//         &mut self,
//         current_session_high: f64,
//         current_session_low: f64,
//         start_ts: u64,
//         end_ts: u64
//     ) -> Result<(), String> {
        
//         // 1. If no Critical Grid exists, create one from the session data
//         let r_n_minus_1 = match self.get_grid(GridLayer::Critical) {
//             Some(grid) => grid.clone(),
//             None => {
//                 let initial = AnchoredGrid::new(GridLayer::Critical, current_session_high, current_session_low, start_ts, end_ts)?;
//                 self.update_grid(initial);
//                 return Ok(());
//             }
//         };

//         let spg_prev = &r_n_minus_1.spg;
//         let mut new_high = current_session_high;
//         let mut new_low = current_session_low;
//         let mut new_start_ts = start_ts;

//         // 2. Rule 3: Range Satisfaction (Exit Levels)
//         let exit_upper = spg_prev.get_level_price(PriceLevel::ExitUpper).unwrap_or(f64::MAX);
//         let exit_lower = spg_prev.get_level_price(PriceLevel::ExitLower).unwrap_or(f64::MIN);

//         if current_session_high >= exit_upper || current_session_low <= exit_lower {
//             // Price hit the target extension; we reset the anchor to the current session
//             new_start_ts = end_ts;  
//         } 
        
//         // 3. Rule 1: Initial Rejection check
//         else if let Some(rejection_type) = is_initial_rejection(spg_prev, current_session_high, current_session_low) {
//             // We only update the side of the grid that was rejected
//             match rejection_type {
//                 GridLayer::Critical => { 
//                     new_high = current_session_high;
//                     new_low = spg_prev.low;
//                     new_start_ts = r_n_minus_1.start_timestamp; 
//                 },
//                 GridLayer::MultiDay => { 
//                     new_high = spg_prev.high;
//                     new_low = current_session_low;
//                     new_start_ts = r_n_minus_1.start_timestamp;
//                 },
//                 _ => {}
//             }
//         } else {
//             // No rules triggered; keep the previous grid
//             return Ok(());
//         }

//         let final_grid = AnchoredGrid::new(GridLayer::Critical, new_high, new_low, new_start_ts, end_ts)?;
//         self.update_grid(final_grid);
//         Ok(())
//     }
// }


// /// Checks if the current high/low constitutes an Initial Rejection (Rule 1).
// fn is_initial_rejection(
//     prev_spg: &SymmetricPriceGrid,
//     current_high: f64,
//     current_low: f64
// ) -> Option<GridLayer> { 
//     // These levels come from the math we just updated
//     let p1_25 = prev_spg.get_level_price(PriceLevel::MidPointUpper).unwrap_or(f64::MAX);
//     let p_0_25 = prev_spg.get_level_price(PriceLevel::MidPointLower).unwrap_or(f64::MIN);

//     // Check for High Rejection (R-Swing)
//     // Price went above the old high but failed before hitting the P1.25 extension
//     if current_high > prev_spg.high && current_high < p1_25 {
//         return Some(GridLayer::Critical); 
//     }

//     // Check for Low Rejection (L-Swing)
//     // Price went below the old low but failed before hitting the P-0.25 extension
//     if current_low < prev_spg.low && current_low > p_0_25 {
//         return Some(GridLayer::MultiDay); 
//     }

//     None
// }


impl SpgTracker {
    /// Bootstraps the tracker using the Prior Day/Week levels found in the Daily View.
    /// 
    
        pub fn new() -> Self {
        // In a real system, initial anchors (like Yearly) would be loaded here.
        // For now, we initialize an empty tracker.
        SpgTracker {
            active_grids: HashMap::new(),
        }
    }

    /// Adds or updates an AnchoredGrid in the tracker.
    pub fn update_grid(&mut self, grid: AnchoredGrid) {
        self.active_grids.insert(grid.layer, grid);
    }

    /// Retrieves a grid by its layer, useful for implementing the Veto system.
    pub fn get_grid(&self, layer: GridLayer) -> Option<&AnchoredGrid> {
        self.active_grids.get(&layer)
    }
    
    /// Anchors the Previous Day grid (L5) using the ClassifiedDailyView.
    pub fn anchor_daily_grid(&mut self, view: &dm::ClassifiedDailyView) -> Result<(), String> {
        
        if let (Some(h), Some(l)) = (view.prior_day_high, view.prior_day_low) {
            let daily_grid = AnchoredGrid::new(
                GridLayer::PreviousDay,
                h,
                l,
                view.start_ts.timestamp_millis() as u64, // Placeholder for actual start of prior day
                view.end_ts.timestamp_millis() as u64,
            )?;
            self.active_grids.insert(GridLayer::PreviousDay, daily_grid);
        }
        Ok(())
    }

    // 2. Anchor the Previous Week (L3)
        // This provides the higher-timeframe Veto context.

    pub fn anchor_weekly_grid(&mut self, view: &dm::ClassifiedDailyView) -> Result<(), String> {

        if let (Some(wh), Some(wl)) = (view.prior_week_high, view.prior_week_low) {
            let weekly_grid = AnchoredGrid::new(
                GridLayer::Weekly,
                wh,
                wl,
                view.start_ts.timestamp_millis() as u64,
                view.end_ts.timestamp_millis() as u64,
            )?;
            self.active_grids.insert(GridLayer::Weekly, weekly_grid);
        }

        Ok(())
    }
}

pub async fn initialize_tracker(
    data_service: &dm::DataService, 
    asset_id: &str
) -> Result<Self, String> {
    let mut tracker = SpgTracker::new(asset_id);

    // --- 1. Fetch Yearly Views (L1 Anchor) ---
    let yearly_data = data_service.yearly_views(asset_id).await
        .map_err(|e| format!("Failed to fetch yearly views: {}", e))?;

    // DEBUG: Let's see what the DB actually returned
    println!("DEBUG [{}]: Yearly rows found: {}", asset_id, yearly_data.len());

    let current_year = Utc::now().year(); 
    
    // Find the most recent year that is NOT the current year (the complete anchor)
    if let Some(prev_year) = yearly_data.iter().rev().find(|y| y.year_start.year() < current_year) {
        tracker.anchor_yearly_grid(prev_year)?;
        println!("✅ [{}] Bound L1 Yearly Anchor to year: {}", asset_id, prev_year.year_start.year());
    } else {
        println!("⚠️ [{}] No suitable Previous Year found in history for L1.", asset_id);
    }

    // --- 2. Fetch Weekly History ---
    // ... (rest of your weekly code) ...

    // --- 3. Fetch Daily View ---
    let daily_data = data_service.daily_views(asset_id).await
        .map_err(|e| format!("Failed to fetch daily views: {}", e))?;

    if let Some(v) = daily_data.last() {
        tracker.anchor_daily_grid(v)?;
        
        if let Some(prior_week_grid) = tracker.weekly_chain.first() {
            tracker.active_grids.insert(GridLayer::Weekly, prior_week_grid.clone());
        }

        // --- VERIFICATION REPORTING ---
        println!("\n============================================================");
        println!("🔍 SPG VERIFICATION REPORT: {}", asset_id);
        println!("============================================================");

        // ADDED: Print Yearly Grid here
        if let Some(yearly) = &tracker.yearly_grid {
            tracker.print_grid_report("L1 (PREVIOUS YEAR ANCHOR)", yearly);
        } else {
            println!("\n--- L1 (PREVIOUS YEAR ANCHOR) ---");
            println!("  MISSING: No Yearly Grid Anchored.");
        }

        if let Some(daily) = &tracker.daily_grid {
            tracker.print_grid_report("L5 (PREVIOUS DAY GRID)", daily);
        }

        // ... (rest of the weekly chain printing) ...
    }
    
    Ok(tracker)
}

async fn calculate_session_and_context(
    &self, 
    asset_id: &str,
    lookback_hours: Option<i64> // Just like your lookback_days
) -> Result<(Vec<dm::ClassifiedSession>, Vec<sm::SessionContextData>)> {
    
    // 1. Determine starting point
    let hwm: Option<DateTime<Utc>> = self.get_hwm("session_context", "session_end_ts", asset_id).await?;
    
    let start_ts = if let Some(hours) = lookback_hours {
        // Cold Boot: Force fetch X hours of history for the Tracker
        Some(Utc::now() - Duration::hours(hours))
    } else {
        // Incremental: Use HWM for normal background updates
        hwm
    };

    // 2. Fetch from session_base using the buffer
    let raw: Vec<dm::RawSessionData> = self.fetch_buffered(
        "SELECT * FROM session_base WHERE asset_id = $1", 
        asset_id, 
        start_ts, 
        Duration::hours(24), // Keep the 24h buffer for context calculation
        "end_ts" 
    ).await?;

    if raw.is_empty() { return Ok((Vec::new(), Vec::new())); }

    // 3. Map to Classified (Your existing logic)
    let mut classified: Vec<dm::ClassifiedSession> = raw.into_iter().map(|s| {
        // ... (Mapping code) ...
    }).collect();
    classified.sort_by_key(|s| s.end_ts);

    // 4. Generate Context (Lookback needed for p1, p2)
    let contexts = generate_context(&classified, |curr, p1, p2| {
        // ... (Context generation code) ...
    });

    // 5. THE CRITICAL SPLIT (Protects the Database)
    // We only return the history in 'classified' for the Tracker.
    // We only return NEW 'contexts' for the Database to save.
    let limit = hwm.unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());

    Ok((
        classified, // The Tracker gets the full history (CS, PS1, PS2)
        contexts.into_iter().filter(|c| c.session_end_ts > limit).collect() // DB only gets NEW
    ))
}


async fn calculate_session_and_context(
    &self, 
    asset_id: &str,
    lookback_hours: Option<i64>
) -> Result<(Vec<dm::ClassifiedSession>, Vec<sm::SessionContextData>)> {
    
    let hwm: Option<DateTime<Utc>> = self.get_hwm("session_context", "session_end_ts", asset_id).await?;
    
    // 1. Pivot the limit based on input (Cold Boot vs Incremental)
    let start_ts = if let Some(hours) = lookback_hours {
        Utc::now() - Duration::hours(hours)
    } else {
        hwm.unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
    };

    // 2. Fetch with the buffer to ensure the very first session has its p1/p2
    let raw: Vec<dm::RawSessionData> = self.fetch_buffered(
        "SELECT * FROM session_base WHERE asset_id = $1", 
        asset_id, 
        Some(start_ts), // Use the start_ts as the fetch anchor
        Duration::hours(24), 
        "end_ts" 
    ).await?;

    if raw.is_empty() { return Ok((Vec::new(), Vec::new())); }

    // 3. Mapping
    let mut all_classified: Vec<dm::ClassifiedSession> = raw.into_iter().map(|s| {
        /* ... your mapping code ... */
    }).collect();
    all_classified.sort_by_key(|s| s.end_ts);

    // 4. Context Generation (uses the buffer data)
    let all_contexts = generate_context(&all_classified, |curr, p1, p2| {
        /* ... your context code ... */
    });

    // 5. Final Filter & Sort (THE FIX)
    // We filter by start_ts so the return matches the 'lookback' request exactly.
    let mut history_result: Vec<_> = all_classified.into_iter()
        .filter(|s| s.end_ts >= start_ts)
        .collect();
    
    let mut context_result: Vec<_> = all_contexts.into_iter()
        .filter(|c| c.session_end_ts >= start_ts)
        .collect();

    // Chronological order is mandatory for tracker.session_chain.push() logic
    history_result.sort_by_key(|s| s.end_ts);
    context_result.sort_by_key(|c| c.session_end_ts);

    Ok((history_result, context_result))
}

impl SpgTracker {
    /// KAFKA FAST PATH: Process every incoming tick
    pub fn process_live_update(&mut self, data: &MarketData) {
        let price = data.close;
        let ts = data.timestamp;
        
        // 1. Convert timestamp to Enum using your SessionUtil
        let incoming_session_enum = get_trading_session(ts.timestamp_millis())
            .unwrap_or(TradingSession::Unknown);
        let incoming_session_name = format!("{:?}", incoming_session_enum);

        // 2. Detect Transition
        let needs_transition = self.session_history.last()
            .map(|last| last.session_name != incoming_session_name)
            .unwrap_or(false);

        if needs_transition {
            self.handle_session_transition(incoming_session_name, ts, price);
        }

        // 3. Update the Current Active Grid (Index 0)
        if let Some(cs_grid) = self.session_chain.get_mut(0) {
            cs_grid.update(price);
        }

        // 4. Update the Active History Record (The "Draft")
        if let Some(last_session) = self.session_history.last_mut() {
            if price > last_session.high { 
                last_session.high = price; 
                last_session.high_ts = ts;
            }
            if price < last_session.low { 
                last_session.low = price; 
                last_session.low_ts = ts;
            }
            last_session.close = price;
            last_session.end_ts = ts;
            last_session.bars += 1;
            // Note: Volume is usually sum of ticks or provided by feed
        }

        self.apply_simple_tick(price);
    }

    /// Internal: The "Shift" that makes CS -> PS1 instantly
    fn handle_session_transition(&mut self, new_name: String, ts: DateTime<Utc>, price: f64) {
        info!("🚀 SESSION TRANSITION [{}] -> New Session: {}", self.asset_id, new_name);

        let new_draft = ClassifiedSession {
            asset_id: self.asset_id.clone(),
            session_name: new_name,
            start_ts: ts,
            end_ts: ts,
            open: price,
            high: price,
            low: price,
            close: price,
            high_ts: ts,
            low_ts: ts,
            ..Default::default()
        };

        self.session_history.push(new_draft);

        // Keep last 6 sessions to ensure we have CS, PS1, PS2 plus buffers
        if self.session_history.len() > 6 {
            self.session_history.remove(0);
        }

        // Re-anchor to move old CS grid into PS1 slot
        let _ = self.anchor_session_chain();
    }

    /// DATABASE SYNC: Reconciles Kafka Drafts with DB Final records
    pub fn sync_sessions_with_db(&mut self, verified_sessions: Vec<ClassifiedSession>) {
        let mut modified = false;

        for verified in verified_sessions {
            if let Some(draft) = self.session_history.iter_mut().find(|s| 
                s.session_name == verified.session_name && 
                s.start_ts.date_naive() == verified.start_ts.date_naive()
            ) {
                // If DB has different High/Low, update the draft to become Verified
                if (draft.high - verified.high).abs() > f64::EPSILON || 
                   (draft.low - verified.low).abs() > f64::EPSILON {
                    
                    debug!("⚖️ SYNC [{}] Reconciling session: {}", self.asset_id, verified.session_name);
                    
                    draft.high = verified.high;
                    draft.low = verified.low;
                    draft.open = verified.open;
                    draft.close = verified.close;
                    draft.high_ts = verified.high_ts;
                    draft.low_ts = verified.low_ts;
                    
                    modified = true;
                }
            }
        }

        if modified {
            let _ = self.anchor_session_chain();
        }
    }
}

    fn evaluate_market_context(&mut self, data: &MarketData) {
        if let Some(daily) = &self.daily_grid {
            let anchor_low = daily.get_price(PriceLevel::Zero).unwrap_or(f64::MAX);
            let exit_lower = daily.get_price(PriceLevel::ExitLower).unwrap_or(f64::MAX);

            // 1. DETECTION: Are we in the "Trap Zone" (Below Zero but above Exit Lower)?
            if data.close < anchor_low && data.close > exit_lower {
                
                // 2. MOMENTUM: Is volume actually supporting this drop?
                // For now, let's just flag the breach
                let breach_depth = anchor_low - data.close;
                
                if breach_depth > 0.0 {
                    info!(
                        "⚠️ TRAP WATCH [{}] Price is {:.2} below Anchor Low. Monitoring for Reclaim...", 
                        self.asset_id, 
                        breach_depth
                    );
                }
            }

            // 3. RECLAIM LOGIC: If we were below and now we closed back ABOVE Zero
            if self.last_price <= anchor_low && data.close > anchor_low {
                info!(
                    "🔥 ALGO ALERT [{}] Daily Anchor Low RECLAIMED. Potential Bear Trap confirmed.", 
                    self.asset_id
                );
            }
        }
    }


pub fn process_live_update(&mut self, data: &MarketData) {
        let price = data.close;
        let ts_dt = DateTime::<Utc>::from_timestamp_millis(data.ts).unwrap_or_else(|| Utc::now());
        let ny_now = ts_dt.with_timezone(&New_York);
        let current_date = ny_now.date_naive();

        // 1. SESSION (Live - Expanding)
        let incoming_session = get_trading_session(data.ts).unwrap_or(TradingSession::Unknown);
        if self.should_shift_session(incoming_session) {
            self.transition_session(incoming_session, ts_dt, price);
        }
        self.session_layer.update_live(price, false); // Session is NEVER static

        // 2. DAILY (Static Anchor)
        if current_date > self.last_daily_anchor_date {
            info!("🌅 NEW DAY DETECTED: Shifting Daily Anchor");
            // Here, we don't 'update' the grid with live price. 
            // We trigger a 'sync' or 're-anchor' from the finalized DB data.
            self.refresh_static_layer(GridLayer::Daily); 
            self.last_daily_anchor_date = current_date;
        }

        // 3. WEEKLY (Static Anchor Chain)
        let current_week_start = current_date - Duration::days(current_date.weekday().num_days_from_monday() as i64);
        if current_week_start > self.last_weekly_anchor_date {
            info!("📅 NEW WEEK DETECTED: Shifting Weekly Chain");
            self.refresh_static_layer(GridLayer::Weekly);
            self.last_weekly_anchor_date = current_week_start;
        }
    }

    impl AnchoredGrid {
    pub fn new(layer: GridLayer, high: f64, low: f64, start_ts: u64, end_ts: u64) -> Result<Self, String> {
        if low >= high {
            return Err(format!("Invalid anchors for {:?}: low ({}) >= high ({})", layer, low, high));
        }
        let spg = SymmetricPriceGrid::new(high, low);
        Ok(AnchoredGrid { layer, spg, start_timestamp: start_ts, end_timestamp: end_ts })
    }

    /// This is ONLY used by the Session Layer (Live Execution)
    pub fn update(&mut self, price: f64) {
        let mut changed = false;
        let mut new_high = self.spg.anchor_high;
        let mut new_low = self.spg.anchor_low;

        if price > new_high { new_high = price; changed = true; }
        if price < new_low { new_low = price; changed = true; }

        if changed {
            // Re-anchor the math grid
            self.spg = SymmetricPriceGrid::new(new_high, new_low);
        }
    }
}


impl SpgTracker {
    /// --- THE GENERIC REFRESH GATEWAY ---
    /// Call this from your live loop when the date changes.
    pub async fn refresh_layer(
        &mut self, 
        layer: GridLayer, 
        data_service: &dm::DataService
    ) -> Result<(), String> {
        match layer {
            GridLayer::PreviousDay => self.sync_daily_layer(data_service).await,
            GridLayer::Weekly => self.sync_weekly_layer(data_service).await,
            GridLayer::Yearly => self.sync_yearly_layer(data_service).await,
            GridLayer::Session => self.sync_session_layer(data_service).await,
            _ => Ok(()),
        }
    }

    /// --- DAILY LAYER ---
    pub async fn sync_daily_layer(&mut self, ds: &dm::DataService) -> Result<(), String> {
        let daily_data = ds.daily_views(&self.asset_id).await
            .map_err(|e| e.to_string())?;

        if let Some(v) = daily_data.last() {
            self.anchor_daily_grid(v)?;
            info!("✅ [{}] Daily Layer Synced", self.asset_id);
        }
        Ok(())
    }

    /// --- WEEKLY LAYER ---
    pub async fn sync_weekly_layer(&mut self, ds: &dm::DataService) -> Result<(), String> {
        // Fetch 30 days of history to find the last 3 completed weeks
        let weekly_data = ds.weekly_views(&self.asset_id, Some(30)).await
            .map_err(|e| e.to_string())?;

        let now = Utc::now().date_naive();
        let current_week_start = now - chrono::Duration::days(now.weekday().num_days_from_monday() as i64);

        // Clear the old chain and rebuild
        self.weekly_chain.clear();
        for week in weekly_data {
            if week.week_start < current_week_start {
                self.add_weekly_anchor_to_chain(&week)?;
            }
        }
        
        // Update the active map for context lookups
        if let Some(prior_week_grid) = self.weekly_chain.first() {
            self.active_grids.insert(GridLayer::Weekly, prior_week_grid.clone());
        }
        Ok(())
    }

    /// --- SESSION LAYER ---
    pub async fn sync_session_layer(&mut self, ds: &dm::DataService) -> Result<(), String> {
        let (sessions, _) = ds.calculate_session_and_context(&self.asset_id, Some(24)).await
            .map_err(|e| e.to_string())?;

        self.session_history = sessions.into_iter().rev().take(5).collect();
        self.session_history.reverse(); 
        self.anchor_session_chain()?;
        Ok(())
    }
}


#[derive(Debug, Clone)]
pub struct SpgTracker {
    pub asset_id: String,
    // We group layers here
    pub session_layer: LayerManager<dm::ClassifiedSession>,
    pub daily_layer: LayerManager<dm::ClassifiedDailyView>,
    pub weekly_layer: LayerManager<dm::ClassifiedWeeklyView>,
    pub yearly_layer: LayerManager<dm::ClassifiedYearlyView>,
    
    pub active_grids: HashMap<GridLayer, AnchoredGrid>,
    pub last_price: f64,
    pub last_daily_date: chrono::NaiveDate,
}

impl SpgTracker {
    pub fn process_live_update(&mut self, data: &MarketData) {
        let price = data.close;
        let ts_dt = DateTime::<Utc>::from_timestamp_millis(data.ts)
            .unwrap_or_else(|| Utc::now());
        let ny_date = ts_dt.with_timezone(&chrono_tz::America::New_York).date_naive();

        // --- 1. SESSION LAYER (DYNAMIC) ---
        let incoming_session_enum = get_trading_session(data.ts).unwrap_or(TradingSession::Unknown);
        let incoming_name = format!("{:?}", incoming_session_enum);

        let needs_session_transition = self.session_layer.history.last()
            .map(|last| last.session_name != incoming_name)
            .unwrap_or(false);

        if needs_session_transition {
            self.handle_session_transition(incoming_name, ts_dt, price);
        }

        // Session layer expands with every tick
        self.session_layer.update_live(price, false);

        // Update the raw history for the session (for the high/low/close data)
        if let Some(last_s) = self.session_layer.history.last_mut() {
            if price > last_s.high { last_s.high = price; last_s.high_ts = ts_dt; }
            if price < last_s.low { last_s.low = price; last_s.low_ts = ts_dt; }
            last_s.close = price;
        }

        // --- 2. STATIC LAYERS ---
        // These do NOT call update_live(price, false) because their anchors are fixed.
        // We only check if we need to trigger a Refresh from the DB.
        
        if ny_date > self.last_daily_date {
            info!("🌅 Date Change Detected: {} -> {}", self.last_daily_date, ny_date);
            // This flag can be picked up by your Manager/Engine to call refresh_layer()
            self.last_daily_date = ny_date;
            // Note: We don't update Daily/Weekly here; we wait for the DB Refresh to swap them.
        }

        self.last_price = price;
    }
}


impl SpgOrchestrator {
    /// The "Smart" Sync: Refresh DB Views -> Reconcile Trackers
    pub async fn smart_reconcile(&self) -> Result<(), String> {
        info!("🎯 Smart Sync Triggered: Starting Phase 1 (Database ETL)");

        // 1. Refresh Materialized Views (Phase 1 of your ETL)
        // This ensures L1/L3/L5 calculations are up to date in the DB
        self.data_service.assets_master_etl().await
            .map_err(|e| format!("Master ETL failed: {}", e))?;

        info!("✅ Phase 1 Complete. Starting Phase 2 (Tracker Reconciliation)");

        // 2. Pull the newly calculated levels into the live Trackers
        self.reconcile_all_trackers().await;

        info!("🚀 Smart Sync Finished. SPG Levels are now 100% verified.");
        Ok(())
    }
}

impl SpgOrchestrator {
    /// Initial startup: Load assets AND perform the first smart sync
    pub async fn bootstrap_assets(&self) -> Result<()> {
        info!("🥾 Bootstrapping SPG Engine...");
        
        // 1. Load basic asset config into memory
        self.load_active_assets().await?;

        // 2. Perform the first "Smart Sync" (ETL + Reconcile)
        // This ensures cold-start memory matches the Materialized Views
        if let Err(e) = self.smart_reconcile().await {
            error!("⚠️ Initial Smart Sync failed: {}. Continuing with live data.", e);
        }

        Ok(())
    }
}

// 🔍 SPG TRACKER READY: assets:US2000:FundedNext
// --- L1 YEARLY | Range: 2502.40 - 2505.20 ---
//   One : 2505.20
//   MidPoint : 2503.80
//   Zero : 2502.40
// --- L3 WEEKLY | Range: 2548.40 - 2590.40 ---
//   One : 2590.40
//   MidPoint : 2569.40
//   Zero : 2548.40
// --- L5 DAILY | Range: 2496.50 - 2520.50 ---
//   One : 2520.50
//   MidPoint : 2508.50
//   Zero : 2496.50
// --- CS SESSION | Range: 2502.40 - 2505.20 ---
//   One : 2505.20
//   MidPoint : 2503.80
//   Zero : 2502.40


#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenvy::dotenv().ok();

    let pool = setup_database_pool().await.context("DB Pool failed")?;
    
    // --- CLEAN SPG SETUP ---
    let data_service = DataService::new(pool.clone());
    
    // init_spg_engine now returns an orchestrator that is initially "Dormant" (is_live = false)
    let orchestrator = prediction_engine::symmetric_price_grid::orchestrator::init_spg_engine(data_service).await
        .map_err(|e| anyhow::anyhow!("SPG Init failed: {}", e))?;

    let watchdog_state = WatchdogState::new();
    let producer = ingestion::initialize_producer().await?;

    let python_worker_path = env::var("PYTHON_WORKER_PATH")
        .unwrap_or_else(|_| "./mt5_worker.py".to_string());

    // Join the tasks. The ingestion starts immediately, 
    // feeding the SYNC data into the DB while the Orchestrator waits.
    tokio::try_join!(
        async {
            info!("🛠️ Starting MT5 Watchdog...");
            let mut watchdog = Mt5Watchdog::new(&python_worker_path, watchdog_state.clone());
            watchdog.run().await.context("Watchdog service failed")
        },
        async {
            ingestion::run_multi_topic_consumer(
                pool, 
                &producer, 
                watchdog_state.clone(),
                orchestrator
            ).await.context("Ingestion Engine failed")
        }
    )?;

    Ok(())
}


pub async fn init_spg_engine(data_service: dm::DataService) -> Result<Arc<SpgOrchestrator>, String> {
    let orchestrator = Arc::new(SpgOrchestrator::new(data_service));

    // 1. Spawn the Recovery Task
    // This allows main to continue and start the Ingestion immediately
    let orch_clone = Arc::clone(&orchestrator);
    tokio::spawn(async move {
        info!("⏳ SPG Engine: Waiting 10s for initial Kafka Sync to land in DB...");
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        info!("🛡️ SYNC period grace ended. Running Master ETL...");
        if let Err(e) = orch_clone.data_service.assets_master_etl().await {
            error!("❌ Master ETL failed during recovery: {}", e);
        }

        info!("🥾 Anchoring Grids (Yearly/Weekly/Daily)...");
        if let Err(e) = orch_clone.bootstrap_assets().await {
            error!("❌ Bootstrap failed: {}", e);
        }

        // 2. THE GATE FLIP
        info!("🚀 SPG Engine is now LIVE and ANCHORED.");
        orch_clone.is_live.store(true, std::sync::atomic::Ordering::SeqCst);
        
        // 3. Start the background session-transition watcher
        start_spg_sync_task(Arc::clone(&orch_clone));
    });

    Ok(orchestrator)
}

// Ensure bootstrap_assets clears existing trackers if it's called during a re-sync
pub async fn bootstrap_assets(&self) -> Result<(), String> {
    let asset_ids = self.data_service.fetch_all_active_asset_ids().await
        .map_err(|e| format!("Bootstrap failed: {}", e))?;

    let mut tasks = Vec::with_capacity(asset_ids.len());
    for asset_id in asset_ids {
        let service = Arc::clone(&self.data_service);
        tasks.push(tokio::spawn(async move {
            let result = SpgTracker::initialize_tracker(&service, &asset_id).await;
            (asset_id, result)
        }));
    }

    let results = join_all(tasks).await;
    // We clear current trackers to ensure no "half-baked" anchors remain
    self.trackers.clear(); 

    for res in results {
        if let Ok((id, Ok(tracker))) = res {
            self.trackers.insert(id, tracker);
        }
    }
    info!("✅ Bootstrap completed. Trackers anchored: {}", self.trackers.len());
    Ok(())
}

use log::{info, error};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer, CommitMode}, 
    Message,
    TopicPartitionList,
    message::{OwnedMessage},
};
use sqlx::{PgPool, QueryBuilder, Postgres};
use shared_models::{market_data::MarketData, time_utils};
use crate::data_service::traits::DataServiceBase;
use crate::{producer_config::{
    IngestionCoordinatorConfig, SyncCommand},
    watchdog::WatchdogState,
}; 
use shared_models::data_model::DataService;
use anyhow::{Context, Result}; 
use std::time::Duration; 
use futures::StreamExt;
use std::env;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::collections::{HashSet, HashMap};
use std::sync::Arc;
use tokio::sync::{Mutex, watch};
use lazy_static::lazy_static;
use shared_models::traits::MarketDataHandler;

/// Executes a batch of market data inserts using Postgres ON CONFLICT logic.
async fn execute_batch(
    pool: &PgPool,
    batch: &Vec<MarketData>,
) -> Result<u64> {
    if batch.is_empty() {
        return Ok(0);
    }
    
    let timestamps: Vec<chrono::DateTime<chrono::Utc>> = batch.iter()
        .map(|data| {
            time_utils::ts_to_utc_datetime(data.ts)
                .context(format!("Failed to convert timestamp {} for asset {}", data.ts, data.asset_id))
        })
        .collect::<Result<Vec<_>>>()?;
    
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO market_data (time, asset_id, open, high, low, close, volume) "
    );

    query_builder.push_values(
        batch.iter().zip(timestamps.iter()),
        |mut b, (data, ts)| {
            b.push_bind(*ts)
             .push_bind(&data.asset_id)
             .push_bind(data.open)
             .push_bind(data.high)
             .push_bind(data.low)
             .push_bind(data.close)
             .push_bind(data.volume);
        }
    );

    query_builder.push(r#" 
        ON CONFLICT ("time", asset_id) DO UPDATE SET
            open = EXCLUDED.open,
            high = EXCLUDED.high,
            low = EXCLUDED.low,
            close = EXCLUDED.close,
            volume = EXCLUDED.volume
    "#);

    let rows_affected = query_builder.build()
        .execute(pool).await
        .context("Failed to execute batch insert statement")?
        .rows_affected();

    info!("Batch processed {} record(s) in Postgres", rows_affected);

    Ok(rows_affected)
}

pub async fn initialize_producer() -> Result<FutureProducer> {
    let brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string());
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .create()
        .context("Producer creation error")?;
    Ok(producer)
}

pub async fn run_multi_topic_consumer(
    pool: PgPool, 
    producer: &FutureProducer,
    watchdog_state: WatchdogState,
    orchestrator: Arc<dyn MarketDataHandler>,
    sync_tx: watch::Sender<bool>, // Signal to main/orchestrator when sync is done
) -> Result<()> {
    let config_path = env::var("INGESTION_CONFIG_PATH")
        .context("CRITICAL: Environment variable INGESTION_CONFIG_PATH must be set.")?;
        
    let coordinator_config = IngestionCoordinatorConfig::load_from_file(&config_path)
        .context(format!("Failed to load config from: {}", config_path))?;
        
    let data_service = DataService::new(pool.clone());
    publish_sync_requests(producer, &coordinator_config, &data_service).await
        .context("Failed to publish initial sync requests to Kafka")?;

    let mut topics = Vec::new();
    let mut asset_id_map = HashMap::new();

    for asset in coordinator_config.assets.iter().filter(|a| a.enabled) {
        let topic = format!("{}_{}", asset.system_symbol.to_lowercase(), asset.topic_base);
        let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), coordinator_config.data_source_id);
        
        topics.push(topic.clone());
        asset_id_map.insert(topic, asset_id);
    }

    let topic_refs: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();

    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "market_ingest_v2") 
        .set("bootstrap.servers", env::var("KAFKA_BROKERS").unwrap_or_else(|_| "127.0.0.1:9092".to_string()))
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest") 
        .create()
        .context("Consumer creation error")?;
        
    consumer.subscribe(&topic_refs).context("Failed to subscribe to topics")?;
    info!("Ingestion Engine online. Subscribed to: {:?}", topic_refs);

    const BATCH_SIZE: usize = 100;
    let mut batch: Vec<MarketData> = Vec::with_capacity(BATCH_SIZE);
    let mut last_message: Option<OwnedMessage> = None; 
    let mut message_stream = consumer.stream(); 
    let mut is_caught_up = false;

    loop {
        tokio::select! {
            message_result = message_stream.next() => {
                let msg = match message_result {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => { error!("Kafka error: {:?}", e); continue; },
                    None => break,
                };
                
                if let Some(payload) = msg.payload() {
                    let topic_name = msg.topic();
                    let asset_id = asset_id_map.get(topic_name).cloned().unwrap_or_default();

                    match serde_json::from_slice::<MarketData>(payload) {
                        Ok(mut market_data) => {
                            market_data.asset_id = asset_id.clone(); 
                            
                            // --- [ZERO LATENCY TAP] ---
                            // Note: SPG internally uses is_live to ignore sync data
                            orchestrator.on_price_update(&market_data).await;

                            // --- [DATABASE PERSISTENCE] ---
                            batch.push(market_data);
                            last_message = Some(msg.detach());
                            
                            if batch.len() >= BATCH_SIZE {
                                execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
                            }
                        },
                        Err(e) => { error!("Failed to deserialize: {:?}. Raw: {:?}", e, String::from_utf8_lossy(payload)); }
                    }
                }
            },
            
            // --- CATCH-UP & FLUSH MONITOR ---
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                // 1. Check Kafka Lag to signal sync completion
                if !is_caught_up {
                    if check_actual_lag(&consumer, &topics).await {
                        info!("🏁 Kafka Lag is 0. Signaling SPG to Bootstrap...");
                        let _ = sync_tx.send(true);
                        is_caught_up = true;
                    }
                }

                // 2. Regular interval flush
                if !batch.is_empty() {
                    info!("Interval reached. Flushing {} records.", batch.len());
                    execute_and_commit(&pool, &consumer, &mut batch, &last_message, &watchdog_state).await?;
                }
            }
        }
    }

    Ok(()) 
}

/// Helper with added In-Memory Deduplication to prevent Postgres "row affected a second time" error.
async fn execute_and_commit(
    pool: &PgPool, 
    consumer: &StreamConsumer, 
    batch: &mut Vec<MarketData>, 
    last_msg: &Option<OwnedMessage>,
    watchdog_state: &WatchdogState
) -> Result<()> {
    if batch.is_empty() { return Ok(()); }

    let mut dedup_map: HashMap<(chrono::DateTime<chrono::Utc>, String), MarketData> = HashMap::new();
    
    for item in batch.drain(..) {
        if let Ok(ts) = time_utils::ts_to_utc_datetime(item.ts) {
            let key = (ts, item.asset_id.clone());
            dedup_map.insert(key, item);
        }
    }
    
    let unique_batch: Vec<MarketData> = dedup_map.into_values().collect();
    execute_batch(pool, &unique_batch).await?;

    watchdog_state.update();

    if let Some(msg) = last_msg {
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(msg.topic(), msg.partition(), rdkafka::Offset::Offset(msg.offset() + 1))?;
        consumer.commit(&tpl, CommitMode::Async)?;
    }
    Ok(())
}

async fn check_actual_lag(consumer: &StreamConsumer, topics: &[String]) -> bool {
    for topic in topics {
        if let Ok((_low, high)) = consumer.fetch_watermarks(topic, 0, Duration::from_secs(1)) {
            if high > 0 {
                if let Ok(tpl) = consumer.position() {
                    let current = tpl.find_partition(topic, 0)
                        .map(|p| match p.offset() {
                            rdkafka::Offset::Offset(o) => o,
                            _ => 0,
                        }).unwrap_or(0);

                    if current < (high - 1) { return false; }
                }
            }
        }
    }
    true
}

lazy_static! {
    static ref SYNCED_ASSETS: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
}

pub async fn publish_sync_requests(
    producer: &FutureProducer,
    config: &IngestionCoordinatorConfig,
    data_service: &DataService,
) -> Result<()> {
    let mut tracker = SYNCED_ASSETS.lock().await;

    for asset in &config.assets {
        if !asset.enabled { continue; }
        let asset_id = format!("assets:{}:{}", asset.system_symbol, config.data_source_id);

        if tracker.contains(&asset_id) {
            info!("⏩ Skipping Sync Request for {} (already requested)", asset.system_symbol);
            continue;
        }

        let hwm = data_service.get_market_data_high_watermark(&asset_id).await?
            .unwrap_or(0); 

        let sync_cmd = SyncCommand {
            command: "SYNC".to_string(),
            system_symbol: asset.system_symbol.clone(),
            mt5_symbol: asset.mt5_symbol.clone(),
            start_timestamp_ms: hwm,
            timeframe: asset.timeframe.clone(),
        };

        let payload = serde_json::to_vec(&sync_cmd)?;
        producer.send(
            FutureRecord::to("market_control")
                .payload(&payload)
                .key(&asset.system_symbol),
            Duration::from_secs(5)
        ).await.map_err(|(e, _)| e)?;

       info!("Sent Sync Request for {} from HWM: {}", asset.system_symbol, hwm);
       tracker.insert(asset_id);
    }
    Ok(())
}

pub fn process_live_update(&mut self, data: &MarketData) {
    let price = data.close;
    let ts_dt = DateTime::<Utc>::from_timestamp_millis(data.ts).unwrap_or_else(|| Utc::now());
    let ny_date = ts_dt.with_timezone(&New_York).date_naive();

    // 1. Session Transition Detection
    let incoming_session = get_trading_session(data.ts).unwrap_or(TradingSession::Unknown);
    let incoming_name = format!("{:?}", incoming_session);

    let needs_transition = self.session_layer.history.last()
        .map(|last| last.session_name != incoming_name)
        .unwrap_or(false);

    if needs_transition {
        // handle_session_transition already calls rebuild_session_grids() internally
        self.handle_session_transition(incoming_name, ts_dt, price);
    }

    // 2. Expand Current Session Grid (Live Range)
    // This maintains the internal CS tracker
    self.session_layer.update_live(price, false);

    // 3. Update History Metadata (The 'CS' entry in history)
    if let Some(last_s) = self.session_layer.history.last_mut() {
        let mut expanded = false;
        if price > last_s.high { last_s.high = price; last_s.high_ts = ts_dt; expanded = true; }
        if price < last_s.low { last_s.low = price; last_s.low_ts = ts_dt; expanded = true; }
        last_s.close = price;

        // 4. Update Fast-Lookup Map for CS Grid
        // We only re-insert if the range expanded to save CPU cycles
        if expanded || needs_transition {
            let cs_grid = AnchoredGrid::new(
                GridLayer::CS, 
                last_s.high, 
                last_s.low, 
                last_s.start_ts.timestamp_millis() as u64, 
                0
            ).unwrap();
            self.active_grids.insert(GridLayer::CS, cs_grid);
        }
    }

    // 5. Date Change (Static Refresh Trigger)
    if ny_date > self.last_daily_date {
        info!("🌅 New Trading Day: {}. Flagging for DB Reconcile (Daily/Weekly Layers).", ny_date);
        self.last_daily_date = ny_date;
        
        // This tells the Orchestrator to run sync_daily_layer and sync_weekly_layer
        // to update PW, PW1, PW2, and PreviousDay
        self.needs_reconcile = true;
    }

    self.last_price = price;
}