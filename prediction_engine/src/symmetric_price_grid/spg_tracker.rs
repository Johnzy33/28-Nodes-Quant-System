

use crate::price_grid::{PriceLevel, GridLayer, LayerManager};
use crate::anchored_grid::AnchoredGrid;
use shared_models::{data_model as dm, get_trading_session};
use shared_models::market_data::MarketData_old;
use shared_models::session_utils::TradingSession;
use std::collections::HashMap;
use data_engine::traits::DataViewExt;
use chrono::{DateTime, Utc, TimeZone, Datelike};
use chrono_tz::America::New_York;
use log::{info,debug, warn};

#[derive(Debug, Clone, PartialEq)]
pub enum VetoReason {
    DailyMidpointRejection,
    HistoricalWeeklyMidpoint(usize),   // how many weeks back
    HistoricalWeeklyExhaustion(usize), // Price hit an old P1.5 or P-0.5
}

#[derive(Debug, Clone)]
pub struct SpgTracker {
    pub asset_id: String,
    // Layer Managers handle both Raw Data (History) and SPG Logic (Grids)
    pub session_layer: LayerManager<dm::ClassifiedSession>,
    pub daily_layer: LayerManager<dm::ClassifiedDailyView>,
    pub weekly_layer: LayerManager<dm::ClassifiedWeeklyView>,
    pub yearly_layer: LayerManager<dm::ClassifiedYearlyView>,
    
    // Fast-lookup for current context
    pub active_grids: HashMap<GridLayer, AnchoredGrid>,
    pub needs_reconcile: bool,
    
    pub last_price: f64,
    pub last_daily_date: chrono::NaiveDate,
    pub current_week_high: f64,
    pub current_week_low: f64,
}

impl SpgTracker {
    /// Initialize a new tracker with pre-allocated LayerManagers
    pub fn new(asset_id: &str) -> Self {
        let now = Utc::now().with_timezone(&New_York);
        
        Self {
            asset_id: asset_id.to_string(),
            // We need 5 sessions to ensure we can build PS1 and the composite PS2 reliably
            session_layer: LayerManager::new(GridLayer::CS, 5), 
            needs_reconcile: false,
            // Daily layer stays lean
            daily_layer: LayerManager::new(GridLayer::PreviousDay, 2),
            // We need at least 3 weeks of history to build the PW -> PW1 -> PW2 chain
            weekly_layer: LayerManager::new(GridLayer::PW, 5), 
            yearly_layer: LayerManager::new(GridLayer::Yearly, 2),
            
            active_grids: HashMap::new(),
            last_price: 0.0,
            last_daily_date: now.date_naive(),
            
            // These track the LIVE session/week for the CS and CW layers
            current_week_high: f64::MIN,
            current_week_low: f64::MAX,
        }
    }

    /// Primary initialization: Fetches all historical data and sets up anchors
    pub async fn initialize_tracker(
        data_service: &dm::DataService, 
        asset_id: &str
    ) -> Result<Self, String> {
        let mut tracker = SpgTracker::new(asset_id);

        // Populate all layers from SurrealDB
        tracker.sync_yearly_layer(data_service).await?;
        tracker.sync_weekly_layer(data_service).await?;
        tracker.sync_daily_layer(data_service).await?;
        tracker.sync_session_layer(data_service).await?;

        tracker.render_layout_dashboard();

        Ok(tracker)
    }

    // --- GENERIC SYNC INTERFACE ---

    pub async fn refresh_layer(
        &mut self, 
        layer: GridLayer, 
        ds: &dm::DataService
    ) -> Result<(), String> {

        match layer {
            // PW triggers the full 3-week rolling window calculation
            GridLayer::PW => self.sync_weekly_layer(ds).await,

            // CS/Session triggers both PS1 and the composite PS2 calculation
            GridLayer::CS => self.sync_session_layer(ds).await,

            // Standard layers
            GridLayer::PreviousDay => self.sync_daily_layer(ds).await,
            GridLayer::Yearly => self.sync_yearly_layer(ds).await,

            // Metadata/Calculated layers that don't have direct DB syncs
            _ => {
                debug!("Refresh requested for calculated layer: {:?}. Skipping direct DB sync.", layer);
                Ok(())
            }
        }
    }

    // --- LAYER SPECIFIC SYNCS (SurrealDB -> LayerManager) ---

