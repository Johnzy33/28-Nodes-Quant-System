
use crate::vortex::vortex::{ConvictionResult,VortexSnapshot};
use shared_models::market_classification::MarketType;

#[derive(Debug, Clone, Default)]
pub struct VortexAnchor {
    pub asset_id: String,
    pub session_name: String,
    pub bar_number: String,
    
    // Primary Biases for all 5 Transition Layers
    pub primary_s2s: MarketType, // Session -> Session
    pub primary_s2b: MarketType, // Session -> Bar (CRITICAL)
    pub primary_s2d: MarketType, // Session -> Daily
    pub primary_b2b: MarketType, // Bar -> Bar
    pub primary_b2d: MarketType, // Bar -> Daily

    // Ranked Conviction Lists
    pub s2s_convictions: Vec<ConvictionResult>,
    pub s2b_convictions: Vec<ConvictionResult>,
    pub s2d_convictions: Vec<ConvictionResult>,
    pub b2b_convictions: Vec<ConvictionResult>,
    pub b2d_convictions: Vec<ConvictionResult>,
    
    // Structural Convictions (Extremes)
    pub s_extreme_convictions: Vec<ConvictionResult>, // Logic uses raw string keys
    pub b_extreme_convictions: Vec<ConvictionResult>,

    pub total_conviction_score: f64,
    pub anchored_at: chrono::DateTime<chrono::Utc>,
}

impl VortexAnchor {
    pub fn from_snapshot(snapshot: &VortexSnapshot) -> Self {
        // 1. Calculate convictions for EVERY layer
        let s2s = snapshot.calculate_initial_conviction(&snapshot.session_to_session);
        let s2b = snapshot.calculate_initial_conviction(&snapshot.session_to_bar);
        let s2d = snapshot.calculate_initial_conviction(&snapshot.session_to_daily);
        let b2b = snapshot.calculate_initial_conviction(&snapshot.bar_to_bar);
        let b2d = snapshot.calculate_initial_conviction(&snapshot.bar_to_daily);

        // 2. Extract Primaries
        let p_s2s = s2s.first().map(|c| c.state).unwrap_or(MarketType::PureIndecision);
        let p_s2b = s2b.first().map(|c| c.state).unwrap_or(MarketType::PureIndecision);
        let p_b2b = b2b.first().map(|c| c.state).unwrap_or(MarketType::PureIndecision);

        // 3. Confluence Scoring (The "Agreement" Multiplier)
        // Start with the main Session-to-Session score as base
        let mut final_score = s2s.first().map(|c| c.score).unwrap_or(0.0);
        
        // BOOSTER: If Session-to-Bar agrees with Session-to-Session
        if p_s2s == p_s2b {
            final_score += 10.0; 
        }
        
        // BOOSTER: If Bar-to-Bar agrees with Session-to-Bar (The "Golden Thread")
        if p_b2b == p_s2b {
            final_score += 15.0;
        }

        VortexAnchor {
            asset_id: snapshot.asset_id.clone(),
            session_name: snapshot.context.current_cs.clone(),
            bar_number: snapshot.context.current_cb.clone(),
            
            primary_s2s: p_s2s,
            primary_s2b: p_s2b,
            primary_s2d: s2d.first().map(|c| c.state).unwrap_or(MarketType::PureIndecision),
            primary_b2b: p_b2b,
            primary_b2d: b2d.first().map(|c| c.state).unwrap_or(MarketType::PureIndecision),

            s2s_convictions: s2s,
            s2b_convictions: s2b,
            s2d_convictions: s2d,
            b2b_convictions: b2b,
            b2d_convictions: b2d,
            
            // For extremes, we pass the internal string-based maps
            // You might need a slightly different helper for string-based maps
            s_extreme_convictions: Vec::new(), 
            b_extreme_convictions: Vec::new(),

            total_conviction_score: final_score.min(100.0),
            anchored_at: chrono::Utc::now(),
        }
    }
}