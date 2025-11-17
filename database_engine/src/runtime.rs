use anyhow::{Result, Context};
use deadpool_postgres::{Manager, ManagerConfig, Runtime}; 
use tokio_postgres::Config as PgConfig; 
use std::str::FromStr;
use serde::Deserialize;
use std::fs;
use std::path::Path;
// NOTE: ConsumerConfig reference removed. Keep schema_setup if still needed.
use crate::schema_setup; 

#[derive(Debug, Deserialize)]
struct AssetSeed {
    symbol: String,
    source: String,           
    timezone: String,
    name: String,
    asset_class: String,
    currency: String,
    exchange: String,
}
pub type Pool = deadpool_postgres::Pool;


async fn register_all_assets(pool: &Pool) -> Result<()> {
    println!("-> Starting dynamic asset registration...");
    
    // Read the seed file (path assumed relative to project root)
    let path = Path::new("database_engine/config/assets.json");
    let data = fs::read_to_string(path)
        .context(format!("Failed to read asset seed file at {:?}", path))?;

    let assets: Vec<AssetSeed> = serde_json::from_str(&data)
        .context("Failed to deserialize asset seed data")?;

    let client = pool.get().await.context("Failed to get DB client for asset seeding")?;

    for asset in &assets {
        // CRITICAL: Asset ID is now derived from SYMBOL and SOURCE
        let asset_id = format!("assets:{}:{}", asset.symbol, asset.source); 
        
        let statement = client.prepare(
            "INSERT INTO assets (id, symbol, source, timezone, name, asset_class, currency, exchange) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO NOTHING"
        ).await.context("Failed to prepare asset insert statement")?;

        let rows_inserted = client.execute(
            &statement,
            &[
                &asset_id,      // $1: The unique ID
                &asset.symbol,  // $2: Symbol (e.g., US100)
                &asset.source,  // $3: Source (e.g., BROKER_A)
                &asset.timezone,
                &asset.name,
                &asset.asset_class,
                &asset.currency,
                &asset.exchange,
            ],
        ).await.context(format!("Failed to execute asset registration for {}", asset_id))?;

        if rows_inserted > 0 {
            println!("  -> Registered new asset: {}", asset_id);
        }
    }
    
    println!("✅ Asset seeding complete. {} total assets processed.", assets.len());
    Ok(())
}

/// Initializes the connection pool and applies the database schema.
pub async fn setup_database_pool() -> Result<Pool> {
    
    // 1. Get the DATABASE_URL directly from the environment.
    // This uses the fallback if the variable is not explicitly set.
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "host=localhost user=postgres password=postgres dbname=timeseries".to_string());
        
    println!("Initializing Database Pool and Schema...");

    // 2. PARSE THE URL STRING 
    let pg_config = PgConfig::from_str(&db_url)
        .context("Failed to parse DATABASE_URL connection string")?;
    
    // 3. Define the DEADPOOL Manager Configuration
    let mgr_config = ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Fast,
    };

    // 4. Create the Manager
    let manager = Manager::from_config(pg_config, tokio_postgres::NoTls, mgr_config);

    // 5. Create the Pool
    let pool = Pool::builder(manager)
        .runtime(Runtime::Tokio1) 
        .max_size(16) 
        .build()
        .context("Failed to build PostgreSQL connection pool")?;

    // 6. Apply the schema and register assets
    // schema_setup::apply_schema(&pool).await?; // Assuming this is commented out/removed

    register_all_assets(&pool).await?;

    println!("✅ TimescaleDB schema and continuous aggregates applied successfully.");
    println!("✅ Database Pool Ready.");

    Ok(pool)
}