    pub async fn sync_yearly_layer(
        &mut self, 
        ds: &dm::DataService
    ) -> Result<(), String> {
        let yearly_data = ds.yearly_views(
            &self.asset_id, 
            Some(1))
            .await
            .map_err(|e| e.to_string()
        )?;

        if let Some(v) = yearly_data.first() {
            let start_ts = Utc.from_local_datetime(
                &v.year_start
                .and_hms_opt(0, 0, 0)
                .unwrap())
                .single()
                .ok_or("Invalid year_start"
            )?;

            let end_ts = if v.high_ts > v.low_ts { v.high_ts } else { v.low_ts };

            let grid = AnchoredGrid::new(
                GridLayer::Yearly, 
                v.high, 
                v.low, 
                start_ts.timestamp_millis() as u64, 
                end_ts.timestamp_millis() as u64
            )?;
            
            self.yearly_layer.history = vec![v.clone()];
            self.yearly_layer.grids = vec![grid.clone()];
            self.active_grids.insert(GridLayer::Yearly, grid);
        }
        Ok(())
    }


    pub async fn sync_weekly_layer(
        &mut self,
        ds: &dm::DataService
    ) -> Result<(), String> {

        let mut weekly_data = ds.weekly_views(
            &self.asset_id,
            Some(30))
            .await.
            map_err(|e| e.to_string()
        )?;

        if weekly_data.is_empty() { return Ok(()); }

        // 1. Identify "This Week" start date to filter out live data
        let now = Utc::now();
        let days_since_monday = now.weekday().num_days_from_monday() as i64;
        let current_week_start = now.date_naive() - chrono::Duration::days(days_since_monday);

        // 2. Sort descending so [0] is the most recent
        weekly_data.sort_by(|a, b| b.week_start.cmp(&a.week_start));

        // 3. Filter to get only concluded weeks
        let concluded: Vec<_> = weekly_data.into_iter()
            .filter(|w| w.week_start < current_week_start)
            .collect();

        if concluded.is_empty() { 
            warn!("No concluded weeks found for {}", self.asset_id);
            return Ok(()); 
        }

        info!("🎯 PW identified as week starting: {} for {}", concluded[0].week_start, self.asset_id);

        // PW: The most recent concluded week
        let pw = &concluded[0];
        self.active_grids.insert(GridLayer::PW, 
            AnchoredGrid::new(
                GridLayer::PW, 
                pw.high, pw.low, 
                pw.high_ts.timestamp_millis() as u64,
                pw.low_ts.timestamp_millis() as u64
            )?
        );

        // PW1: Range of the last two concluded weeks
        if concluded.len() >= 2 {
            let slice = &concluded[0..2];
            let high = slice.iter().map(|w| w.high).fold(f64::MIN, f64::max);
            let low = slice.iter().map(|w| w.low).fold(f64::MAX, f64::min);
            
            let h_ts = slice.iter().find(|w| (w.high - high).abs() < f64::EPSILON).map(|w| w.high_ts).unwrap();
            let l_ts = slice.iter().find(|w| (w.low - low).abs() < f64::EPSILON).map(|w| w.low_ts).unwrap();

            self.active_grids.insert(GridLayer::PW1, 
                AnchoredGrid::new(
                    GridLayer::PW1, 
                    high, low,
                    h_ts.timestamp_millis() as u64, 
                    l_ts.timestamp_millis() as u64
                )?
            );
        }

        // PW2: Range of the last three concluded weeks
        if concluded.len() >= 3 {
            let slice = &concluded[0..3];
            let high = slice.iter().map(|w| w.high).fold(f64::MIN, f64::max);
            let low = slice.iter().map(|w| w.low).fold(f64::MAX, f64::min);
            
            let h_ts = slice.iter().find(|w| (w.high - high).abs() < f64::EPSILON).map(|w| w.high_ts).unwrap();
            let l_ts = slice.iter().find(|w| (w.low - low).abs() < f64::EPSILON).map(|w| w.low_ts).unwrap();

            self.active_grids.insert(GridLayer::PW2, 
                AnchoredGrid::new(
                    GridLayer::PW2,
                    high, low,
                    h_ts.timestamp_millis() as u64,
                    l_ts.timestamp_millis() as u64
                )?
            );
        }

        Ok(())
    }

    

