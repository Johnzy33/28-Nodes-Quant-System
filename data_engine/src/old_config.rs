use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use dotenvy;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducerConfig {
    pub kafka_brokers: String,
    pub kafka_topic_base: String, // e.g., "candles_1hr_raw"
    pub file_path: PathBuf,       // Path to the CSV file
    pub asset_symbol: String,     // e.g., "US100"
    pub data_source_id: String,   // e.g., "BROKER_A"
}

impl ProducerConfig {
    /// Loads configuration settings explicitly from environment variables, 
    /// falling back to development defaults if needed.
    pub fn load() -> Result<Self> {
        // Load environment variables from a .env file if it exists
        dotenvy::dotenv().ok();

        let config = Self {
            kafka_brokers: std::env::var("KAFKA_BROKERS")
                .unwrap_or_else(|_| "localhost:9092".to_string()),
            
            kafka_topic_base: std::env::var("KAFKA_TOPIC_BASE")
                .unwrap_or_else(|_| "asset_market_data".to_string()),

            // CRITICAL FIX: Ensure asset-specific variables are read from the environment
            asset_symbol: std::env::var("ASSET_SYMBOL")
                .context("ASSET_SYMBOL must be set in the environment or .env file.")?,
            
            data_source_id: std::env::var("DATA_SOURCE_ID")
                .context("DATA_SOURCE_ID must be set in the environment or .env file.")?,

            // CRITICAL FIX: CSV file path must be dynamic per asset
            file_path: std::env::var("CSV_FILE_PATH")
                .map(PathBuf::from)
                .context("CSV_FILE_PATH must be set in the environment or .env file.")?,
        };

        Ok(config)
    }
}
