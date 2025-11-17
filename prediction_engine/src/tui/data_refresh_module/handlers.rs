use crossterm::event::{KeyCode, KeyEvent};
use crate::queries;
use crate::tui::state::{AppState, ActiveMode};
use crate::tui::events::TuiEvent; 
use crate::tui::events::HandleAction; 
use tokio::sync::mpsc::Sender;
use anyhow::Result;

// Now an async function that returns a Result
pub async fn handle_key_event(key: KeyEvent, state: &mut AppState, sender: &Sender<TuiEvent>) -> Result<HandleAction, anyhow::Error> {
    match key.code {
        // Global navigation: Esc or 'D' returns to the Main Menu
        KeyCode::Esc | KeyCode::Char('d') | KeyCode::Char('D') => {
            state.active_mode = ActiveMode::MainMenu;
            // Must explicitly return a HandleAction
            return Ok(HandleAction::Continue); 
        }
        
        // 'R' to Run the Data Refresh
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if !state.global.is_loading {
                
                // 💥 CRITICAL FIX 1: Clear the log history before starting a new job
                state.data_refresh.log_history.clear(); 
                
                let pool = state.global.pool.clone();
                let event_sender = sender.clone(); 

                state.global.is_loading = true;
                state.global.status_message = "Starting core data refresh...".to_string();
                
                tokio::spawn(async move {
                    // 💥 CRITICAL FIX 2: Pass the TUI sender into the database function
                    // We must update the signature of run_core_data_refresh in database_engine/queries.rs next!
                    let result: std::result::Result<(), anyhow::Error> = queries::run_core_data_refresh(&pool, event_sender.clone()).await; 
                    
                    // The completion event should still be sent at the end
                    let _ = event_sender.send(TuiEvent::DataRefreshCompleted(result)).await; 
                });
            }
        }
        _ => {}
    }
    // All paths must return a Result<HandleAction>
    Ok(HandleAction::Continue)
}