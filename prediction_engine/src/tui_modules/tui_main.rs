use anyhow::{Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io::{self, Stdout};
use std::time::Duration;
use chrono::Datelike; 
use crate::cli_modules::core_logic::DbExecutor;
use super::tui_state::{AppState, Mode}; 
use super::tui_ui::ui; 

/// Type alias for the terminal backend
type CrosstermTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Initializes the terminal, enables raw mode, and enters the alternate screen.
fn setup_terminal() -> Result<CrosstermTerminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(|e| e.into())
}

/// Restores the terminal to its original state.
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

/// Runs the main TUI application loop.
pub async fn run_tui(executor: DbExecutor) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let res = run_app(&mut terminal, executor).await;
    restore_terminal()?;
    if let Err(err) = res {
        eprintln!("{:?}", err)
    }
    Ok(())
}

/// The core application loop, handling drawing and events.
async fn run_app(terminal: &mut CrosstermTerminal, executor: DbExecutor) -> Result<()> {
    let mut state = AppState::default(); 

    // Fetch Asset List (Query A2) on startup
    match executor.fetch_asset_list().await {
        Ok(assets) => {
            state.assets = assets;
            if !state.assets.is_empty() {
                state.asset_list_state.select(Some(0));
            }
        }
        Err(e) => {
            state.report_content = vec![format!("[DB ERROR] Failed to load assets: {}", e)];
        }
    }

    loop {
        // 1. Draw UI
        terminal.draw(|f| ui(f, &mut state))?; 

        // 2. Handle Events
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('d') => state.mode = Mode::Dashboard,
                    
                    // Start TCS Analysis
                    KeyCode::Char('t') => { 
                        if !state.assets.is_empty() {
                            state.mode = Mode::TcsAssetSelection 
                        } else {
                             state.report_content = vec!["Cannot start TCS Analysis: No assets loaded.".to_string()];
                        }
                    },

                    // List Navigation (Up/K and Down/J)
                    KeyCode::Up | KeyCode::Char('k') => {
                        match state.mode {
                            Mode::TcsAssetSelection => state.previous_asset(),
                            Mode::TcsOrderSelection => state.previous_order(),
                            Mode::TcsChainSelection => state.previous_chain(),
                            Mode::TcsBiasInput => state.previous_bias_state(), // Navigate BIAS_STATE_OPTIONS
                            _ => {}
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        match state.mode {
                            Mode::TcsAssetSelection => state.next_asset(),
                            Mode::TcsOrderSelection => state.next_order(),
                            Mode::TcsChainSelection => state.next_chain(),
                            Mode::TcsBiasInput => state.next_bias_state(), // Navigate BIAS_STATE_OPTIONS
                            _ => {}
                        }
                    }
                    
                    // Confirmation/Selection (Enter)
                    KeyCode::Enter => {
                        match state.mode {
                            Mode::TcsAssetSelection => state.select_asset(),
                            Mode::TcsOrderSelection => state.select_order(),
                            Mode::TcsChainSelection => state.select_chain(),
                            Mode::TcsBiasInput => state.select_bias_state(), // Selects current bias and moves to next field
                            _ => {}
                        }
                    }
                    
                    // BIAS INPUT HANDLING (STEP 4)
                    KeyCode::Tab => {
                        if state.mode == Mode::TcsBiasInput {
                            state.next_session_field(); // Switches active session field (e.g., PS1 -> PS2)
                        }
                    }
                    
                    // Finalize Bias Input (F)
                    KeyCode::Char('f') => {
                         if state.mode == Mode::TcsBiasInput {
                             state.confirm_bias_input(); 
                         }
                    }
                    
                    // TcsReportView (Step 5 - Run Report: R)
                    KeyCode::Char('r') => {
                        if state.mode == Mode::TcsReportView {
                            if let (Some(id), Some(symbol), Some(order_type)) = (&state.selected_asset_id, &state.selected_asset_symbol, &state.selected_order_type) {
                                
                                // 1. Parse date string into NaiveDate
                                let date = match chrono::NaiveDate::parse_from_str(&state.analysis_date_str, "%Y-%m-%d") {
                                    Ok(d) => d,
                                    Err(_) => {
                                        state.report_content = vec!["[ERROR] Invalid date format. Please correct in Dashboard.".to_string()];
                                        return Ok(());
                                    }
                                };
                                    
                                // 2. --- Extract PS1 and optional PS2 biases from HashMap ---
                                // PS1 is required for all *single-asset* analysis types.
                                let ps1_bias_input = state.bias_input.get("PS1")
                                    .cloned()
                                    .or_else(|| state.bias_input.get("ASSET_1_PS").cloned()) // Fallback for Crossed Asset
                                    .unwrap_or_else(|| {
                                        // This should only happen if state.confirm_bias_input failed validation somehow
                                        state.report_content = vec!["[ERROR] PS1 Bias not set in state.".to_string()];
                                        "Consolidation".to_string() 
                                    });
                                
                                // PS2 bias is extracted only if the Order Type explicitly requires it.
                                let ps2_bias_input = match order_type.as_str() {
                                    "2nd Order" | "Crossed Order" => state.bias_input.get("PS2").cloned(),
                                    "Crossed Asset" => state.bias_input.get("ASSET_2_PS").cloned(),
                                    _ => None, // 1st Order does not pass PS2
                                };
                                
                                // Execute the database report generation
                                let selected_chain = state.selected_chain.as_deref().unwrap_or("TCS_DEFAULT").to_string();
                                match executor.generate_tcs_report(
                                    symbol.clone(), 
                                    id.clone(), 
                                    date, 
                                    ps1_bias_input, 
                                    ps2_bias_input,
                                    selected_chain,
                                ).await {
                                    Ok(report) => {
                                        // Update state with the final report content
                                        state.report_content = report.lines().map(|s| s.to_owned()).collect();
                                    }
                                    Err(e) => {
                                        state.report_content = vec![
                                            format!("[ERROR] Failed to run report: {}", e),
                                            format!("Details: {:?}", e),
                                        ];
                                    }
                                }
                            } else {
                                state.report_content = vec!["[ERROR] Asset or Order not fully selected. Returning to selection.".to_string()];
                                state.mode = Mode::TcsAssetSelection; 
                            }
                        }
                    }
                    
                    // Date Input Handling (No need for full character input logic)
                    KeyCode::Char(_c) => {
                        // Assuming date input/editing is handled elsewhere or is not the current issue
                    }
                    _ => {}
                }
            }
        }
    }
}