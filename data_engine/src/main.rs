// use tokio::net::TcpListener;
// use tokio::io::{AsyncReadExt, AsyncWriteExt};
// use bytemuck::{Pod, Zeroable};
// use chrono::Utc;

// // --- PACKED STRUCTURES (Exactly 64 Bytes) ---
// #[repr(C, packed)]
// #[derive(Copy, Clone, Pod, Zeroable, Debug)]
// struct MultiplexedTick {
//     asset_name: [u8; 10], _dummy: [u8; 6],
//     bid: f64, ask: f64, last: f64,
//     volume: i64, time_msc: i64,
//     flags: u32, _end_pad: u32,
// }

// #[repr(C, packed)]
// #[derive(Copy, Clone, Pod, Zeroable, Debug)]
// struct SyncBar {
//     asset: [u8; 10], _pad: [u8; 6],
//     open: f64, high: f64, low: f64, close: f64,
//     volume: i64, time: i64,
// }

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let listener = TcpListener::bind("127.0.0.1:9090").await?;
//     println!("📡 Rust Engine Online: 127.0.0.1:9090");

//     loop {
//         let (mut socket, _) = listener.accept().await?;
        
//         tokio::spawn(async move {
//             println!("✅ MT5 Connected.");

//             // 1. Send Heal Request (Request bars from last 12 hours)
//             let start_sync_ts = Utc::now().timestamp() - (1 * 60 * 60); 
//             if socket.write_all(&start_sync_ts.to_le_bytes()).await.is_ok() {
//                 let _ = socket.flush().await;
//             }

//             let mut header = [0u8; 1];
//             let mut buf = [0u8; 64];

//             loop {
//                 // Read 1-byte Header
//                 if socket.read_exact(&mut header).await.is_err() { break; }
//                 // Read 64-byte Payload
//                 if socket.read_exact(&mut buf).await.is_err() { break; }

                // match header[0] {
                //     0 => { // TICK
                //         if let Ok(tick) = bytemuck::try_from_bytes::<MultiplexedTick>(&buf) {
                //             let asset_cow = String::from_utf8_lossy(&tick.asset_name);
                //             let asset = asset_cow.trim_matches(char::from(0));
                //             let bid = tick.bid;
                //             println!("📈 [LIVE] {} | Bid: {:.2}", asset, bid);
                //         }
                //     }
                //     1 => { // BAR
                //         if let Ok(bar) = bytemuck::try_from_bytes::<SyncBar>(&buf) {
                //             let time = bar.time;
                //             let close = bar.close;      
                //             println!("📊 [SYNC] Time: {} | Close: {:.2}", time, close);
                //         } else {
                //             println!("❌ SyncBar ByteMuck failed - size mismatch");
                //         }
                //     }
                //     _ => {
                //         println!("⚠️ Desync: Unknown Header {}", header[0]);
                //         break; 
                //     }
                // }
        //     }
        //     println!("❌ Connection Closed.");
        // });
//     }
// }