    pub async fn sync_daily_layer(
        &mut self,
        ds: &dm::DataService
    ) -> Result<(), String> {

        let daily_data = ds.daily_views(
            &self.asset_id, None).await.map_err(|e| e.to_string()
        )?;

        if let Some(v) = daily_data.last() {
            if let (Some(h), Some(l)) = (v.prior_day_high, v.prior_day_low) {
                let grid = AnchoredGrid::new(
                    GridLayer::PreviousDay,
                    h, l,
                    v.start_ts.timestamp_millis() as u64,
                    v.end_ts.timestamp_millis() as u64
                )?;

                self.daily_layer.history = vec![v.clone()];
                self.daily_layer.grids = vec![grid.clone()];
                self.active_grids.insert(GridLayer::PreviousDay, grid);
            }
        }
        Ok(())
    }


    pub async fn sync_session_layer(
        &mut self,
        ds: &dm::DataService
    ) -> Result<(), String> {

        // Fetch 24 items to ensure we have enough context, but we only need the last few
        let (sessions, _) = ds.calculate_session_and_context(
            &self.asset_id,
            Some(24))
            .await
            .map_err(|e| e.to_string()
        )?;
        
        // history[last] = current active session
        // history[last-1] = completed session (PS1)
        self.session_layer.history = sessions.into_iter().rev().take(5).collect();
        self.session_layer.history.reverse(); 

        self.rebuild_session_grids();
        Ok(())
    }

    pub fn rebuild_session_grids(&mut self) {
        let len = self.session_layer.history.len();
        if len == 0 { return; }

        // 1. CS (Current Session) - The very latest session in history
        if let Some(current) = self.session_layer.history.last() {
            let grid = AnchoredGrid::new(
                GridLayer::CS, 
                current.high, current.low, 
                current.start_ts.timestamp_millis() as u64, 
                0).unwrap();

            self.active_grids.insert(GridLayer::CS, grid);
        }

        // 2. PS1 (Previous Session) - The one before the current
        if len >= 2 {
            let ps1_data = &self.session_layer.history[len - 2];
            let grid = AnchoredGrid::new(
                GridLayer::PS1, 
                ps1_data.high, ps1_data.low, 
                ps1_data.start_ts.timestamp_millis() as u64, 
                ps1_data.end_ts.timestamp_millis() as u64
            ).unwrap();

            self.active_grids.insert(GridLayer::PS1, grid);
        }

        // 3. PS2 (Range of last two sessions: history[len-3 .. len-1])
        // Note: We exclude the LIVE session (CS) from the PS2 range 
        // because PS2 is the range of the PAST two sessions.
        if len >= 3 {
            let slice = &self.session_layer.history[len - 3 .. len - 1]; // exclusive range
            let high = slice.iter().map(|s| s.high).fold(f64::MIN, f64::max);
            let low = slice.iter().map(|s| s.low).fold(f64::MAX, f64::min);
            
            let grid = AnchoredGrid::new(
                GridLayer::PS2, 
                high, low, 
                slice[0].start_ts.timestamp_millis() as u64, 
                slice[1].end_ts.timestamp_millis() as u64
            ).unwrap();

            self.active_grids.insert(GridLayer::PS2, grid);
        }else {
        // Log this so you know why it's missing from the dashboard
        debug!("PS2 skipped: history length is only {}", len);
        }
    }


    pub fn process_live_update(
        &mut self, 
        data: &MarketData_old, 
        ds: &dm::DataService
    ) {

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
            info!("🌅 Date Change: {} -> {}", self.last_daily_date, ny_date);
            self.last_daily_date = ny_date;
        // self.refresh_layer(GridLayer::PreviousDay, ds);
        }

