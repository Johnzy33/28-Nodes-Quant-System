

// use dashmap::DashMap;
// use std::sync::Arc;
// use futures_util::future::join_all;
// use tokio::task::JoinHandle;

// use crate::spg_tracker::SpgTracker;
// use shared_models::data_model as dm;


// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::price_grid::PriceLevel;
//     use shared_models::market_classification::MarketType;
//     use chrono::{Utc, TimeZone};

//     #[tokio::test]
//     async fn test_full_weekly_chain_logic() {
//         let asset = "assets:US30:FundedNext";
//         let mut tracker = SpgTracker::new(asset);

//         // --- MOCK DATA: 3 WEEKS OF HISTORY ---
        
//         // Week 3 (Oldest): Range 30,000 - 31,000
//         let w3 = dm::ClassifiedWeeklyView {
//             week_start: chrono::NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
//             asset_id: asset.to_string(),
//             high: 31000.0,
//             low: 30000.0,
//             close: 4540.0,
//             open: 4500.0,
//             bars: 390,
//             month_of_year: 3,
//             high_trading_date: chrono::NaiveDate::from_ymd_opt(2025, 12, 17).unwrap(),
//             low_trading_date: chrono::NaiveDate::from_ymd_opt(2025, 12, 15).unwrap(),
//             volume: 1_000_000,
//             high_session: "Midday".to_string(),
//             low_session: "Open".to_string(),
//             high_ts: Utc.with_ymd_and_hms(2025, 12, 17, 10, 0, 0).unwrap(),
//             low_ts: Utc.with_ymd_and_hms(2025, 12, 15, 15, 0, 0).unwrap(),
//             weekly_type: MarketType::Bullish,
            
//         };

//         // Week 2: Range 30,500 - 31,500
//         let w2 = dm::ClassifiedWeeklyView {
//             week_start: chrono::NaiveDate::from_ymd_opt(2025, 12, 8).unwrap(),
//             asset_id: asset.to_string(),
//             high: 31500.0,
//             low: 30500.0,
//             close: 4540.0,
//             open: 4500.0,
//             bars: 390,
//             month_of_year: 3,
//             high_trading_date: chrono::NaiveDate::from_ymd_opt(2025, 12, 17).unwrap(),
//             low_trading_date: chrono::NaiveDate::from_ymd_opt(2025, 12, 15).unwrap(),
//             volume: 1_000_000,
//             high_session: "Midday".to_string(),
//             low_session: "Open".to_string(),
//             high_ts: Utc.with_ymd_and_hms(2025, 12, 17, 10, 0, 0).unwrap(),
//             low_ts: Utc.with_ymd_and_hms(2025, 12, 15, 15, 0, 0).unwrap(),
//             weekly_type: MarketType::Bullish,
            
//         };

//         // Week 1 (Most Recent Prior Week): Range 31,000 - 32,000
//         let w1 = dm::ClassifiedWeeklyView {
//             week_start: chrono::NaiveDate::from_ymd_opt(2025, 12, 15).unwrap(),
//             asset_id: asset.to_string(),
//             high: 32000.0,
//             low: 31000.0,
//             close: 4540.0,
//             open: 4500.0,
//             bars: 390,
//             month_of_year: 3,
//             high_trading_date: chrono::NaiveDate::from_ymd_opt(2025, 12, 17).unwrap(),
//             low_trading_date: chrono::NaiveDate::from_ymd_opt(2025, 12, 15).unwrap(),
//             volume: 1_000_000,
//             high_session: "Midday".to_string(),
//             low_session: "Open".to_string(),
//             high_ts: Utc.with_ymd_and_hms(2025, 12, 17, 10, 0, 0).unwrap(),
//             low_ts: Utc.with_ymd_and_hms(2025, 12, 15, 15, 0, 0).unwrap(),
//             weekly_type: MarketType::Bullish,
            
//         };

//         // --- EXECUTION ---
//         // Simulating the loop in your initialize_tracker (Oldest to Newest)
//         tracker.add_weekly_anchor_to_chain(&w3).unwrap();
//         tracker.add_weekly_anchor_to_chain(&w2).unwrap();
//         tracker.add_weekly_anchor_to_chain(&w1).unwrap();

//         // --- VERIFICATION ---
        
//         // 1. Check Chain Length
//         assert_eq!(tracker.weekly_chain.len(), 3);

//         // 2. Check Week 1 (Index 0): Should be the most recent (31,000 - 32,000)
//         let week_1_mid = tracker.get_price_from_chain(0, PriceLevel::MidPoint).unwrap();
//         assert_eq!(week_1_mid, 31500.0);
//         println!("Week 1 Midpoint (Index 0): {}", week_1_mid);

//         // 3. Check Week 3 (Index 2): Should be the oldest (30,000 - 31,000)
//         let week_3_mid = tracker.get_price_from_chain(2, PriceLevel::MidPoint).unwrap();
//         assert_eq!(week_3_mid, 30500.0);
//         println!("Week 3 Midpoint (Index 2): {}", week_3_mid);

//         // 4. Test Confluence Logic
//         // Price is currently 31500. It's at the Midpoint of LAST week (Index 0).
//         let current_price = 31500.0;
//         let p0_mid = tracker.get_price_from_chain(0, PriceLevel::MidPoint).unwrap();
        
//         if (current_price - p0_mid).abs() < 1.0 {
//             println!("🎯 Price is reacting to Week 1 Midpoint Confluence!");
//         }

//         assert!((current_price - p0_mid).abs() < 1.0);
//     }
// }