use data_engine::data_service;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytemuck::{Pod, Zeroable};
use serde::{Serialize, Deserialize};
use data_engine::producer_config::{IngestionCoordinatorConfig,AssetIngestJob};
use shared_models::data_model::DataService;
use data_engine::traits::DataServiceBase;
use database_engine::runtime;
use dotenvy;
use std::env;
use std::sync::Arc;
use anyhow::Context;
use chrono::Utc;
use data_engine::watchdog::{Mt5Watchdog,WatchdogState};
use log::{info, error, debug, warn};
// use surrealdb::engine::remote::ws::{Ws, Client};
// use surrealdb::opt::auth::Root;
// use surrealdb::Surreal;
use surrealdb::{Surreal, engine::local::Mem, engine::local::SurrealKv, opt::auth::Root};
#[repr(C, packed)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
struct MultiplexedTick {
    asset_name: [u8; 10], _dummy: [u8; 6],
    bid: f64, ask: f64, last: f64,
    volume: i64, time_msc: i64,
    flags: u32, _end_pad: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
struct SyncBar {
    asset: [u8; 10], _pad: [u8; 6],
    open: f64, high: f64, low: f64, close: f64,
    volume: i64, time: i64,
}





// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {

//     env_logger::init();
//     dotenvy::dotenv().ok();
//     // 1. Connect to SurrealDB
//     let pool = runtime::setup_database_pool().await?;
//     let data_service = Arc::new(DataService::new(pool));

//     let listener = TcpListener::bind("127.0.0.1:9090").await?;
//     println!("📡 Rust Engine Online: 127.0.0.1:9090");

//     loop {
//         let (mut socket, _) = listener.accept().await?;
//         let data_service = Arc::clone(&data_service);

//         tokio::spawn(async move {
//             println!("✅ MT5 Connected. Preparing Subscription...");
            
//             let config_path = env::var("INGESTION_CONFIG_PATH").unwrap_or_else(|_| "config.json".into());
//             let config = IngestionCoordinatorConfig::load_from_file(&config_path).expect("Config load failed");

//             // 2. Build Subscriptions with real HWM
//             let mut sub_list = Vec::new();
//             for asset in config.assets.iter().filter(|a| a.enabled) {
//                 let asset_id = format!("assets:{}:{}", asset.system_symbol.to_uppercase(), config.data_source_id);
//                 let hwm = data_service.get_market_data_high_watermark(&asset_id).await.unwrap_or(None);
                
//                 sub_list.push(serde_json::json!({
//                     "mt5": asset.mt5_symbol,
//                     "tf": asset.timeframe,
//                     "hwm": hwm
//                 }));
//             }

//             let handshake_json = serde_json::to_string(&sub_list).unwrap();
//             let json_bytes = handshake_json.as_bytes();
//             let json_len = json_bytes.len() as u32;

//             // 3. Handshake: [Type 255][Length 4][JSON]
//             let mut header_buf = [0u8; 5];
//             header_buf[0] = 255;
//             header_buf[1..5].copy_from_slice(&json_len.to_le_bytes());
//             let _ = socket.write_all(&header_buf).await;
//             let _ = socket.write_all(json_bytes).await;
//             let _ = socket.flush().await;

//             let mut header = [0u8; 1];
//             let mut buf = [0u8; 64];

//             loop {
//                 if socket.read_exact(&mut header).await.is_err() { break; }
//                 if socket.read_exact(&mut buf).await.is_err() { break; }

//                 match header[0] {
//                     0 => { // TICK -> Store in Surreal or Stream
//                          if let Ok(tick) = bytemuck::try_from_bytes::<MultiplexedTick>(&buf) {
//                             let asset_cow = String::from_utf8_lossy(&tick.asset_name);
//                             let asset = asset_cow.trim_matches(char::from(0));
//                             let bid = tick.bid;
//                             println!("📈 [LIVE] {} | Bid: {:.2}", asset, bid);
//                         }
//                     }
//                     1 => { // BAR
//                         if let Ok(bar) = bytemuck::try_from_bytes::<SyncBar>(&buf) {
//                             let time = bar.time;
//                             let close = bar.close;      
//                             println!("📊 [SYNC] Time: {} | Close: {:.2}", time, close);
//                         } else {
//                             println!("❌ SyncBar ByteMuck failed - size mismatch");
//                         }
//                     }
//                     _ => {
//                         println!("⚠️ Desync: Unknown Header {}", header[0]);
//                         break;
//                     }
//                 }
//             }
//             println!("❌ Connection Closed.");
//         });
//     }
// }

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // tracing_subscriber::fmt::init();
//     env_logger::init();
//     dotenvy::dotenv().ok();
    
//     // 1. Connect to DB (using root/root)
//     let pool = runtime::setup_database_pool().await?; 
    
//     // 2. Load Config
//     // let config = IngestionCoordinatorConfig::load_from_file("config.json")?;

//     // 3. Create Service (It takes ownership/Arc of both DB and Config)
//     let data_service = Arc::new(DataService::new(pool));
//     let state = WatchdogState::new();
//     let watchdog_state = state.clone();

//     // 4. Just Run
//     data_service.run("127.0.0.1:9090", Arc::clone(&data_service)).await
// }


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    dotenvy::dotenv().ok();
    
    let pool = runtime::setup_database_pool().await?; 
    let database = runtime::setup_database().await?;
    let dbs = database.clone();
    let data_service = Arc::new(DataService::new(pool));

    // ✅ Create the state OUTSIDE the service
    let state = WatchdogState::new();

    // ✅ Spawn the watchdog using this state
    let watchdog_state = state.clone();
    tokio::spawn(async move {
        let watchdog = Mt5Watchdog::new(watchdog_state);
        if let Err(e) = watchdog.run().await {
            error!("Watchdog error: {}", e);
        }
    });

    // ✅ Pass the state into the run method
    data_service.run_new("127.0.0.1:9090", dbs, Arc::clone(&data_service), state).await
}
