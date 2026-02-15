
use std::collections::HashMap;
use shared_models::market_classification::MarketType; // Bullish, Bearish, etc.
use strum_macros::{Display, EnumString};
use sqlx::{Postgres, Pool, Row};



#[derive(Debug, Clone, PartialEq, Copy,Default)]
pub struct VortexPoint {
    pub p_6m: f64,
    pub p_1y: f64,
    pub p_all: f64,
    pub lift_6m: f64,
    pub lift_1y: f64,
    pub lift_all: f64,
    pub count_6m: f64,
    pub count_1y: f64,
    pub count_all: f64,
}

/// Holds the 7-state distribution for a specific metric type
pub type MetricMap = HashMap<MarketType, VortexPoint>;

#[derive(Debug, Clone, Default)]
pub struct VortexSnapshot {
    pub asset_id: String,
    pub anchor_time: chrono::DateTime<chrono::Utc>,
    pub context: SnapshotContext, // Added this field
    
    // The "True View" Layers
    pub session_to_session: MetricMap,
    pub session_to_daily: MetricMap,
    pub session_to_bar: MetricMap,
    pub bar_to_bar: MetricMap,
    pub bar_to_daily: MetricMap,
    
    // Structural Layers (Keyed by "High" or "Low" string)
    pub session_extreme: HashMap<String, VortexPoint>,
    pub bar_extreme: HashMap<String, VortexPoint>,
}

#[derive(Debug, Clone, Default)]
pub struct SnapshotContext {
    pub session_path: (String, String), // (path_p2, path_p1)
    pub bar_path: (String, String),     // (path_p2, path_p1)
    pub current_cs: String,
    pub current_cb: String,
}

#[derive(Debug, Clone)]
pub struct ConvictionResult {
    pub state: MarketType,
    pub score: f64,
    pub period_used: String, // "6m" or "all"
}

impl VortexSnapshot {

