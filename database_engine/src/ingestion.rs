use anyhow::{Result, Context};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    Message,
};
use deadpool_postgres::Pool;
use shared_models::{market_data::MarketData, time_utils};
use crate::config::ConsumerConfig;


/// Executes a batch of market data inserts within a single database transaction.
async fn execute_batch(pool: &Pool, sql_stmt: &str, batch: &[MarketData]) -> Result<()> {
    let mut client = pool.get().await.context("Failed to get DB client from pool")?;
    let transaction = client.transaction().await.context("Failed to begin transaction")?;
    let statement = transaction.prepare(sql_stmt).await.context("Failed to prepare statement")?;

    for data in batch {
        let timestamp = time_utils::ts_to_utc_datetime(data.ts)?;

        transaction.execute(
            &statement,
            &[&timestamp, &data.asset_id, &data.open, &data.high, &data.low, &data.close, &data.volume],
        ).await.context("Failed to execute batch insert")?;
    }

    transaction.commit().await.context("Failed to commit transaction")?;
    println!("Successfully inserted {} records into market_data_1hr.", batch.len());
    Ok(())
}


/// Main asynchronous consumer loop for Kafka to TimescaleDB ingestion.
pub async fn run_consumer(config: ConsumerConfig, pool: Pool) -> Result<()> {
    
    // Dynamic Asset ID Derivation from Topic
    let asset_symbol = config.kafka_topic
        .split('_')
        .next()
        .context("Invalid Kafka topic format")?
        .to_uppercase();
    let injected_asset_id = format!("assets:{}", asset_symbol);
    
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", &config.kafka_group_id)
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()
        .context("Consumer creation error")?;
        
    consumer.subscribe(&[&config.kafka_topic])
        .context("Failed to subscribe to Kafka topic")?;

    let sql_stmt = format!("INSERT INTO market_data_1hr (time, asset_id, open, high, low, close, volume) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (time, asset_id) DO NOTHING");

    let mut batch: Vec<MarketData> = Vec::with_capacity(1000);
    
    loop {
        match consumer.recv().await {
            Err(e) => eprintln!("Kafka error: {:?}", e),
            Ok(msg) => {
                if let Some(payload) = msg.payload() {
                    match serde_json::from_slice::<MarketData>(payload) {
                        Ok(mut market_data) => {
                            // CRITICAL: ASSET ID INJECTION (for real-time consistency)
                            market_data.asset_id = injected_asset_id.clone();
                            
                            batch.push(market_data);
                            
                            if batch.len() >= 1000 {
                                if let Err(e) = execute_batch(&pool, &sql_stmt, &batch).await {
                                    eprintln!("TimescaleDB batch insert failed: {:?}", e);
                                }
                                batch.clear();
                            }
                        },
                        Err(e) => eprintln!("Failed to deserialize payload: {:?}", e),
                    }
                }
            },
        }
    }
}