use std::env;
use serde::Deserialize;
use anyhow::Result; // Assuming you use anyhow::Result in this module

// --- NEW STRUCTS: For Multi-Asset Ingestion Coordination ---

/// Configuration for a single asset's CSV ingestion job.
#[derive(Debug, Deserialize, Clone)]
pub struct AssetIngestJob {
    pub symbol: String,
    pub file_path: String,
    pub topic_base: String, // e.g., "market_data"
}

/// The overall configuration for the ingestion coordinator, loaded from a config file.
#[derive(Debug, Deserialize)]
pub struct IngestionCoordinatorConfig {
    pub kafka_brokers: String,
    pub data_source_id: String,
    pub assets: Vec<AssetIngestJob>,
}

impl IngestionCoordinatorConfig {
    /// Loads the configuration from a file (requires `serde_yaml` or similar).
    pub fn load_from_file(path: &str) -> Result<Self> {
        // NOTE: This implementation requires `serde_yaml` or another parser
        let file_content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&file_content)?;
        Ok(config)
    }
}

// --- ProducerConfig Definition ---

#[derive(Debug, Clone)]
pub struct ProducerConfig {
    pub kafka_brokers: String,
    // This is the SINGLE, calculated topic the Producer will send to.
    pub kafka_topic: String, 
    pub data_source_id: String,
    pub file_path: String,
    pub asset_symbol: String,
    // We keep the topic base for calculation, but don't need to expose it outside this file.
    kafka_topic_base: String, 
}

impl ProducerConfig {
    // REMOVED: The pub fn load() function that relied on std::env::var for asset-specific fields.

    /// NEW CONSTRUCTOR: Initializes config from shared brokers and per-asset job data.
    /// This decouples the config from environment variables for asset-specific settings.
    pub fn from_job(
        brokers: String, 
        data_source: String, 
        job: AssetIngestJob
    ) -> Self {
        
        // 💥 CALCULATE THE SINGLE TARGET TOPIC based on the job data
        let kafka_topic = format!(
            "{}_{}", 
            job.symbol.to_lowercase(), 
            job.topic_base
        );

        ProducerConfig {
            kafka_brokers: brokers,
            kafka_topic, // The calculated single topic
            data_source_id: data_source,
            file_path: job.file_path,
            asset_symbol: job.symbol,
            kafka_topic_base: job.topic_base,
        }
    }
    
    // Helper function used by ingestion.rs to construct the full asset ID
    pub fn get_asset_id(&self) -> String {
        format!("assets:{}:{}", self.asset_symbol, self.data_source_id)
    }

    pub fn get_topic_base(&self) -> &str {
        &self.kafka_topic_base
    }
}