use crate::vortex::anchor::VortexAnchor;
use shared_models::market_classification::MarketType;
use shared_models::AssetMetadata;
use chrono::{DateTime,Utc};

#[derive(Debug, Clone)]
pub struct LiveVortexMonitor {
    pub session_match: f64,   // Alignment with s2s anchor
    pub bar_match: f64,       // Alignment with b2b anchor
    pub daily_match: f64,     // Alignment with s2d anchor
    pub current_total_score: f64,
    pub decay_k: f64,
    pub last_high_ts: DateTime<Utc>,
    pub last_low_ts: DateTime<Utc>,
    pub last_update: chrono::DateTime<chrono::Utc>,
    pub is_failed: bool,
}

// impl LiveVortexMonitor {

//     pub fn new(anchor: &VortexAnchor) -> Self {
//             // Set decay based on the specific session intensity
//             let k_session = match anchor.session_name.as_str() {
//                 "NYAM" | "NYPM" => 3.0, // High volatility: Move fast or the edge dies
//                 "LN"            => 2.0, // Active but steady
//                 "NYL"           => 1.5, // Lunch drift
//                 "AS"            => 0.8, // Slow accumulation
//                 _ => 1.0,
//             };

//             // If we are in Bar 3, the Bar decay matches the Session intensity.
//             // If we are in Bar 1 (Asia/Night), it's much slower.
//             let k_bar = match anchor.bar_number.as_str() {
//                 "3" => k_session, 
//                 "2" => 1.2,
//                 "1" => 0.5,
//                 _ => 1.0,
//             };

//             Self {
//                 session_match: 1.0,
//                 bar_match: 1.0,
//                 daily_match: 1.0,
//                 current_total_score: anchor.total_conviction_score,
//                 decay_k: k_session, // We use the Session K as the master heartbeat
//                 last_update: chrono::Utc::now(),
//             }
//     }

//     pub fn calculate_geospatial_k(anchor: &VortexAnchor, meta: &AssetMetadata) -> f64 {
//         let mut k = 1.0;

//         // 1. Base Score Intensity (Phase 1 Conviction)
//         let top_score = anchor.s2s_convictions.first().map(|c| c.score).unwrap_or(0.0);
//         k = match top_score {
//             s if s > 70.0 => 3.0, // High conviction = Low patience
//             s if s > 50.0 => 1.8,
//             _ => 0.8,             // Low conviction = High patience (chop)
//         };

//         // 2. The "Home Session" Multiplier
//         // We detect if the current session name matches the asset's primary market hours
//         let is_home_session = match (meta.timezone.as_str(), anchor.session_name.as_str()) {
//             ("Asia/Tokyo", "AS") => true,
//             ("Europe/London", "LN") => true,
//             ("America/New_York", session) if session.contains("NY") => true,
//             _ => false,
//         };

//         // 3. Asset Class Sensitivity
//         // Indices are highly sensitive to their home session. 
//         // Commodities (Gold/Silver) are sensitive to NY but trade globally.
//         if is_home_session {
//             match meta.asset_class.as_str() {
//                 "Index_Future" => k *= 1.75, // Indices move violently in home hours
//                 "Commodity Future" => k *= 1.25, // Gold moves in NY, but less "session-locked"
//                 _ => k *= 1.1,
//             }
//         }

//         k
//     }

//     //     pub fn process_tick(&mut self, anchor: &VortexAnchor, live_bias: MarketType) {
//     //     let now = chrono::Utc::now();
//     //     let elapsed_hours = (now - self.last_update).num_milliseconds() as f64 / 3600000.0;

//     //     // 1. Layered Match Factors
//     //     // How does live action match the Session Anchor?
//     //     self.session_match = self.calculate_match(live_bias, anchor.primary_s2s);
        
//     //     // How does live action match the Bar Anchor? (Crucial for timing)
//     //     self.bar_match = self.calculate_match(live_bias, anchor.primary_b2b);

//     //     // 2. Global Decay
//     //     let decay = (-self.decay_k * elapsed_hours).exp();

//     //     // 3. Composite Scoring
//     //     // We weight the Session more for trend, and the Bar more for immediate entry
//     //     let weighted_match = (self.session_match * 0.6) + (self.bar_match * 0.4);
        
//     //     // The Final "In-Flight" Score
//     //     self.current_total_score = (anchor.total_conviction_score * weighted_match) * decay;

//     //     self.last_update = now;
//     // }

//     // pub fn process_tick(&mut self, anchor: &VortexAnchor, live_bias: MarketType) {
//     //     let now = chrono::Utc::now();
//     //     // Convert to hours for the exponential decay formula
//     //     let elapsed_hours = (now - self.last_update).num_milliseconds() as f64 / 3600000.0;

