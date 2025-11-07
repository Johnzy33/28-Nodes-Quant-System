use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use dotenvy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    pub kafka_brokers: String,
    pub kafka_group_id: String,
    pub kafka_topic: String, 
    pub db_url: String, // PostgreSQL connection string
}

impl ConsumerConfig {
    /// Loads configuration settings explicitly from environment variables.
    pub fn 
    load() -> Result<Self> {
        // Load environment variables from a .env file if it exists
        dotenvy::dotenv().ok();
        
        let config = Self {
            kafka_brokers: std::env::var("KAFKA_BROKERS")
                .unwrap_or_else(|_| "localhost:9092".to_string()),
            
            kafka_group_id: std::env::var("KAFKA_GROUP_ID")
                .unwrap_or_else(|_| "db_ingestion_group".to_string()),
            
            // CRITICAL: The consumer's specific topic MUST be explicitly set
            kafka_topic: std::env::var("KAFKA_TOPICS")
                .context("KAFKA_TOPIC must be set in the environment or .env file.")?,
            
            db_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "host=localhost user=postgres password=postgres dbname=timeseries".to_string()),
        };

        Ok(config)
    }
}

