// database_engine/src/schema_setup.rs

use anyhow::{Result, Context};
use deadpool_postgres::Pool;
use tokio_postgres::NoTls;
use tokio::fs;

// NOTE: Ensure your SQL file is accessible at this path relative to the executable
const SCHEMA_FILE_PATH: &str = "sql/schema_setup.sql"; 


pub async fn apply_schema(pool: &Pool) -> Result<()> {
    
    println!("Applying TimescaleDB schema from: {}...", SCHEMA_FILE_PATH);

    // 1. Get a dedicated client connection from the pool
    let client = pool.get().await.context("Failed to get DB client for schema setup")?;

    // 2. Read the entire SQL script file asynchronously
    let sql_script = fs::read_to_string(SCHEMA_FILE_PATH)
        .await
        .context(format!("Failed to read schema file at: {}", SCHEMA_FILE_PATH))?;
    
    // 3. Execute the entire multi-statement script as a batch
    // This is ideal for schema definition (CREATE TABLE, CREATE AGGREGATE, etc.)
    client.batch_execute(&sql_script)
        .await
        .context("Failed to execute batch SQL script against TimescaleDB")?;
    
    println!("✅ TimescaleDB schema and continuous aggregates applied successfully.");
    
    Ok(())
}