//     //     // 1. Calculate Match Factors for all 3 layers
//     //     self.session_match = self.calculate_match(live_bias, anchor.primary_s2s);
//     //     self.bar_match = self.calculate_match(live_bias, anchor.primary_b2b);
//     //     self.daily_match = self.calculate_match(live_bias, anchor.primary_s2d);

//     //     // 2. Conflict Detection (Phase 3)
//     //     // If Session Anchor is Bullish but Bar Anchor is Bearish, apply "Structural Friction"
//     //     let mut friction = 1.0;
//     //     if anchor.primary_s2s != anchor.primary_b2b {
//     //         friction = 0.85; // 15% penalty for layers being out of sync
//     //     }

//     //     // 3. The Decay Formula: Score = Score * e^(-k * t)
//     //     let decay = (-self.decay_k * elapsed_hours).exp();

//     //     // 4. Weighted Confluence Score
//     //     // Session (40%) + Bar (40%) + Daily (20%)
//     //     let confluence = (self.session_match * 0.4) + (self.bar_match * 0.4) + (self.daily_match * 0.2);
        
//     //     // Final Live Conviction
//     //     self.current_total_score = (anchor.total_conviction_score * confluence * friction) * decay;

//     //     self.last_update = now;
//     // }

//     pub fn process_tick(&mut self, anchor: &VortexAnchor, live_bias: MarketType) {
//         let now = chrono::Utc::now();
//         // Convert to hours for the exponential decay formula
//         let elapsed_hours = (now - self.last_update).num_milliseconds() as f64 / 3600000.0;

//         // 1. Calculate Match Factors for all 3 layers
//         self.session_match = self.calculate_match(live_bias, anchor.primary_s2s);
//         self.bar_match = self.calculate_match(live_bias, anchor.primary_b2b);
//         self.daily_match = self.calculate_match(live_bias, anchor.primary_s2d);

//         // 2. Conflict Detection (Phase 3)
//         // If Session Anchor is Bullish but Bar Anchor is Bearish, apply "Structural Friction"
//         let mut friction = 1.0;
//         if anchor.primary_s2s != anchor.primary_b2b {
//             friction = 0.85; // 15% penalty for layers being out of sync
//         }

//         // 3. The Decay Formula: Score = Score * e^(-k * t)
//         let decay = (-self.decay_k * elapsed_hours).exp();

//         // 4. Weighted Confluence Score
//         // Session (40%) + Bar (40%) + Daily (20%)
//         let confluence = (self.session_match * 0.4) + (self.bar_match * 0.4) + (self.daily_match * 0.2);
        
//         // Final Live Conviction
//         self.current_total_score = (anchor.total_conviction_score * confluence * friction) * decay;

//         self.last_update = now;
//     }

//     pub fn tick(&mut self, anchor: &VortexAnchor, live_bias: MarketType) {
//         let now = chrono::Utc::now();
//         let elapsed_hours = (now - self.last_update).num_milliseconds() as f64 / 3600000.0;

//         // Calculate current match across layers
//         self.session_match = self.calculate_match(live_bias, anchor.primary_s2s);
//         self.bar_match = self.calculate_match(live_bias, anchor.primary_b2b);

//         // Apply Exponential Decay: Score = Base * e^(-k*t)
//         let decay = (-self.decay_k * elapsed_hours).exp();
        
//         // Final Score weighted by Confluence
//         let confluence = (self.session_match * 0.5) + (self.bar_match * 0.5);
//         self.current_total_score = (anchor.total_conviction_score * confluence) * decay;

//         self.last_update = now;
//     }

//     fn calculate_match(&self, live: MarketType, anchor_target: MarketType) -> f64 {
//         if live == anchor_target { 1.25 }
//         else if live == anchor_target.inverse() { 0.5 }
//         else { 1.0 }
//     }
// }


impl LiveVortexMonitor {
    /// Initialize the monitor using both the Anchor (Statistics) and Metadata (Geography)
    pub fn new(anchor: &VortexAnchor, meta: &AssetMetadata) -> Self {
        let k = Self::calculate_geospatial_k(anchor, meta);

        Self {
            current_total_score: anchor.total_conviction_score,
            decay_k: k,
            session_match: 1.0,
            bar_match: 1.0,
            daily_match: 1.0,
            last_update: Utc::now(),
            last_high_ts:Utc::now(),
            last_low_ts:Utc::now(),
            is_failed: false,
        }
    }

    /// Determines how fast the probability should bleed based on Asset Class and Home Session
    fn calculate_geospatial_k(anchor: &VortexAnchor, meta: &AssetMetadata) -> f64 {
        
        // 1. Base Intensity from Phase 1 Conviction
        // High conviction setups have a shorter "window of opportunity"
        let mut k = if anchor.total_conviction_score > 70.0 {
            3.0 
        } else if anchor.total_conviction_score > 50.0 {
            1.8
        } else {
            0.8 // Low conviction allows for more "choppy" time
        };

        // 2. Home Session Detection
        let is_home = match (meta.timezone.as_str(), anchor.session_name.as_str()) {
            ("Asia/Tokyo", "AS") => true,
            ("Europe/London", "LN") => true,
            ("America/New_York", s) if s.contains("NY") => true,
            _ => false,
        };

        // 3. Asset Class Sharpening
        if is_home {
            match meta.asset_class.as_deref() {
                Some("Index_Future") => k *= 1.75, // Indices must move fast in home hours
                Some("Commodity Future") => k *= 1.25,
                _ => k *= 1.1,
            }
        }

        k
    }

