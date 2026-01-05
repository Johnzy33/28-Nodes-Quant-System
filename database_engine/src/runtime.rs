use anyhow::{Result, Context};
use sqlx::postgres::PgPoolOptions; 
use sqlx::PgPool; 
use std::env;
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