    pub async fn load_vortex_snapshot(
        
        pool: &Pool<Postgres>,
        asset_id: &str,
    ) -> Result<VortexSnapshot, sqlx::Error> {
        // 1. Initialize an empty VortexSnapshot
        let mut snapshot = VortexSnapshot {
            asset_id: asset_id.to_string(),
            anchor_time: chrono::Utc::now(),
            ..Default::default()
        };

        // 2. Execute the Master Query
        let rows = sqlx::query(
            r#"
            WITH s_ctx AS (
                SELECT ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name
                FROM session_context
                WHERE asset_id = $1 ORDER BY session_end_ts DESC LIMIT 1
            ),
            b_ctx AS (
                SELECT pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num
                FROM block_context
                WHERE asset_id = $1 ORDER BY session_end_ts DESC LIMIT 1
            )
            -- 1. Session to Session
            SELECT 
                'session_to_session' as m_type, s_ctx.cs_name::text as anchor_name, 
                m.cs_bias as state, m.p_6m, m.p_1y, m.p_all, 
                l.tcs_score_6m as l_6m, l.tcs_score_1y as l_1y, l.tcs_score_all as l_all, 
                m.count_6m, m.count_1y, m.count_all,
                s_ctx.ps2_name || ':' || s_ctx.ps2_bias as path_p2, 
                s_ctx.ps1_name || ':' || s_ctx.ps1_bias as path_p1
            FROM session_transition_2nd_order_ml m
            JOIN s_ctx ON m.asset_id = $1 AND m.ps2_name = s_ctx.ps2_name AND m.ps2_bias = s_ctx.ps2_bias AND m.ps1_name = s_ctx.ps1_name AND m.ps1_bias = s_ctx.ps1_bias AND m.cs_name = s_ctx.cs_name
            JOIN session_transition_lift_ml l ON l.asset_id = $1 AND l.cs_bias = m.cs_bias AND l.ps2_name = s_ctx.ps2_name AND l.ps2_bias = s_ctx.ps2_bias AND l.ps1_name = s_ctx.ps1_name AND l.ps1_bias = s_ctx.ps1_bias AND l.cs_name = s_ctx.cs_name

            UNION ALL
            -- 2. Session to Daily
            SELECT 
                'session_to_daily' as m_type, 'D1' as anchor_name, 
                m.day_type as state, m.p_6m, m.p_1y, m.p_all, 
                l.lift_6m as l_6m, l.lift_1y as l_1y, l.lift_all as l_all, 
                m.count_6m, m.count_1y, m.count_all,
                s_ctx.ps2_name || ':' || s_ctx.ps2_bias as path_p2, 
                s_ctx.ps1_name || ':' || s_ctx.ps1_bias as path_p1
            FROM session_to_daily_outcome_ml m
            JOIN s_ctx ON m.asset_id = $1 AND m.ps2_name = s_ctx.ps2_name AND m.ps2_bias = s_ctx.ps2_bias AND m.ps1_name = s_ctx.ps1_name AND m.ps1_bias = s_ctx.ps1_bias
            JOIN session_to_daily_lift_ml l ON l.asset_id = $1 AND l.day_type = m.day_type AND l.ps2_name = s_ctx.ps2_name AND l.ps2_bias = s_ctx.ps2_bias AND l.ps1_name = s_ctx.ps1_name AND l.ps1_bias = s_ctx.ps1_bias

            UNION ALL
            -- 3. Session to Bar
            SELECT 
                'session_to_bar' as m_type, b_ctx.cb_num::text as anchor_name, 
                m.cb_bias as state, m.p_6m, m.p_1y, m.p_all, 
                l.lift_6m as l_6m, l.lift_1y as l_1y, l.lift_all as l_all, 
                m.count_6m, m.count_1y, m.count_all,
                s_ctx.ps2_name || ':' || s_ctx.ps2_bias as path_p2, 
                s_ctx.ps1_name || ':' || s_ctx.ps1_bias as path_p1
            FROM session_to_bar_outcome_ml m
            JOIN s_ctx ON m.asset_id = $1 AND m.ps2_name = s_ctx.ps2_name AND m.ps2_bias = s_ctx.ps2_bias AND m.ps1_name = s_ctx.ps1_name AND m.ps1_bias = s_ctx.ps1_bias
            JOIN b_ctx ON m.cb_num = b_ctx.cb_num
            JOIN session_to_bar_lift_ml l ON l.asset_id = $1 AND l.cb_bias = m.cb_bias AND l.ps2_name = s_ctx.ps2_name AND l.ps2_bias = s_ctx.ps2_bias AND l.ps1_name = s_ctx.ps1_name AND l.ps1_bias = s_ctx.ps1_bias AND l.cb_num = b_ctx.cb_num

            UNION ALL
            -- 4. Bar to Bar
            SELECT 
                'bar_to_bar' as m_type, b_ctx.cb_num::text as anchor_name, 
                m.cb_bias as state, m.p_6m, m.p_1y, m.p_all, 
                l.stcs_score_6m as l_6m, l.stcs_score_1y as l_1y, l.stcs_score_all as l_all, 
                m.count_6m, m.count_1y, m.count_all,
                b_ctx.pb2_num::text || ':' || b_ctx.pb2_bias as path_p2, 
                b_ctx.pb1_num::text || ':' || b_ctx.pb1_bias as path_p1
            FROM bar_transition_2nd_order_ml m
            JOIN b_ctx ON m.asset_id = $1 AND m.pb2_num = b_ctx.pb2_num AND m.pb2_bias = b_ctx.pb2_bias AND m.pb1_num = b_ctx.pb1_num AND m.pb1_bias = b_ctx.pb1_bias AND m.cb_num = b_ctx.cb_num
            JOIN bar_transition_lift_ml l ON l.asset_id = $1 AND l.cb_bias = m.cb_bias AND l.pb2_num = b_ctx.pb2_num AND l.pb2_bias = b_ctx.pb2_bias AND l.pb1_num = b_ctx.pb1_num AND l.pb1_bias = b_ctx.pb1_bias AND l.cb_num = b_ctx.cb_num

            UNION ALL
            -- 5. Bar to Daily
            SELECT 
                'bar_to_daily' as m_type, 'D1' as anchor_name, 
                m.day_type as state, m.p_6m, m.p_1y, m.p_all, 
                l.lift_6m as l_6m, l.lift_1y as l_1y, l.lift_all as l_all, 
                m.count_6m, m.count_1y, m.count_all,
                b_ctx.pb2_num::text || ':' || b_ctx.pb2_bias as path_p2, 
                b_ctx.pb1_num::text || ':' || b_ctx.pb1_bias as path_p1
            FROM bar_to_daily_outcome_ml m
            JOIN b_ctx ON m.asset_id = $1 AND m.pb2_num = b_ctx.pb2_num AND m.pb2_bias = b_ctx.pb2_bias AND m.pb1_num = b_ctx.pb1_num AND m.pb1_bias = b_ctx.pb1_bias
            JOIN bar_to_daily_lift_ml l ON l.asset_id = $1 AND l.day_type = m.day_type AND l.pb2_num = b_ctx.pb2_num AND l.pb2_bias = b_ctx.pb2_bias AND l.pb1_num = b_ctx.pb1_num AND l.pb1_bias = b_ctx.pb1_bias

            UNION ALL
            -- 6. Session Extremes
            SELECT 
                'session_extreme' as m_type, m.extreme_session_name as anchor_name, 
                m.extreme_type as state, m.p_6m, m.p_1y, m.p_all, 
                l.lift_6m as l_6m, l.lift_1y as l_1y, l.lift_all as l_all, 
                m.count_6m, m.count_1y, m.count_all,
                s_ctx.ps2_name || ':' || s_ctx.ps2_bias as path_p2, 
                s_ctx.ps1_name || ':' || s_ctx.ps1_bias as path_p1
            FROM session_extreme_outcome_ml m
            JOIN s_ctx ON m.asset_id = $1 AND m.ps2_name = s_ctx.ps2_name AND m.ps2_bias = s_ctx.ps2_bias AND m.ps1_name = s_ctx.ps1_name AND m.ps1_bias = s_ctx.ps1_bias
            JOIN session_extreme_lift_ml l ON l.asset_id = $1 AND l.extreme_type = m.extreme_type AND l.extreme_session_name = m.extreme_session_name 
                AND l.ps2_name = s_ctx.ps2_name AND l.ps2_bias = s_ctx.ps2_bias AND l.ps1_name = s_ctx.ps1_name AND l.ps1_bias = s_ctx.ps1_bias

            UNION ALL
            -- 7. Bar Extremes
            SELECT 
                'bar_extreme' as m_type, m.extreme_cb_num::text as anchor_name, 
                m.extreme_type as state, m.p_6m, m.p_1y, m.p_all, 
                l.lift_6m as l_6m, l.lift_1y as l_1y, l.lift_all as l_all, 
                m.count_6m, m.count_1y, m.count_all,
                b_ctx.pb2_num::text || ':' || b_ctx.pb2_bias as path_p2, 
                b_ctx.pb1_num::text || ':' || b_ctx.pb1_bias as path_p1
            FROM bar_extreme_outcome_ml m
            JOIN b_ctx ON m.asset_id = $1 AND m.pb2_num = b_ctx.pb2_num AND m.pb2_bias = b_ctx.pb2_bias AND m.pb1_num = b_ctx.pb1_num AND m.pb1_bias = b_ctx.pb1_bias
            JOIN bar_extreme_lift_ml l ON l.asset_id = $1 AND l.extreme_type = m.extreme_type AND l.extreme_cb_num = m.extreme_cb_num 
                AND l.pb2_num = b_ctx.pb2_num AND l.pb2_bias = b_ctx.pb2_bias AND l.pb1_num = b_ctx.pb1_num AND l.pb1_bias = b_ctx.pb1_bias;
            "#
        )
        .bind(asset_id)
        .fetch_all(pool)
        .await?;

        // 3. Iterate through the Postgres rows
        for (i, row) in rows.iter().enumerate() {
            let m_type: String = row.get("m_type");
            let state_raw: String = row.get("state");
            let anchor_name: String = row.get("anchor_name");

            if i == 0{
                snapshot.context.session_path = (row.get("path_p2"), row.get("path_p1"));
                snapshot.context.current_cs = row.get("anchor_name");
            }

            if m_type == "bar_to_bar" && snapshot.context.bar_path.0.is_empty() {
            snapshot.context.bar_path = (row.get("path_p2"), row.get("path_p1"));
            snapshot.context.current_cb = row.get("anchor_name");
            }

            let point = VortexPoint {
                p_6m: row.get("p_6m"),
                p_1y: row.get("p_1y"),
                p_all: row.get("p_all"),
                lift_6m: row.get("l_6m"),
                lift_1y: row.get("l_1y"),
                lift_all: row.get("l_all"),
                count_6m: row.get("count_6m"),
                count_1y: row.get("count_1y"),
                count_all: row.get("count_all"),
            };

            // 4. Pattern matching with refined keying for Extremes
            match m_type.as_str() {
                "session_to_session" => {
                    if let Ok(st) = state_raw.parse::<MarketType>() {
                        snapshot.session_to_session.insert(st, point);
                    }
                }
                "session_to_daily" => {
                    if let Ok(st) = state_raw.parse::<MarketType>() {
                        snapshot.session_to_daily.insert(st, point);
                    }
                }
                "session_to_bar" => {
                    if let Ok(st) = state_raw.parse::<MarketType>() {
                        snapshot.session_to_bar.insert(st, point);
                    }
                }
                "bar_to_bar" => {
                    if let Ok(st) = state_raw.parse::<MarketType>() {
                        snapshot.bar_to_bar.insert(st, point);
                    }
                }
                "bar_to_daily" => {
                    if let Ok(st) = state_raw.parse::<MarketType>() {
                        snapshot.bar_to_daily.insert(st, point);
                    }
                }
                "session_extreme" => {
                    // Formulate key as "High:LN" or "Low:NYAM"
                    let key = format!("{}:{}", state_raw, anchor_name);
                    snapshot.session_extreme.insert(key, point);
                }
                "bar_extreme" => {
                    // Formulate key as "High:3" or "Low:1"
                    let key = format!("{}:{}", state_raw, anchor_name);
                    snapshot.bar_extreme.insert(key, point);
                }
                _ => {}
            }
        }

        println!("--- Vortex Snapshot: {} ---", &snapshot.asset_id);
            
            // Print the Session Path
            let (s_p2, s_p1) = &snapshot.context.session_path;
            println!(
                "Session Path:  [{}] -> [{}]  => Current: {}", 
                s_p2, s_p1, &snapshot.context.current_cs
            );

            // Print the Bar Path
            let (b_p2, b_p1) = &snapshot.context.bar_path;
            println!(
                "Bar Path:      [{}] -> [{}]  => Current Bar: {}", 
                b_p2, b_p1, &snapshot.context.current_cb
            );
            
        println!("-------------------------------------------");
        

        
        

        Ok(snapshot)
    }

