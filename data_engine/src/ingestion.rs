use anyhow::{Result, Context};
use std::time::Duration;
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use csv::ReaderBuilder;
use crate::{csv_reader::CsvRecord, config::ProducerConfig}; 
use shared_models::{market_data::MarketData, time_utils}; 


/// Ingests historical market data from a CSV file and streams it to Kafka.
pub async fn ingest_from_csv(config: ProducerConfig) -> Result<()> {
    
    // 1. DYNAMIC ASSET ID and TOPIC CONSTRUCTION
    let asset_id = format!("assets:{}:{}", config.asset_symbol, config.data_source_id);
    let kafka_topic = format!("{}_{}", config.asset_symbol.to_lowercase(), config.kafka_topic_base);

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .context("Producer creation error")?;
    
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_path(&config.file_path)?;
    
    let mut total_records = 0;
    
    for result in reader.deserialize() {
        let csv_record: CsvRecord = result.context("CSV record deserialization failed")?;
        
        let dt_str = format!("{} {}", csv_record.date, csv_record.time);
        let ts_ms = time_utils::parse_ymd_hms_to_utc_datetime(&dt_str)
            .context("Failed to parse date/time string")?
            .timestamp_millis();
            
        // CRITICAL: ENRICHMENT and INJECTION
        let market_data = MarketData {
            asset_id: asset_id.clone(), 
            ts: ts_ms,
            open: csv_record.open,
            high: csv_record.high,
            low: csv_record.low,
            close: csv_record.close,
            volume: csv_record.volume,
            seq: None, 
            source: Some("FundedNext".to_string()),
        };
        
        let payload = serde_json::to_vec(&market_data).context("Failed to serialize MarketData")?;
        let key = format!("{}:{}", market_data.asset_id, market_data.ts); 
        
        let delivery_result = producer
            .send(FutureRecord::to(&kafka_topic).payload(&payload).key(&key), Duration::from_secs(0))
            .await;
            
        if let Err((e, _)) = delivery_result {
            eprintln!("Kafka delivery failed for record {}: {:?}", total_records, e);
        }
        total_records += 1;
    }
    
    // FutureProducer does not expose an async `flush` method; each send above is awaited,
    // so there is no additional flush necessary here.
    println!("\n✅ Ingestion complete. Total records produced: {}", total_records);
    Ok(())
}