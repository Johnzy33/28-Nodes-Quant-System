use anyhow::{Result, Context, anyhow};
use chrono::{NaiveDateTime, Utc, TimeZone}; 
use std::time::Duration;
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
    producer::Producer as KafkaProducer, 
};
use csv::{ReaderBuilder, Trim}; 
use crate::{
    csv_reader::{CsvRecordStandard, CsvRecordBracketed}, 
    config::ProducerConfig,
}; 
// This line imports your critical time utility functions
use shared_models::{market_data::MarketData, time_utils}; 
use serde::de::DeserializeOwned; 
use std::fs::File;

// --- Helper Trait and Implementations ---

/// Defines the method required to convert any CSV record struct into the final MarketData struct.
trait CsvMapping {
    fn to_market_data(&self, asset_id: &str) -> Result<MarketData>;
}

// --- Implement Trait for Standard Struct ---
impl CsvMapping for CsvRecordStandard {
    fn to_market_data(&self, asset_id: &str) -> Result<MarketData> {
        process_record(&self.date, &self.time, asset_id, self.open, self.high, self.low, self.close, self.volume)
    }
}

// --- Implement Trait for Bracketed Struct ---
impl CsvMapping for CsvRecordBracketed {
    fn to_market_data(&self, asset_id: &str) -> Result<MarketData> {
        // volume maps to <TICKVOL> for this struct
        process_record(&self.date, &self.time, asset_id, self.open, self.high, self.low, self.close, self.volume)
    }
}

// --- Common Record Processing Logic ---

fn process_record(
    date: &str, time: &str, asset_id: &str, 
    open: f64, high: f64, low: f64, close: f64, volume: f64
) -> Result<MarketData> {
    
    // 1. Prepare raw date/time string
    let cleaned_date: String = date.trim().chars().filter(|c| c.is_ascii()).collect();
    let cleaned_time: String = time.trim().chars().filter(|c| c.is_ascii()).collect();
    let dt_str = format!("{} {}", cleaned_date, cleaned_time); 
    
    // 💥 THE CRITICAL FIX: Reverting the erroneous logic. 
    // We must call time_utils::parse_ymd_hms_to_utc_datetime. This function 
    // knows the raw string is in Europe::Athens time and correctly converts it to UTC.
    let ts_ms = time_utils::parse_ymd_hms_to_utc_datetime(&dt_str)
        .context("Failed to parse date/time string; check if time_utils anchors to Athens time.")?
        .timestamp_millis();
        
    Ok(MarketData {
        asset_id: asset_id.to_string(), 
        ts: ts_ms,
        open,
        high,
        low,
        close,
        volume,
        seq: None, 
        source: Some("FundedNext".to_string()),
    })
}


/// Ingests historical market data from a CSV file, automatically detecting the header format.
pub async fn ingest_from_csv(config: ProducerConfig) -> Result<()> {
    
    // 1. Check file headers to determine format and delimiter
    let file = File::open(&config.file_path).context("Failed to open CSV file for header check")?;
    
    // Use a robust reader for the initial check (assuming one of two delimiters)
    let mut temp_reader = ReaderBuilder::new()
        .has_headers(true)
        // Check for tab-delimited first, as bracketed files often use tabs
        .delimiter(b'\t') 
        .from_reader(file); 
        
    let headers = temp_reader.headers()?.clone();

    // Check for bracketed format
    let is_bracketed = headers.iter()
        .any(|h| h.trim().starts_with('<') && h.trim().ends_with('>'));

    // Determine which concrete struct type to use
    if is_bracketed {
        println!("INFO: Detected BRACKETED CSV format. Using CsvRecordBracketed.");
        // Bracketed format uses TAB ('\t')
        ingest_generic::<CsvRecordBracketed>(config, b'\t').await
    } else {
        println!("INFO: Detected STANDARD CSV format. Using CsvRecordStandard.");
        // Standard format usually uses COMMA (',')
        ingest_generic::<CsvRecordStandard>(config, b',').await
    }
}

// --- Generic Ingestion Worker ---

async fn ingest_generic<T>(config: ProducerConfig, delimiter: u8) -> Result<()>
where 
    T: DeserializeOwned + CsvMapping, // T must be Deserializable and implement our trait
{
    
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .context("Producer creation error")?;
    
    // Re-open the file to restart reading from the beginning
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        // Set the delimiter dynamically (',' or '\t')
        .delimiter(delimiter) 
        // Crucial: trims whitespace from headers and fields during deserialization
        .trim(Trim::All) 
        .from_path(&config.file_path)?;

    let asset_id = config.get_asset_id();
    let kafka_topic = &config.kafka_topic; 
    
    let mut total_records = 0;
    
    // 3. INGESTION LOOP
    for result in reader.deserialize() {
        let csv_record: T = result.context("CSV record deserialization failed")?;
        
        let market_data = csv_record.to_market_data(&asset_id)
            .context("Failed to map CSV record to MarketData")?;
            
        let payload = serde_json::to_vec(&market_data).context("Failed to serialize MarketData")?;
        let key = format!("{}:{}", market_data.asset_id, market_data.ts); 
        
        let delivery_result = producer
            .send(FutureRecord::to(kafka_topic).payload(&payload).key(&key), Duration::from_secs(0))
            .await;
            
        if let Err((e, _)) = delivery_result {
            eprintln!("Kafka delivery failed for record {}: {:?}", total_records, e);
        }
        total_records += 1;
    }
    
    // 4. CLEANUP
    producer.flush(Duration::from_secs(10))
        .context("Failed to flush remaining Kafka messages")?;
    
    println!("\n✅ Ingestion complete. Total records produced: {}", total_records);
    Ok(())
}
