use anyhow::{Context};
// use futures::TryStreamExt;
// use futures::future::ok;

// use sqlx::PgPool; 
// use std::{env};
// use surrealdb::types::{RecordId};
use std::sync::Arc;
// use surrealdb::engine::local::{Db, Mem, SurrealKv};
use surrealdb::opt::auth::Root;
use surrealdb::{Surreal, Connection, Error};
use surrealdb::engine::remote::ws::{Ws, Client};
use surrealdb_types::{Datetime, SurrealValue};

// use surrealdb::engine::remote::ws::Client;
use shared_models::{db_models::*};
// use std::error::Error;
// use log::{ error};
// use std::fmt::format;
// use std::fs;
// use std::path::Path;
// use tokio::task;
// use shared_models::models::AssetSeed;
use log::{info,warn,error};
// use tracing;

//use crate::schema_setup; 

use std::collections::HashMap;
use std::sync::OnceLock;

use std::fs;

// pub type Pool = PgPool;


static METADATA_CACHE: OnceLock<HashMap<String, StaticMetadata>> = OnceLock::new();

/// Initializes the connection pool and applies the database schema.
// pub async fn setup_database_pool() -> Result<Pool> {
    
//     let db_url = env::var("DATABASE_URL")
//         .context("DATABASE_URL environment variable must be set (e.g., postgresql://user:pass@host:port/db)")?;

//     info!("Initializing Database Pool ...");

//     //BUILD THE SQLX POOL
//     let pool = PgPoolOptions::new()
//         .max_connections(16) // Use max_connections for sqlx
//         .connect(&db_url)
//         .await
//         .context("Failed to build PostgreSQL connection pool using sqlx")?;

//     //register_all_assets(&pool).await?;

//     info!("Database Pool Ready....");

//     Ok(pool)
// }


// use tokio::process::Command;
// use std::net::TcpStream;
// use std::time::Duration;





pub async fn setup_database() -> AppResult<Arc<AppDatabases>> {
    info!("Connecting to SurrealDB Background Services...");

    // 1. Define Addresses 
    let mem_addr = "127.0.0.1:8000";   // Systemd: Memory Node
    let disk_addr = "127.0.0.1:8001";  // Systemd: Disk Node (SurrealKV)

    // 2. Initialize WebSocket Connections
    let mem_db = Surreal::new::<Ws>(mem_addr).await?;
    let disk_db = Surreal::new::<Ws>(disk_addr).await?;
    
    // We'll treat the Disk DB as the "Mirror" target if needed, 
    // or keep it separate if you decide to add a 3rd instance later.
    let mirror_db: Option<Surreal<Client>> = None; 

    // 3. Prepare Credentials (from your saved info: root/root)
     let credentials = Root {
        username: "root".to_string(),
        password: "root".to_string(),
    };

    // 4. Authenticate & Scope Connections
    // We group them to handle the "Wait for Service" logic uniformly
    let targets = [
        ("Memory-Node", &mem_db, "28_node_cache"),
        ("Disk-Node",   &disk_db, "28_node"),
    ];

    for (name, db, db_name) in targets {
        let mut authenticated = false;
        for i in 1..=10 { // Increased to 10 for systemd startup lag
            if db.signin(credentials.clone()).await.is_ok() {
                db.use_ns("trading_system").use_db(db_name).await?;
                authenticated = true;
                info!("{} Authenticated and Scoped to {}", name, db_name);
                break;
            }
            warn!(" Waiting for {} service (attempt {}/10)...", name, i);
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }

        if !authenticated {
            error!(" {} Connection Failed. Check systemctl status surrealdb-primary", name);
            return Err(surrealdb::Error::thrown(format!("{} Auth Timeout", name)).into());
        }
    }

    // 5. Apply Schemas
    // We apply schemas to both nodes so they stay in sync
    let schema_sql = include_str!("schema.surql"); 
    let tick_schema_sql = include_str!("mem_tick_schema.surql");
    let market_data_schema_sql = include_str!("market_data_schema.surql");
    let functions_sql = include_str!("functions.surql");    

    // info!(" Applying schemas to all nodes...");
    // for db in [&mem_db, &disk_db] {
    //     db.query(schema_sql).await?;
    //     // db.query(tick_schema_sql).await?;
    //     db.query(market_data_schema_sql).await?;
    //    let _ = sync_timezone_config(db).await; //<--- Can i use it like this in the db setup 
        
    // }

    info!(" Applying schemas to all nodes...");
    for db in [&mem_db, &disk_db] {
        db.query(schema_sql).await?;
        db.query(market_data_schema_sql).await?;
        db.query(functions_sql).await?;
        
        // Propagate the error! Do not use `let _ =`
        sync_timezone_config(db).await
            .map_err(|e| {
                error!("Failed to sync DST config: {}", e);
                e // Keep error propagating
            })?; 
    }
    mem_db.query(tick_schema_sql).await?;

    let dbs = Arc::new(AppDatabases {
        mem: mem_db,
        disk: disk_db,
        mirror: mirror_db,
    });
    
    info!(" Hybrid System Online (Mem:8000, Disk:8001)");
    Ok(dbs)
}