    /// The Main Live Loop Function: Call this on every Kafka tick
    // pub fn process_tick(&mut self, anchor: &VortexAnchor, live_bias: MarketType) {

    //     let now = Utc::now();
    //     let elapsed_hours = (now - self.last_update).num_milliseconds() as f64 / 3_600_000.0;

    //     // 1. Calculate Match Factors across all layers
    //     self.session_match = self.calculate_match(live_bias, anchor.primary_s2s);
    //     self.bar_match = self.calculate_match(live_bias, anchor.primary_b2b);
    //     self.daily_match = self.calculate_match(live_bias, anchor.primary_s2d);

    //     // 2. Conflict Friction (Phase 3)
    //     // If Session and Bar anchors disagree, the setup is internally "noisy"
    //     let friction = if anchor.primary_s2s != anchor.primary_b2b { 0.85 } else { 1.0 };

    //     // 3. Composite Confluence
    //     // We weight Session and Bar heavily, Daily provides a background nudge
    //     let confluence = (self.session_match * 0.45) + (self.bar_match * 0.45) + (self.daily_match * 0.1);

    //     // 4. Exponential Decay
    //     let decay = (-self.decay_k * elapsed_hours).exp();

    //     // 5. Final Live Score Calculation
    //     self.current_total_score = (anchor.total_conviction_score * confluence * friction) * decay;

    //     // 6. Failure Trigger (Phase 4)
    //     if self.current_total_score < 20.0 {
    //         self.is_failed = true;
    //     }

    //     self.last_update = now;
    // }

    pub fn process_tick(
        &mut self, 
        anchor: &VortexAnchor, 
        live_bias: MarketType,
        current_high_ts: DateTime<Utc>,
        current_low_ts: DateTime<Utc>
    ) {
        let now = Utc::now();
        let elapsed_hours = (now - self.last_update).num_milliseconds() as f64 / 3_600_000.0;

        // 1. Update Extremes
        self.last_high_ts = current_high_ts;
        self.last_low_ts = current_low_ts;

        // 1. Dynamic Stagnation Threshold
        // High k (3.5) -> ~15 min threshold
        // Low k (0.8)  -> ~60 min threshold
        let wait_threshold_mins = if self.decay_k >= 2.5 {
            15 // Expansion expected now (Home Session)
        } else if self.decay_k >= 1.5 {
            30 // Steady trend expected
        } else {
            60 // Off-hours drift allowed
        };

        // 2. Intra-Session Stagnation Penalty (Step 5)
        let mut stagnation_penalty = 1.0;
        let minutes_since_high = (now - self.last_high_ts).num_minutes();
        let minutes_since_low = (now - self.last_low_ts).num_minutes();

        if anchor.primary_s2s == MarketType::Bullish && minutes_since_high > wait_threshold_mins {
            // Expected Bullish expansion, but no new high in 15 mins
            stagnation_penalty = 0.85; 
        } else if anchor.primary_s2s == MarketType::Bearish && minutes_since_low > wait_threshold_mins {
            // Expected Bearish expansion, but no new low in 15 mins
            stagnation_penalty = 0.85;
        }

        // 3. Match Factors & Friction (Phase 3)
        self.session_match = self.calculate_match(live_bias, anchor.primary_s2s);
        self.bar_match = self.calculate_match(live_bias, anchor.primary_b2b);
        let friction = if anchor.primary_s2s != anchor.primary_b2b { 0.85 } else { 1.0 };

        // 4. Combined Decay (Standard + Stagnation)
        // If stagnating, we effectively increase the decay speed
        let active_k = if stagnation_penalty < 1.0 { self.decay_k * 1.5 } else { self.decay_k };
        let decay = (-active_k * elapsed_hours).exp();

        // 5. Final Score
        let confluence = (self.session_match * 0.45) + (self.bar_match * 0.45) + (self.daily_match * 0.1);
        self.current_total_score = (anchor.total_conviction_score * confluence * friction * stagnation_penalty) * decay;

        if self.current_total_score < 20.0 {
            self.is_failed = true;
        }

        self.last_update = now;
    }

    fn calculate_match(&self, live: MarketType, target: MarketType) -> f64 {
        if live == target {
            1.25 // Confirming
        } else if live == target.inverse() {
            0.50 // Diverging
        } else {
            1.00 // Neutral
        }
    }
}