    pub fn print_summary(&self) {
        println!("--- Vortex Snapshot: {} ---", self.asset_id);
        
        // Print the Session Path
        let (s_p2, s_p1) = &self.context.session_path;
        println!(
            "Session Path:  [{}] -> [{}]  => Current: {}", 
            s_p2, s_p1, self.context.current_cs
        );

        // Print the Bar Path
        let (b_p2, b_p1) = &self.context.bar_path;
        println!(
            "Bar Path:      [{}] -> [{}]  => Current Bar: {}", 
            b_p2, b_p1, self.context.current_cb
        );
        
        println!("-------------------------------------------");
    }

/// Normalizes a raw lift value into a 0.0 - 1.0 "Edge Multiplier"
    pub fn normalize_lift(raw_lift: f64) -> f64 {
        if raw_lift <= 1.0 { return 0.0; } 
        
        let max_expected: f64 = 5.0; 
        let normalized = raw_lift.ln() / max_expected.ln();
        
        normalized.min(1.0).max(0.0)
    }

    pub fn calculate_initial_conviction(&self, map: &MetricMap) -> Vec<ConvictionResult> {
        let mut results: Vec<ConvictionResult> = Vec::new();

        for (state, point) in map {
            // 1. Regime Consistency & Tax Logic
            let lift_6m = point.lift_6m;
            let lift_all = point.lift_all;

            // Consistency Tax: If long-term history is negative (<1.0) 
            // but short-term is positive, we don't fully trust it.
            let consistency_tax = if lift_all < 1.0 && lift_6m > 1.0 { 0.8 } else { 1.0 };

            // Regime Consistency: How much is the current regime 
            // outperforming or underperforming history?
            let regime_consistency = if lift_all > 0.0 { lift_6m / lift_all } else { 1.0 };

            // 2. Select Base Stats (using your Floor of 5 logic)
            let (p, raw_lift, period) = if point.count_6m >= 5.0 {
                (point.p_6m, lift_6m, "6m")
            } else {
                (point.p_all, lift_all, "all")
            };

            // 3. Normalize & Weight
            let normalized_lift = Self::normalize_lift(raw_lift);
            
            // Base Score: (P * 0.4) + (Lift * 0.6)
            let mut score = (p * 0.4 + normalized_lift * 0.6) * 100.0;

            // 4. Apply the Regime Modifiers
            if regime_consistency > 1.0 {
                score *= 1.1; // Boost for trend alignment
            }
            score *= consistency_tax; // Apply tax for historical divergence

            results.push(ConvictionResult {
                state: *state,
                score: score.min(100.0), // Cap final score
                period_used: period.to_string(),
            });
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}