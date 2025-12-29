
use anyhow::Result;
// use shared_models::models::{
//     SessionContextData, DailyContextData,EightContextData
// };
use crate::metric::fetchable::FetchableML;
use crate::metrics_service::MetricsService;
use shared_models::models as sm;




// impl MetricsService {
//     pub async fn fetch_session_contexts_for_ml(
//     &self,
//     asset_id: &str,
//     ) -> Result<Vec<SessionContextData>> {
    
//     // We only filter by asset_id and ensure the outcome is valid
//     let query = r#"
//         SELECT *    
//         FROM session_context
//         WHERE asset_id = $1 AND cs_bias IS NOT NULL AND cs_bias != 'Other'
//         ORDER BY trading_date ASC, session_end_ts ASC
//     "#;

//     sqlx::query_as(query)
//         .bind(asset_id)
//         .fetch_all(&self.pool)
//         .await
//         .map_err(|e| anyhow::anyhow!("Failed to fetch all Session Contexts for ML: {}", e))
//     }

//     pub async fn fetch_bar_contexts_for_ml(
//     &self,
//     asset_id: &str,
//     ) -> Result<Vec<EightContextData>> {
    
//     // We only filter by asset_id and ensure the outcome is valid
//     let query = r#"
//         SELECT *    
//         FROM block_context
//         WHERE asset_id = $1 AND cb_bias IS NOT NULL AND cb_bias != 'Other'
//         ORDER BY trading_date ASC, session_end_ts ASC
//     "#;

//     sqlx::query_as(query)
//         .bind(asset_id)
//         .fetch_all(&self.pool)
//         .await
//         .map_err(|e| anyhow::anyhow!("Failed to fetch all Bar Contexts for ML: {}", e))
//     }

//     pub async fn fetch_day_contexts_for_ml(
//         &self,
//         asset_id: &str,
//     ) -> Result<Vec<DailyContextData>> {
    
//         let query = r#"
//             SELECT 
//                 trading_date, asset_id, day_type, high_session, low_session, high_bar, low_bar      
//             FROM daily_views
//             WHERE asset_id = $1 AND day_type IS NOT NULL AND day_type != 'Other'
//             ORDER BY trading_date ASC
//         "#;
    
//         sqlx::query_as(query)
//             .bind(asset_id)
//             .fetch_all(&self.pool)
//             .await
//             .map_err(|e| anyhow::anyhow!("Failed to fetch all Day Contexts for ML: {}", e))
//         }
    
// }

impl MetricsService {
    /// Generic internal fetcher
    async fn fetch_data<T>(&self, asset_id: &str) -> Result<Vec<T>> 
    where T: FetchableML {
        sqlx::query_as::<_, T>(T::query())
            .bind(asset_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database fetch error: {}", e))
    }

    // Public API - Clean and Scannable
    pub async fn fetch_session_contexts(&self, asset_id: &str) -> Result<Vec<sm::SessionContextData>> {
        self.fetch_data(asset_id).await
    }

    pub async fn fetch_bar_contexts(&self, asset_id: &str) -> Result<Vec<sm::EightContextData>> {
        self.fetch_data(asset_id).await
    }

    pub async fn fetch_day_contexts(&self, asset_id: &str) -> Result<Vec<sm::DailyContextData>> {
        self.fetch_data(asset_id).await
    }
}