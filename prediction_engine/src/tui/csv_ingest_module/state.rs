use chrono::{DateTime, Utc};

#[derive(Debug, Clone)] // Add Default if you want CsvIngestState::default()
pub struct CsvIngestState {
    // 💥 ADD THIS FIELD: A history of status messages for the TUI panel
    pub log_history: Vec<String>,
}

impl Default for CsvIngestState {
    fn default() -> Self {
        Self {
            log_history: vec!["STATUS: Ready to process assets listed in ingestion_assets.json. Press I to start ingestion.".to_string()],
        }
    }
}