
use serde::{Deserialize, Serialize};
use anyhow::Result; 



/// Configuration for a single asset's CSV ingestion job.
// #[derive(Debug, Deserialize, Clone)]
// pub struct AssetIngestJob {
//     pub symbol: String,
//     pub file_path: String,
//     pub topic_base: String, 
// }

#[derive(Debug, Deserialize, Clone)]
pub struct AssetIngestJob {
    pub system_symbol: String, // Formerly 'symbol'
    pub mt5_symbol: String,    // The new mapping field
    //pub file_path: String,
    pub topic_base: String,
    pub timeframe: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncCommand {
    pub command: String,          // e.g., "SYNC"
    pub system_symbol: String,    // e.g., "US100"
    pub mt5_symbol: String,       // e.g., "NDX100"
    pub start_timestamp_ms: i64,  // The HWM from your DB
    pub timeframe: String,        // e.g., "M1"
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
        let file_content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&file_content)?;
        Ok(config)
    }
}

// --- ProducerConfig Definition ---

#[derive(Debug, Clone)]
pub struct ProducerConfig {
    pub kafka_brokers: String,
    pub kafka_topic: String, 
    pub data_source_id: String,
    //pub file_path: String,
    pub asset_symbol: String,
    kafka_topic_base: String, 
}

impl ProducerConfig {
    
    pub fn from_job(
        brokers: String, 
        data_source: String, 
        job: AssetIngestJob
    ) -> Self {
        
        let kafka_topic = format!(
            "{}_{}", 
            job.system_symbol.to_lowercase(), 
            job.topic_base
        );

        ProducerConfig {
            kafka_brokers: brokers,
            kafka_topic, 
            data_source_id: data_source,
            //file_path: job.file_path,
            asset_symbol: job.system_symbol,
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