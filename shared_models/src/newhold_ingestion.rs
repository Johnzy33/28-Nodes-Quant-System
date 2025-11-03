// NOTE: This assumes `log` and `tokio-postgres` are configured in your Cargo.toml
use anyhow::{anyhow, Result, Context};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use tokio_postgres::types::ToSql;
use tokio_postgres::{Client, NoTls, Statement};
use serde::{Deserialize, Serialize};
use log::{info, error}; // Integrity 3: Using log crate macros

pub mod timeutil; 

// --- Structs and Constants ---

const ASSET_TABLE_NAME: &str = "US2000_market_data"; // Architecture 1: New table name

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketData {
    pub time: String, 
    #[serde(skip_deserializing)]
    pub asset_id: String, 
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
}

// --- Database Connection and Batch Execution ---

pub async fn connect_to_db(config: &str) -> Result<Client> {
    let (client, connection) = tokio_postgres::connect(config, NoTls).await
        .context("Failed to connect to PostgreSQL/TimescaleDB")?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("Database connection error: {}", e); // Integrity 3
        }
    });

    Ok(client)
}

/// Executes a batch insert operation into TimescaleDB.
async fn execute_batch(pool: &Client, statement: &Statement, batch: &Vec<MarketData>) -> Result<()> {
    let mut data_rows: Vec<Box<dyn ToSql + Sync>> = Vec::new();
    
    // Flatten all parameters into a single vector
    for data in batch {
        // Use the updated timeutil to correctly anchor and convert the time
        let ts_utc = timeutil::parse_ymd_hms_to_utc_datetime(&data.time)?;
        
        data_rows.push(Box::new(ts_utc));
        data_rows.push(Box::new(data.asset_id.clone()));
        data_rows.push(Box::new(data.open));
        data_rows.push(Box::new(data.high));
        data_rows.push(Box::new(data.low));
        data_rows.push(Box::new(data.close));
        data_rows.push(Box::new(data.volume));
    }

    // Execute the prepared statement with the flattened parameters
    let rows_inserted = pool.execute(statement, data_rows.as_slice()).await?;
    info!("Successfully inserted {} records into {}.", rows_inserted, ASSET_TABLE_NAME); // Integrity 3
    
    Ok(())
}

/// Ingests data from Kafka for a specific asset and writes it to TimescaleDB.
pub async fn start_ingestion(
    db_config: String,
    kafka_brokers: String,
    kafka_topic: String,
    injected_asset_id: String,
) -> Result<()> {
    info!("Starting ingestion process for asset: {}", injected_asset_id); // Integrity 3

    let pool = connect_to_db(&db_config).await?;
    
    let sql_stmt_text = format!(
        "INSERT INTO {} (time, asset_id, open, high, low, close, volume) 
        VALUES ($1, $2, $3, $4, $5, $6, $7)", ASSET_TABLE_NAME
    );
    let sql_stmt = pool.prepare(&sql_stmt_text).await?;

    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "timescale_consumer_group")
        .set("bootstrap.servers", kafka_brokers)
        .set("auto.offset.reset", "beginning")
        .create()
        .context("Consumer creation failed")?;

    consumer
        .subscribe(&[&kafka_topic])
        .context(format!("Failed to subscribe to topic {}", kafka_topic))?;

    let mut batch: Vec<MarketData> = Vec::with_capacity(1000);

    // --- Main ingestion loop ---
    loop {
        match consumer.recv().await {
            Err(e) => {
                error!("Kafka error: {:?}", e); // Integrity 3
                // Break on fatal error or stream end
                break; 
            },
            Ok(msg) => {
                if let Some(payload) = msg.payload() {
                    match serde_json::from_slice::<MarketData>(payload) {
                        Ok(mut market_data) => {
                            market_data.asset_id = injected_asset_id.clone();
                            batch.push(market_data);
                            
                            if batch.len() >= 1000 {
                                if let Err(e) = execute_batch(&pool, &sql_stmt, &batch).await {
                                    error!("TimescaleDB batch insert failed: {:?}", e); // Integrity 3
                                }
                                batch.clear();
                            }
                        },
                        Err(e) => error!("Failed to deserialize payload: {:?}", e), // Integrity 3
                    }
                }
            },
        }
    }

    // --- CRITICAL INTEGRITY FIX: FINAL BATCH FLUSH (Integrity 2) ---
    if !batch.is_empty() {
        info!("Flushing final partial batch of {} records...", batch.len()); // Integrity 3
        if let Err(e) = execute_batch(&pool, &sql_stmt, &batch).await {
            error!("TimescaleDB final batch insert failed: {:?}", e); // Integrity 3
            return Err(anyhow!("Failed to insert final batch: {:?}", e));
        }
    }

    info!("Ingestion process finished for topic: {}", kafka_topic); // Integrity 3

    Ok(())
}