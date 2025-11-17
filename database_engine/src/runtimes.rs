// database_engine/src/runtime.rs

use anyhow::{Result, Context};
use tokio_postgres::NoTls; // Assuming no SSL/TLS is needed for development
use deadpool_postgres::{ Manager, Runtime}; // Import the deadpool types
use tokio_postgres::Config; // Used to parse the URL

// Imports from local modules
use crate::config::ConsumerConfig; 
// use crate::schema_setup; // If needed

// The consistent type alias is now deadpool_postgres::Pool
pub type Pool = deadpool_postgres::Pool;

// This function needs tokio_postgres updates if you use it
// You'd use `pool.get().await?` to get a client here.
async fn register_all_assets(_pool: &Pool) -> Result<()> {
    println!("-> Starting dynamic asset registration (Placeholder)...");
    // Implementation using deadpool client needed here
    Ok(())
}

/// Initializes the connection pool using deadpool-postgres.
pub async fn setup_database_pool() -> Result<Pool> {
    let consumer_config = ConsumerConfig::load()?;
    let db_url = consumer_config.db_url;

    println!("Initializing Database Pool and Schema...");

    // 1. Parse the db_url string into a tokio_postgres Config.
    // tokio_postgres::Config implements FromStr, allowing direct parsing of the URL string.
    let pg_config: Config = db_url.parse()
        .context("Failed to parse DATABASE_URL into a tokio_postgres::Config. Check the format (e.g., 'host=...').")?;

    // 2. Create the deadpool Manager
    let manager = Manager::new(pg_config, NoTls); 
    
    // You'd use `tokio_postgres::MakeTlsConnect::new(tls_connector)` if you needed SSL.
    // For development, `NoTls` is usually fine.

    // 3. Create the deadpool Pool
    let pool = deadpool_postgres::Pool::builder(manager)
        .max_size(16) // Maximum connections
        .runtime(Runtime::Tokio1) // Specify the Tokio runtime
        .build()
        .context("Failed to build deadpool-postgres pool")?;

    // register_all_assets(&pool).await?; // Assuming this is updated to use deadpool client

    println!("✅ Database Pool Ready.");

    Ok(pool)
}