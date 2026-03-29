use sqlx::{Postgres, Pool, Row};
use std::collections::HashMap;
use shared_models::market_classification::MarketType;
use crate::vortex::vortex::{VortexSnapshot, VortexPoint};



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
}