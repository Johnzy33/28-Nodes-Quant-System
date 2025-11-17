use anyhow::{Result, Context};
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc::Sender;

use crate::tui::state::{AppState, ActiveMode};
use crate::tui::events::{TuiEvent, HandleAction}; // TuiEvent must include CsvIngestionAssetStatus(String)
use data_engine::ingestion; 
use database_engine::producer_config::{ProducerConfig, IngestionCoordinatorConfig}; 
use log::info; // Using the log macro for robustness

/// **THE ASYNCHRONOUS INGESTION WORKER**
/// Loads the config, runs through all assets sequentially, and handles errors.
async fn run_ingestion_coordinator(
    sender: Sender<TuiEvent>, 
    // Note: AppState is not passed here, as all configuration comes from the file.
) -> Result<()> {
    
    // 1. Load the Coordinator Configuration from the JSON file
    let coordinator_config = IngestionCoordinatorConfig::load_from_file("./database_engine/config/ingestion_assets.json")
        .context("Failed to load ingestion configuration file (ingestion_assets.json). Check file existence and JSON syntax.")?;

    let brokers = coordinator_config.kafka_brokers;
    let data_source = coordinator_config.data_source_id;
    
    // 2. Iterate through all assets and run the job sequentially
    for job in coordinator_config.assets {
        let asset_symbol = job.symbol.clone();
        let kafka_topic = format!("{}_{}", job.symbol.to_lowercase(), job.topic_base);

        // Create the runtime config for the current asset
        let producer_config = ProducerConfig::from_job(
            brokers.clone(), 
            data_source.clone(), 
            job
        );

        // 💥 STEP 1: Notify TUI the ingestion is starting for this specific asset
        let start_msg = format!("STARTING: Ingesting asset {} (Topic: {})", asset_symbol, kafka_topic);
        info!("{}", start_msg); // Log it for terminal output as well
        let _ = sender.send(TuiEvent::CsvIngestionAssetStatus(start_msg)).await;
        
        // Call the core ingestion logic
        match ingestion::ingest_from_csv(producer_config).await {
            Ok(_) => {
                // 💥 STEP 2: Notify TUI of SUCCESS
                let success_msg = format!("✅ COMPLETED: Ingestion successful for {}", asset_symbol);
                info!("{}", success_msg);
                let _ = sender.send(TuiEvent::CsvIngestionAssetStatus(success_msg)).await;
            }
            Err(e) => {
                // 💥 STEP 3: Notify TUI of FAILURE
                let error_msg = format!("❌ FAILURE: Ingestion failed for {}: {:?}", asset_symbol, e);
                log::error!("{}", error_msg);
                let _ = sender.send(TuiEvent::CsvIngestionAssetStatus(error_msg)).await;
                
                // Stop the loop and return the error
                return Err(e).context(format!("CRITICAL: Ingestion failed for asset {}", asset_symbol));
            }
        }
    }
    
    Ok(()) // All assets completed successfully
}


/// Handler for the CSV Ingestion mode.
pub async fn handle_key_event(key: KeyEvent, state: &mut AppState, sender: &Sender<TuiEvent>) -> Result<HandleAction> {
    match key.code {
        // Global navigation: Esc or 'D' returns to the Main Menu
        KeyCode::Esc | KeyCode::Char('d') | KeyCode::Char('D') => {
            state.active_mode = ActiveMode::MainMenu; 
            state.global.status_message = "Returned to Main Menu.".to_string();
        }
        
        // 'I' to Initiate the Ingestion Coordinator
        KeyCode::Char('i') | KeyCode::Char('I') => {
            if !state.global.is_loading {
                
                state.csv_ingest.log_history.clear();
                state.global.is_loading = true;
                state.global.status_message = "Starting multi-asset CSV ingestion... (Check terminal for detailed output).".to_string();
                
                let event_sender = sender.clone(); 
                
                // Spawn the long-running, looping ingestion task
                tokio::spawn(async move {
                    // Pass the sender to the worker
                    let result = run_ingestion_coordinator(event_sender.clone()).await; 
                    
                    // Send final completion message back to the TUI
                    let _ = event_sender.send(TuiEvent::CsvIngestionCompleted(result)).await;
                });
            }
        }
        _ => {}
    }
    
    Ok(HandleAction::Continue)
}