use std::env;

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
    /// Loads configuration from environment variables and calculates the single target topic.
    pub fn load() -> Result<Self, String> {
        // Load dotenv file if available (requires the 'dotenv' crate dependency)
        // let _ = dotenv::dotenv(); 

        let kafka_brokers = env::var("KAFKA_BROKERS").map_err(|_| "KAFKA_BROKERS not set in .env")?;
        let data_source_id = env::var("DATA_SOURCE_ID").map_err(|_| "DATA_SOURCE_ID not set in .env")?;
        let file_path = env::var("FILE_PATH").map_err(|_| "FILE_PATH not set in .env")?;
        
        let asset_symbol = env::var("ASSET_SYMBOL").map_err(|_| "ASSET_SYMBOL not set in .env")?;
        let kafka_topic_base = env::var("KAFKA_TOPIC_BASE").map_err(|_| "KAFKA_TOPIC_BASE not set in .env")?;

        // 💥 THE CRUCIAL FIX: CALCULATE THE SINGLE TARGET TOPIC
        // This ensures the producer sends data to "us100_market_data" (single topic)
        let kafka_topic = format!(
            "{}_{}", 
            asset_symbol.to_lowercase(), 
            kafka_topic_base
        );

        Ok(ProducerConfig {
            kafka_brokers,
            kafka_topic, // The calculated single topic
            data_source_id,
            file_path,
            asset_symbol,
            kafka_topic_base,
        })
    }
    
    // Helper function used by ingestion.rs to construct the full asset ID
    pub fn get_asset_id(&self) -> String {
        format!("assets:{}:{}", self.asset_symbol, self.data_source_id)
    }

    pub fn get_topic_base(&self) -> &str {
        &self.kafka_topic_base
    }
}
