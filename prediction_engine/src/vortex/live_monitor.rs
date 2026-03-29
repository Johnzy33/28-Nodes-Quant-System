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