// use serde::{Deserialize, Serialize};


// use surrealdb::types::Notification;

// use surrealdb::method::QueryStream;


// use surrealdb::types::Action;
// use surrealdb::opt::Resource;

// use futures_util::StreamExt; // Still required for .next()








pub async fn register_assets(dbs: Arc<AppDatabases>, packet: AssetInfoPacket) -> surrealdb::Result<()> {

    let packet = Arc::new(packet);
    let asset_data = map_packet_to_asset(&packet);

    // Composite ID: symbol + source ensures uniqueness across providers
    // let asset_key = format!("{}:{}", asset_data.symbol, asset_data.source);
    // let record_id = RecordId::new("assets", asset_key);
    let id = make_composite_id("assets", &[&asset_data.symbol, &asset_data.source]);
    

    // Upsert the record

    // let _: Option<Asset> = dbs.mem
    //     .upsert(id.clone())
    //     .content(asset_data.clone()) 
    //     .await?;

    let _ = dbs.mem
    .query("UPSERT $id CONTENT $data")
    .bind(("id", id.clone()))
    .bind(("data", asset_data.clone()))
    .await
    .context("Failed to upsert asset into Memory Node");

    let _ = dbs.disk
    .query("UPSERT $id CONTENT $data")
    .bind(("id", id.clone()))
    .bind(("data", asset_data.clone()))
    .await
    .context("Failed to upsert asset into Disk Node");

    // 2. Register in Disk (WebSocket)
        // let _: Option<Asset> = dbs.disk
        //     .upsert(record_id.clone())
        //     .content(asset_data.clone())
        //     .await?;

    
    info!("SurrealDB: Updated DNA for {}", id.to_raw_string());
    Ok(())
}

pub fn map_packet_to_asset(packet: &AssetInfoPacket) -> Asset {

    let clean_str = |b: &[u8]| {
        String::from_utf8_lossy(b)
            .trim_matches(char::from(0))
            .to_string()
    };

    let symbol = clean_str(&packet.asset);
    let (home_tz, exchange) = get_static_metadata(&symbol);
    Asset {
        symbol: clean_str(&packet.asset),
        name: clean_str(&packet.name),
        source: clean_str(&packet.source),
        asset_type: clean_str(&packet.asset_type),
        timezone: home_tz, 
        exchange: exchange,
        sector: clean_str(&packet.sector),
        industry: clean_str(&packet.industry),
        
        calculation: AssetCalculationModel {
            digits: packet.calculation.digits,
            stops_level: packet.calculation.stops_level,
            tick_value: packet.calculation.tick_value,
            chart_mode: clean_str(&packet.calculation.chart_mode),
        },
        
        execution: AssetExecutionModel {
            tick_size: packet.execution.tick_size,
            lot_size: packet.execution.lot_size,
            min_quantity: packet.execution.min_quantity,
            max_quantity: packet.execution.max_quantity,
            volume_step: packet.execution.volume_step,
            filling_modes: packet.execution.filling_modes,
            order_modes: packet.execution.order_modes,
            expiration_modes: packet.execution.expiration_modes,
        },

        pnl: AssetPnLModel {
            profit_currency: clean_str(&packet.pnl.profit_currency),
            swap_long: packet.pnl.swap_long,
            swap_short: packet.pnl.swap_short,
            swap_rates: vec![
                packet.pnl.swap_rates.monday,
                packet.pnl.swap_rates.tuesday,
                packet.pnl.swap_rates.wednesday,
                packet.pnl.swap_rates.thursday,
                packet.pnl.swap_rates.friday,
            ],
        },

        risk: AssetRiskModel {
            initial_margin: packet.risk.initial_margin,
            maintenance_margin: packet.risk.maintenance_margin,
            max_leverage: packet.risk.max_leverage,
            is_shortable: packet.risk.is_shortable != 0,
            margin_currency: clean_str(&packet.risk.margin_currency),
        },

        sessions: packet.sessions.iter().enumerate()
            .filter(|(_, s)| s.is_active == 1)
            .map(|(i, s)| TradingSession {
                day_index: i as i32,
                open: format!("{:02}:{:02}", s.open_hour, s.open_min),
                close: format!("{:02}:{:02}", s.close_hour, s.close_min),
            })
            .collect(),
    }
}

