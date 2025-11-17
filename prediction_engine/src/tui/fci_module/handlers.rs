// prediction_engine/src/tui/fci_module/handlers.rs

use anyhow::Result;
use crossterm::event::KeyCode;
use crate::queries;
use crate::tui::state::AppState;
// Import the enum specifically
use crate::tui::fci_module::state::SignalModeOperation;
use crate::tui::events::HandleAction; // <-- ADD THIS LINE

// Define possible outcomes of an event handler


/// Handles all events when the active mode is ActiveMode::FciSystem
pub async fn handle_fci_events(key_code: KeyCode, app: &mut AppState) -> Result<HandleAction> {
    match key_code {
        // Global escape key to return to the main menu
        KeyCode::Esc => {
            app.active_mode = crate::tui::state::ActiveMode::MainMenu;
            Ok(HandleAction::Continue)
        }

        // FCI Dashboard Specific Interactions
        KeyCode::Up => { app.select_previous_asset(); Ok(HandleAction::Continue) }
        KeyCode::Down => { app.select_next_asset(); Ok(HandleAction::Continue) }
        
        KeyCode::Enter => {
            // Trigger signal fetch for the single asset view
            if app.fci.operation_mode == SignalModeOperation::SingleAsset {
                trigger_single_fci_fetch(app).await?;
            }
            Ok(HandleAction::Continue)
        }

        KeyCode::Char('t') => {
            // Correct the match arm usage by fully qualifying the enum variants:
            app.fci.operation_mode = match app.fci.operation_mode { 
                SignalModeOperation::SingleAsset => SignalModeOperation::AllAssets, 
                SignalModeOperation::AllAssets => SignalModeOperation::SingleAsset, 
            };
            
            // Clear results when switching view modes
            app.fci.all_signals_list.clear();
            app.fci.last_single_signal = None;
            Ok(HandleAction::Continue)
        }

        KeyCode::Char('r') => {
            // Trigger batch job only when in all-assets view mode
            if app.fci.operation_mode == SignalModeOperation::AllAssets {
                trigger_all_fci_fetch(app).await?;
            }
            Ok(HandleAction::Continue)
        }
        
        _ => Ok(HandleAction::Continue),
    }
}

// --- Async Database Triggers ---

async fn trigger_single_fci_fetch(app: &mut AppState) -> Result<()> {
    app.global.is_loading = true;
    app.global.status_message = "Fetching single asset FCI signal...".to_string();
    
    if let Some(asset_id) = app.get_selected_asset_id() {
        match queries::fetch_single_fci_signal(&app.global.pool, &asset_id, None).await {
            Ok(signal) => {
                app.fci.last_single_signal = signal;
                app.global.status_message = "Signal fetch complete.".to_string();
            }
            Err(e) => {
                app.global.status_message = format!("DB Error: {}", e).to_string();
            }
        }
    } else {
        app.global.status_message = "No asset selected.".to_string();
    }
    app.global.is_loading = false;
    Ok(())
}

async fn trigger_all_fci_fetch(app: &mut AppState) -> Result<()> {
    app.global.is_loading = true;
    app.global.status_message = "Running batch FCI signal calculation...".to_string();

    match queries::fetch_all_fci_signals(&app.global.pool).await {
        Ok(signals) => {
            app.fci.all_signals_list = signals;
            app.global.status_message = "Batch job complete.".to_string();
        }
        Err(e) => {
            // CRITICAL CHANGE: Print the full error to your terminal's stderr
            eprintln!("Database Error on fetch_all_fci_signals: {:?}", e); 
            app.global.status_message = format!("DB Error: {}", e).to_string();
            // You may want to set db_connected = false here if it's a fatal error
        }
    }
    app.global.is_loading = false;
    Ok(())
}
