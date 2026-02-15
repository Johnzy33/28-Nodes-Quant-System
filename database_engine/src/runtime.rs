use anyhow::{Result, Context};
use sqlx::postgres::PgPoolOptions; 
use sqlx::PgPool; 
use std::env;
use std::fmt::format;
use std::fs;
use std::path::Path;
use tokio::task;
use shared_models::models::AssetSeed;
use log::info;

//use crate::schema_setup; 

pub type Pool = PgPool;


async fn register_all_assets(pool: &PgPool) -> Result<()> {
    
    //let path = Path::new("database_engine/config/assets.json").to_owned();
    let path = env::var("ASSET_SEED")
        .context("Asset Path not found")?;

    // Execute synchronous file I/O on a blocking task thread
    let data = task::spawn_blocking(move || {
        fs::read_to_string(&path)
            .context(format!("Failed to read asset seed file at {:?}", path))
    }).await??; 

    let assets_to_seed: Vec<AssetSeed> = serde_json::from_str(&data)
        .context("Failed to deserialize asset seed data")?;

    let mut total_inserted = 0;

    for asset in &assets_to_seed {

        let asset_id = format!("assets:{}:{}", asset.symbol, asset.source); 
        
        let result = sqlx::query(
            "INSERT INTO assets (id, symbol, source, timezone, name, asset_class, currency, exchange, active) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (id) DO NOTHING"
        )
        .bind(&asset_id)                    
        .bind(&asset.symbol)                
        .bind(&asset.source)                
        .bind(&asset.timezone)              
        .bind(&asset.name)                  
        .bind(&asset.asset_class)           
        .bind(&asset.currency)              
        .bind(&asset.exchange)              
        .bind(true)                         
        .execute(pool) 
        .await
        .context(format!("Failed to execute asset registration for {}", asset_id))?;

        if result.rows_affected() > 0 {
            total_inserted += 1;
            info!("  -> Registered new asset: {}", asset_id);
        }
    }

    info!("Asset seeding complete. {} new assets inserted ({} total processed).", 
          total_inserted, assets_to_seed.len());
    Ok(())
}

/// Initializes the connection pool and applies the database schema.
pub async fn setup_database_pool() -> Result<Pool> {
    
    let db_url = env::var("DATABASE_URL")
        .context("DATABASE_URL environment variable must be set (e.g., postgresql://user:pass@host:port/db)")?;

    info!("Initializing Database Pool ...");

    //BUILD THE SQLX POOL
    let pool = PgPoolOptions::new()
        .max_connections(16) // Use max_connections for sqlx
        .connect(&db_url)
        .await
        .context("Failed to build PostgreSQL connection pool using sqlx")?;

    //register_all_assets(&pool).await?;

    info!("Database Pool Ready....");

    Ok(pool)
}

use std::sync::Arc;
use surrealdb::engine::local::{Db, Mem, SurrealKv};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use shared_models::db_models::*;



// pub async fn setup_database() -> surrealdb::Result<Arc<AppDatabases>> {
//     // 1. Initialize engines
//     let mem_db = Surreal::new::<Mem>(()).await?;
//     let disk_db = Surreal::new::<SurrealKv>("28_node_system").await?;

//     // 2. Auth using your root:root credentials
//     let credentials = Root {
//         username: "root",
//         password: "root",
//     };

//     mem_db.signin(credentials.clone()).await?;
//     disk_db.signin(credentials).await?;

//     // 3. Namespace/DB setup
//     mem_db.use_ns("trading_system").use_db("28_node_cache").await?;
//     disk_db.use_ns("trading_system").use_db("28_node").await?;

//     // 4. Execute schema before wrapping in Arc
//     let schema_sql = include_str!("schema.surql"); // Put your SQL in this file
//     disk_db.query(schema_sql).await?;

//     // 5. Wrap in Arc for thread-safe sharing
//     let dbs = Arc::new(AppDatabases {
//         mem: mem_db,
//         disk: disk_db,
//     });

//     println!("✅ Databases initialized and wrapped in Arc");
//     Ok(dbs)
// }


pub async fn setup_database() -> surrealdb::Result<Arc<AppDatabases>> {
    // 1. Initialize engines
    let mem_db = Surreal::new::<Mem>(()).await?;
    let disk_db = Surreal::new::<SurrealKv>("28_node_system").await?;

    // 2. Define the Root User (This fixes InvalidAuth)
    // In embedded mode, we query to define the user on the system level
    let setup_sql = "DEFINE USER root ON SYSTEM PASSWORD 'root' ROLES OWNER;";
    
    // We can run this without signing in first in embedded mode
    let _ = mem_db.query(setup_sql).await?;
    let _ = disk_db.query(setup_sql).await?;

    // 3. Now you can safely sign in
    let credentials = Root {
        username: "root",
        password: "root",
    };

    mem_db.signin(credentials.clone()).await?;
    disk_db.signin(credentials).await?;

    // 4. Namespace/DB setup
    mem_db.use_ns("trading_system").use_db("28_node_cache").await?;
    disk_db.use_ns("trading_system").use_db("28_node").await?;

    // 5. Execute schema
    let schema_sql = include_str!("schema.surql");
    disk_db.query(schema_sql).await?;

    let dbs = Arc::new(AppDatabases {
        mem: mem_db,
        disk: disk_db,
    });
    
    info!("✅ Databases initialized with Root authentication");
    Ok(dbs)
}




// pub async fn register_assets(
//     dbs: Arc<AppDatabases>,
//     asset: AssetInfoPacket,
// ) -> surrealdb::Result<()> {
//     let asset = Arc::new(asset); // Wrap it
//     let id_str = format!("{}:{}", asset.symbol, asset.source);

//     dbs.disk
//         .query("CREATE type::thing('asset', $id) CONTENT $data")
//         .bind(("id", id_str))
//         .bind(("data", (*asset).clone())) // Clone the inner data into the DB
//         .await?;

//     Ok(())

    
// }

pub async fn register_assets(dbs: Arc<AppDatabases>, packet: AssetInfoPacket) -> surrealdb::Result<()> {
    let packet = Arc::new(packet); // Wrap the packet in Arc for thread-safe sharing
    let asset_data = map_packet_to_asset(&packet);

    
    let id = format!("{}:{}", asset_data.symbol, asset_data.source);

    // Update or Create the asset record in the 'assets' table
    let _: Option<Asset> = dbs.disk
        .upsert(("assets", &id))
        .content(asset_data)
        .await?;

    println!("✅ SurrealDB: Updated DNA for {}", id);
    Ok(())
}
pub fn map_packet_to_asset(packet: &AssetInfoPacket) -> Asset {
    let clean_str = |b: &[u8]| {
        String::from_utf8_lossy(b)
            .trim_matches(char::from(0))
            .to_string()
    };

    Asset {
        symbol: clean_str(&packet.asset),
        name: clean_str(&packet.name),
        source: clean_str(&packet.source),
        asset_type: clean_str(&packet.asset_type),
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