pub fn get_static_metadata(symbol: &str) -> (String, String) {
    let cache = METADATA_CACHE.get_or_init(|| {
        // Load the file on the first call
        let data = fs::read_to_string("assets.json")
            .expect("Failed to read assets.json - ensure it is in the root directory");
        
        serde_json::from_str(&data)
            .expect("Failed to parse assets.json - check format")
    });

    // Lookup symbol, fallback to defaults if not found
    match cache.get(symbol) {
        Some(m) => (m.timezone.clone(), m.exchange.clone()),
        None => {
            // Logic for FX pairs (6 chars) if not explicitly in JSON
            if symbol.len() == 6 {
                ("Europe/London".to_string(), "Interbank".to_string())
            } else {
                ("UTC".to_string(), "Unknown".to_string())
            }
        }
    }
}



pub fn get_ny_dst_map()  -> AppResult<HashMap<String, DstRange>> {
    let data = fs::read_to_string("dst_rules.json")
        .context("Failed to read dst_rules.json")?;
    let map: HashMap<String, DstRange> = serde_json::from_str(&data)
        .context("Failed to deserialize dst_rules.json")?;
    Ok(map)
}



// pub async fn sync_timezone_config<C: Connection>(
//     db: &Surreal<C>
// )  -> AppResult<()> {
//     let ny_map = get_ny_dst_map()?;

//   let _result: DstConfig = db.upsert(("dst_config", "ny_map"))
//     .merge(serde_json::json!({ "years": ny_map }))
//     .await?
//     .expect("Wrong DTS");

//     Ok(())
// }

pub async fn sync_timezone_config<C: Connection>(
    db: &Surreal<C>
) -> AppResult<()> {
    let ny_map = get_ny_dst_map()?;

    // Use a parameterized query with explicit casting
    db.query("UPSERT dst_config:ny_map MERGE { years: $map }")
        .bind(("map", ny_map))
        .await?
        .check()?;

    Ok(())
}


// pub async fn start_live_bridge(dbs: Arc<AppDatabases>) -> surrealdb::Result<()> {
//     // 1. Subscribe to the Memory Node's candle table
//     let mut stream = dbs.mem
//         .select("market_data")
//         .live()
//         .await?;

//     info!("High-Performance Bridge Active: Listening for 5m completions...");

//     while let Some(notification) = stream.next().await {
//         match notification {
//             Ok(Notification { data, action, .. }) => {
//                 // We only care about NEW candles (Action::Create)
//                 if action == Action::Create {
//                     let new_candle: MarketData = data;
                    
                
                    
//                     // Use * to dereference the Surreal Datetime into a Chrono Datetime
//                     // let target_ts = *new_candle.time - chrono::Duration::minutes(5);   
                    
//                     // Fetch the stable candle from RAM and push to Disk
//                     let dbs_clone = Arc::clone(&dbs);
//                     tokio::spawn(async move {
//                         // 1. Fetch from RAM
//                         if let Ok(mut res) = dbs_clone.mem
//                             .query("SELECT * FROM market_data WHERE time = $ts - 5m")
//                             .bind(("ts", new_candle.time))
//                             .await 
//                         {
//                             let stable_candles: Vec<MarketData> = res.take(0).unwrap_or_default();
                            
//                             for candle in stable_candles {
//                                 // 2. Explicitly define the ID in the INSERT statement
//                                 // Format: INSERT INTO market_data:[asset, time] CONTENT $data
//                                 let id = RecordId::new("market_data", format!("{}:{}", candle.asset_id.to_raw_string(), candle.time)); // Composite ID
//                                 let query = "INSERT INTO market_data:[$asset_id, $time] CONTENT $data";
                                
//                                 let _ = dbs_clone.disk
//                                     .query(query)
//                                     .bind(("asset_id", candle.asset_id.clone()))
//                                     .bind(("time", candle.time.clone()))
//                                     .bind(("data", candle)) // The DB will ignore the 'id' inside $data and use the one in the path
//                                     .await;

//                                 // info!("Explicitly mirrored {:?} candle to Disk with Composite ID", candle.asset_id);
//                             }
                            
                           
//                         }
//                     });
//                 }
//             }
//             Err(e) => error!("Live Stream Error: {}", e),
//         }
//     }
//     Ok(())
// }


// pub async fn start_optimized_flush(dbs: Arc<AppDatabases>) {
//     // Check every 30 seconds to see if we've crossed a 5m boundary
//     let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30)); 
//     let mut last_flushed_window: Option<Datetime> = None;

