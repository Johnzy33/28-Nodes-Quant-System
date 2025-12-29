use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::time::Duration;
use tokio::sync::mpsc::{self, Sender, Receiver};
use dotenvy;

mod tui; 
use tui::state::{AppState, ActiveMode};
use tui::renderer::render_ui;
use tui::fci_module::handlers as fci_handlers; 
use tui::data_refresh_module::handlers as refresh_handlers; 
use tui::csv_ingest_module::handlers as csv_handlers; 
use tui::events::{TuiEvent, HandleAction};

use database_engine::runtime::setup_database_pool; 
use tui::queries; 

#[tokio::main]
async fn main() -> Result<()> {
    //  Terminal Setup
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    dotenvy::dotenv().ok(); 

    // Database Setup
    let pool = setup_database_pool().await?; 
    let mut app = AppState::new(pool); 


    // Fetch initial asset list asynchronously
    match queries::fetch_asset_list(&app.global.pool).await {
        Ok(assets) => {
            app.global.asset_list = assets;
            app.global.status_message = "Welcome! Press 'Enter' in the menu to select a system.".to_string();
        }
        Err(e) => {
            eprintln!("ERROR: Failed to fetch initial asset list: {:?}", e);
            app.global.status_message = format!("DB Error: {}", e).to_string();
            app.global.db_connected = false;
        }
    }
    
    // ASYNC TASK SETUP: MPSC Channel
    let (tx, mut rx): (Sender<TuiEvent>, Receiver<TuiEvent>) = mpsc::channel(100);

    //  ASYNC EVENT POLLING TASK (listens for key presses and ticks)
    tokio::spawn(poll_events_task(tx.clone()));

    //  Main Application Loop
    let result = run_app_async(&mut terminal, &mut app, tx.clone(), &mut rx).await;

    //  Terminal Teardown
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    terminal.show_cursor()?; 
    
    result
}

/// Task dedicated to polling crossterm events and sending them via the MPSC channel.
async fn poll_events_task(tx: Sender<TuiEvent>) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    loop {
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    tx.send(TuiEvent::Key(key)).await?;
                }
            }
        }
        tx.send(TuiEvent::Tick).await?;
    }
}

/// The main application loop, now driven by the MPSC receiver.
async fn run_app_async<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    event_sender: Sender<TuiEvent>,
    event_receiver: &mut Receiver<TuiEvent>,
) -> Result<()> {
    loop {
        //  Draw the UI
        terminal.draw(|f| render_ui(f, app))?;

        //  Wait for the next event from the channel
        let event = event_receiver.recv().await;
        
        match event {
            Some(TuiEvent::Quit) => break,
            Some(TuiEvent::Tick) => {}
            
            Some(TuiEvent::Key(key)) => { 
                if handle_key_input(key, app, &event_sender).await? == HandleAction::Quit { 
                    break;
                }
            }

            Some(TuiEvent::DataRefreshStatus(msg)) => {
                app.global.status_message = msg.clone(); 
                app.data_refresh.log_history.push(msg); 
            }
            
            //  Handle Asynchronous Task Completions
            Some(TuiEvent::DataRefreshCompleted(result)) => {
                app.global.is_loading = false;
                match result {
                    Ok(_) => {
                        app.global.status_message = "Core Data Refresh complete. Use 'R' in FCI view to calculate signals.".to_string();
                    }
                    Err(e) => {
                        app.global.status_message = format!("ERROR: Data Refresh failed: {e}");
                    }
                }
            }
            
            //  Handle CSV Ingestion Completion (Final Message)
            Some(TuiEvent::CsvIngestionCompleted(result)) => {
                app.global.is_loading = false;
                match result {
                    Ok(_) => {
                        let msg = "✅ Multi-Asset CSV Ingestion completed successfully.".to_string();
                        app.global.status_message = msg.clone();
                        app.csv_ingest.log_history.push(msg); // Append final success message to log
                    }
                    Err(e) => {
                        let msg = format!("❌ CRITICAL: CSV Ingestion failed: {e}");
                        app.global.status_message = msg.clone();
                        app.csv_ingest.log_history.push(msg); // Append final error message to log
                    }
                }
            }
            
            // Handle CSV Ingestion Status (Intermediate Asset Messages)
            Some(TuiEvent::CsvIngestionAssetStatus(msg)) => {
                // 1. Update the main status line for immediate visual feedback
                app.global.status_message = msg.clone(); 
                
                // 2. Append the message to the dedicated log history for the panel
                // This relies on the previous step where CsvIngestState was updated with log_history
                app.csv_ingest.log_history.push(msg); 
            }
            
            _ => {}
        }
    }
    Ok(())
}

/// Key Input Dispatcher: routes key presses to the correct module handler.
async fn handle_key_input(key: KeyEvent, app: &mut AppState, event_sender: &Sender<TuiEvent>) -> Result<HandleAction> {
    
    let key_code = key.code; 

    // 1. Global Quits
    if key_code == KeyCode::Char('q') {
        event_sender.send(TuiEvent::Quit).await?;
        return Ok(HandleAction::Quit); 
    }

    // 2. Global Return to Main Menu
    if key_code == KeyCode::Esc && app.active_mode != ActiveMode::MainMenu {
        app.active_mode = ActiveMode::MainMenu;
        app.global.status_message = "Returned to Main Menu.".to_string();
        return Ok(HandleAction::Continue);
    }

    // 3. Dispatch to Active Mode Handler
    match app.active_mode {
        ActiveMode::MainMenu => handle_main_menu_events(key, app).await, 
        ActiveMode::FciSystem => fci_handlers::handle_fci_events(key_code, app).await, 
        ActiveMode::DataRefresh => refresh_handlers::handle_key_event(key, app, event_sender).await, 
        
        // Dispatch to new CSV Ingest handler
      //  ActiveMode::CsvIngest => csv_handlers::handle_key_event(key, app, event_sender).await,
        
        ActiveMode::DcsSystem => { 
            app.global.status_message = format!("DCS System active. Key press: {:?}", key_code);
            Ok(HandleAction::Continue) 
        }
        _ => Ok(HandleAction::Continue),
    }
}

/// Handles navigation within the main application menu
async fn handle_main_menu_events(key: KeyEvent, app: &mut AppState) -> Result<HandleAction> { 
    match key.code { 
        KeyCode::Up => {
            if app.main_menu.selected_index > 0 {
                app.main_menu.selected_index -= 1;
            } else {
                app.main_menu.selected_index = app.main_menu.menu_items.len() - 1;
            }
        }
        KeyCode::Down => {
            if app.main_menu.selected_index < app.main_menu.menu_items.len() - 1 {
                app.main_menu.selected_index += 1;
            } else {
                app.main_menu.selected_index = 0;
            }
        }
        KeyCode::Enter => {
            match app.main_menu.selected_index {
                0 => app.active_mode = ActiveMode::FciSystem,
                1 => app.active_mode = ActiveMode::DcsSystem,
                2 => app.active_mode = ActiveMode::DataRefresh, 
                3 => app.active_mode = ActiveMode::QueryMenu,
                4 => app.active_mode = ActiveMode::CsvIngest,
                _ => {}
            }
        }
        _ => {}
    }
    Ok(HandleAction::Continue)
}