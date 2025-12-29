
use anyhow::Result;
use shared_models::models::{
    SessionContextData, DailyContextData,EightContextData
};
use crate::metrics_service::MetricsService;

// pub fn map_daily_bias(
//     day_type: Option<String>, 
//     consolidation_subtype: Option<String>
// ) -> Option<String> {
    
//     // Equivalent to: WHERE dv.day_type IS NOT NULL;
//     let dt = match day_type {
//         Some(s) => s,
//         None => return None,
//     };
    
//     match dt.as_str() {
//         // WHEN dv.day_type IN ('Bullish', 'Bearish') THEN dv.day_type
//         "Bullish" => Some("Bullish".to_string()),
//         "Bearish" => Some("Bearish".to_string()),
        
//         "Consolidation" => {
//             // WHEN dv.day_type = 'Consolidation' THEN COALESCE(dv.consolidation_subtype, 'Consolidation-Other')
//             match consolidation_subtype {
//                 Some(subtype) => Some(subtype), // Assuming subtype is already a 7-level bias (e.g., 'BullishReversal')
//                 None => Some("Consolidation-Other".to_string()), // If subtype is NULL, map to the generic consolidation label
//             }
//         }
        
//         // ELSE 'Other'
//         _ => Some("Other".to_string()),
//     }
// }

impl MetricsService {
    pub async fn fetch_session_contexts_for_ml(
    &self,
    asset_id: &str,
    ) -> Result<Vec<SessionContextData>> {
    
    // We only filter by asset_id and ensure the outcome is valid
    let query = r#"
        SELECT *    
        FROM session_context
        WHERE asset_id = $1 AND cs_bias IS NOT NULL AND cs_bias != 'Other'
        ORDER BY trading_date ASC, session_end_ts ASC
    "#;

    sqlx::query_as(query)
        .bind(asset_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch all Session Contexts for ML: {}", e))
    }

    pub async fn fetch_bar_contexts_for_ml(
    &self,
    asset_id: &str,
    ) -> Result<Vec<EightContextData>> {
    
    // We only filter by asset_id and ensure the outcome is valid
    let query = r#"
        SELECT *    
        FROM block_context
        WHERE asset_id = $1 AND cb_bias IS NOT NULL AND cb_bias != 'Other'
        ORDER BY trading_date ASC, session_end_ts ASC
    "#;

    sqlx::query_as(query)
        .bind(asset_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch all Bar Contexts for ML: {}", e))
    }

    pub async fn fetch_day_contexts_for_ml(
        &self,
        asset_id: &str,
    ) -> Result<Vec<DailyContextData>> {
    
        let query = r#"
            SELECT 
                trading_date, asset_id, day_type, high_session, low_session, high_bar, low_bar      
            FROM daily_views
            WHERE asset_id = $1 AND day_type IS NOT NULL AND day_type != 'Other'
            ORDER BY trading_date ASC
        "#;
    
        sqlx::query_as(query)
            .bind(asset_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch all Day Contexts for ML: {}", e))
        }
    
}