        self.last_price = price;
    }

  
    fn handle_session_transition(
        &mut self, 
        name: String, 
        ts: DateTime<Utc>, 
        price: f64
    ) {
        let new_draft = dm::ClassifiedSession {
            asset_id: self.asset_id.clone(),
            session_name: name,
            start_ts: ts,
            end_ts: ts,
            open: price,
            high: price + 0.0001,
            low: price - 0.0001,
            close: price,
          //  state: dm::SessionState::Draft
            ..Default::default()
        };

        self.session_layer.history.push(new_draft);
        if self.session_layer.history.len() > 5 { self.session_layer.history.remove(0); }
        self.rebuild_session_grids();
        // SIGNAL: We just started a new session, we need the DB to verify the OLD one
        self.needs_reconcile = true;
    }

    pub fn sync_sessions_with_db(
        &mut self, 
        verified_sessions: Vec<dm::ClassifiedSession>
    ) {
        let mut modified = false;

        for verified in verified_sessions {
            if let Some(draft) = self.session_layer.history.iter_mut().find(|s| 
                s.session_name == verified.session_name && s.start_ts.date_naive() == verified.start_ts.date_naive()
            ) {
                if (draft.high - verified.high).abs() > f64::EPSILON || (draft.low - verified.low).abs() > f64::EPSILON {
                    draft.high = verified.high; draft.low = verified.low;
                    draft.open = verified.open; draft.close = verified.close;
                    modified = true;
                }
            }
        }
        if modified { self.rebuild_session_grids(); }
    }


    // --- UTILITIES & REPORTING ---

    pub fn get_grid(
        &self, 
        layer: GridLayer
    ) -> Option<&AnchoredGrid> {

        self.active_grids.get(&layer)
    }

    
    pub fn apply_simple_tick(
        &mut self, 
        price: f64
    ) {

        self.last_price = price;
        
        // 1. Update the "Current Week" bounds (used for real-time L3 context)
        if price > self.current_week_high { self.current_week_high = price; }
        if price < self.current_week_low { self.current_week_low = price; }

        // 2. Update the Dynamic Session Grid (CS)
        // This ensures the P0.5 and P0.6 levels move with the price
        self.session_layer.update_live(price, false);
        
        // 3. Keep the active_grids HashMap in sync for the Veto system
        if let Some(cs_grid) = self.session_layer.grids.first() {
            self.active_grids.insert(GridLayer::CS, cs_grid.clone());
        }
    }

    pub fn evaluate_veto_status(
        &self, 
        current_price: f64
    ) -> Option<VetoReason> {

        if let Some(daily) = self.daily_layer.grids.first() {
            if self.is_near_level(current_price, daily, PriceLevel::MidPoint) {
                return Some(VetoReason::DailyMidpointRejection);
            }
        }
        for (i, week_grid) in self.weekly_layer.grids.iter().enumerate() {
            if self.is_near_level(current_price, week_grid, PriceLevel::MidPoint) {
                return Some(VetoReason::HistoricalWeeklyMidpoint(i));
            }
        }
        None
    }


    fn is_near_level(&self, price: f64, grid: &AnchoredGrid, level: PriceLevel) -> bool {
        grid.get_price(level).map_or(false, |target| (price - target).abs() <= target * 0.0002)
    }


    pub fn render_layout_dashboard(&self) {
        println!("\n{}", "=".repeat(100));
        println!("📈 SPG LIVE LAYOUT: {:<10} | Price: {:<10.2}", self.asset_id, self.last_price);
        println!("{}", "-".repeat(100));
        
        println!("{:<15} | {:<12} | {:<10} | {:<10} | {:<10} | {:<10}", 
            "LAYER", "CONTEXT", "P1.00 (H)", "P0.50 (M)", "P0.00 (L)", "RANGE");
        println!("{}", "-".repeat(100));

        let display_order = vec![
            GridLayer::Yearly,
            GridLayer::PW2,
            GridLayer::PW1,
            GridLayer::PW,
            GridLayer::PreviousDay,
            GridLayer::PS2,
            GridLayer::PS1,
            GridLayer::CS,
        ];

        for layer in display_order {
            if let Some(grid) = self.active_grids.get(&layer) {
                let h = grid.get_price(PriceLevel::One).unwrap_or(grid.spg.high);
                let m = grid.get_price(PriceLevel::MidPoint).unwrap_or((h + grid.spg.low) / 2.0);
                let l = grid.get_price(PriceLevel::Zero).unwrap_or(grid.spg.low);
                let range = grid.spg.range_distance;
                
                // Derive Context (Session Name or Date)
                let dt = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(grid.start_timestamp as i64).unwrap_or_default();
                let context = match layer {
                    GridLayer::CS | GridLayer::PS1 | GridLayer::PS2 => {
                        // This matches the name from your session logic
                        get_trading_session(grid.start_timestamp as i64).map(|s| format!("{:?}", s)).unwrap_or("UNK".to_string())
                    },
                    _ => dt.format("%Y-%m-%d").to_string(),
                };

                let prefix = if layer == GridLayer::CS { ">> " } else { "   " };
                
                println!("{}{:<12} | {:<12} | {:<10.2} | {:<10.2} | {:<10.2} | {:<10.2}", 
                    prefix,
                    format!("{:?}", layer),
                    context,
                    h, 
                    m, 
                    l,
                    range
                );
            }
        }
        println!("{}", "=".repeat(100));
        println!(); 
    }

}
