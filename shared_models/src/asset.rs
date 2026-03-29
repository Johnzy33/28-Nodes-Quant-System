
// use serde::{Deserialize, Serialize};

// /// Represents a financial instrument. Stored in the 'assets' table in TimescaleDB.
// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// pub struct Asset {
//     // The canonical ID used as the Foreign Key in 'MarketData'. 
//     // Format: "assets:US1000"
//     pub id: String, 
//     pub symbol: String,       // "US1000"
//     pub source: String,      // "BROKER_A"
//     pub name: Option<String>, 
//     pub asset_class: Option<String>, 
//     pub currency: Option<String>,
//     pub timezone: String,     // Crucial for correct time zone calculations
//     pub exchange: Option<String>,
//     pub active: bool,
//     pub first_listed_ts: Option<i64>, // Unix milliseconds
// }

// impl Asset {
//     // Constructor used when injecting/upserting the asset record
//     pub fn new(id: String, symbol: String, timezone: String) -> Self {
//         Self {
//             id,
//             symbol,
//             source: "DEFAULT".to_string(),
//             timezone,
//             name: None,
//             asset_class: None,
//             currency: None,
//             exchange: None,
//             active: true,
//             first_listed_ts: None,
//         }
//     }
// }