//     info!("🚀 Flush Worker Active: Synchronized to 5m clock.");

//     loop {
//         interval.tick().await;

//         let now = chrono::Utc::now();
//         // 1. Calculate the floor of the CURRENT window (e.g., 14:10:00)
//         let current_window_timestamp = now.timestamp() - (now.timestamp() % 300);
//         let current_window = Datetime::from(chrono::DateTime::from_timestamp(current_window_timestamp, 0).unwrap());

//         // 2. We only flush if we have moved INTO a new window (at least 1 second past the floor)
//         // and we haven't flushed the PREVIOUS window yet.
//         let target_ts = Datetime::from(chrono::DateTime::from_timestamp(current_window_timestamp - 300, 0).unwrap());

//         if last_flushed_window != Some(target_ts.clone()) && now.timestamp() % 300 >= 1 {
            
//             // 3. Targeted Move Query
//             // We select from RAM and 'upsert' to Disk with explicit ID formatting
//             let query = "
//                 let $candles = SELECT * FROM market_data WHERE time = $target_ts;
//                 RETURN $candles;
//             ";

//             if let Ok(mut res) = dbs.mem.query(query).bind(("target_ts", target_ts.clone())).await {
//                 let completed_candles: Vec<Candle> = res.take(0).unwrap_or_default();

//                 if !completed_candles.is_empty() {
//                     for mut candle in completed_candles {
//                         // Ensure the ID is composite for the Disk Node
//                         let _ = dbs.disk
//                             .query("UPSERT market_data:[$asset, $time] CONTENT $data")
//                             .bind(("asset", candle.asset_id.clone()))
//                             .bind(("time", candle.time.clone()))
//                             .bind(("data", candle))
//                             .await;
//                     }
//                     info!("💾 Flushed window {} to Disk Node", target_ts);
//                     last_flushed_window = Some(target_ts);
//                 }
//             }
//         }
//     }
// }


// pub async fn start_optimized_flush(dbs: Arc<AppDatabases>) {
//     // Check every 30 seconds to catch the window
//     let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30)); 
//     let mut last_flushed_window: Option<Datetime> = None;

//     info!("Flush Worker Active: Synchronized to 5m clock.");

//     loop {
//         interval.tick().await;

//         let now = chrono::Utc::now();
//         let now_ts = now.timestamp();
        
//         // 1. Calculate the floor of the CURRENT window (e.g., 14:10:00)
//         let current_window_timestamp = now_ts - (now_ts % 300);
        
//         // 2. The target is the window that just closed (5 minutes ago)
//         let target_chrono = chrono::DateTime::from_timestamp(current_window_timestamp - 300, 0)
//             .expect("Invalid timestamp");
//         let target_ts = Datetime::from(target_chrono);

//         // 3. Logic Guard:
//         // seconds_into_window: How far are we into the current 5m block?
//         let seconds_into_window = now_ts % 300;

//         // We run if:
//         // - We are at least 1 second into the new window (to allow ingestion to finish)
//         // - We are not too close to the end of the window (before 240s)
//         // - We haven't successfully flushed this target_ts yet.
//         if seconds_into_window >= 1 && seconds_into_window < 240 && last_flushed_window != Some(target_ts.clone()) {
            
//             info!("Flush trigger hit for window: {}. Offset: {}s", target_ts, seconds_into_window);

//             // 4. Targeted Move Query
//             let select_query = "SELECT * FROM market_data WHERE time = $target_ts";

//             match dbs.mem.query(select_query).bind(("target_ts", target_ts.clone())).await {
//                 Ok(mut res) => {
//                     let completed_candles: Vec<MarketData> = res.take(0).unwrap_or_default();

//                     if !completed_candles.is_empty() {
//                         let count = completed_candles.len();
//                         for candle in completed_candles {
//                             // Explicitly UPSERT to Disk with Composite ID
//                             let _ = dbs.disk
//                                 .query("UPSERT market_data:[$asset_id, $time] CONTENT $data")
//                                 .bind(("asset_id", candle.asset_id.clone()))
//                                 .bind(("time", candle.time.clone()))
//                                 .bind(("data", candle))
//                                 .await;
//                         }
//                         info!("Successfully flushed {} candles for window {} to Disk", count, target_ts);
//                         last_flushed_window = Some(target_ts);
//                     } else {
//                         // If we hit the trigger but found no data, we don't update last_flushed_window
//                         // This allows it to retry on the next 30s tick in case ingestion was slightly late
//                         warn!("flush trigger hit for {}, but no candles were found in Mem", target_ts);
//                     }
//                 },
//                 Err(e) => error!("Failed to query Mem Node during flush: {}", e),
//             }
//         }
//     }
// }