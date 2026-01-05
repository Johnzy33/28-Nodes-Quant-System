
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

// Replace these with your actual crate paths
use crate::symmetric_price_grid::orchestrator::SpgOrchestrator;
use crate::ui::render_dashboard;

// pub async fn run_tui(orchestrator: Arc<SpgOrchestrator>) -> Result<()> {
//     // 1. Terminal Initialization
//     enable_raw_mode()?;
//     let mut stdout = std::io::stdout();
//     execute!(stdout, EnterAlternateScreen)?;
//     let backend = CrosstermBackend::new(stdout);
//     let mut terminal = Terminal::new(backend)?;

//     // 2. Render Loop Control (10 FPS is plenty for a dashboard)
//     let mut tick_rate = interval(Duration::from_millis(100));

//     loop {
//         tick_rate.tick().await;

//         // 3. Draw the UI
//         terminal.draw(|f| {
//             // We iterate through all trackers managed by the orchestrator
//             // If you only want to see one, you can filter by asset_id
//             for entry in orchestrator.trackers.iter() {
//                 let tracker = entry.value();
//                 render_dashboard(f, tracker);
//                 // Note: If you have multiple assets, you'll need a way to 
//                 // switch between them (tabs or scrolling).
//                 break; // Just showing the first one for now
//             }
//         })?;

//         // 4. Input Handling
//         if event::poll(Duration::from_millis(0))? {
//             if let Event::Key(key) = event::read()? {
//                 match key.code {
//                     KeyCode::Char('q') => break, // Quit
//                     // You could add KeyCode::Tab here to cycle through assets
//                     _ => {}
//                 }
//             }
//         }
//     }

//     // 5. Restoration
//     disable_raw_mode()?;
//     execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
//     terminal.show_cursor()?;

//     Ok(())
// }

pub async fn run_tui(orchestrator: Arc<SpgOrchestrator>) -> Result<()> {
    // 1. Terminal Setup
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut tick_rate = tokio::time::interval(Duration::from_millis(100));
    let mut active_index = 0;

    loop {
        tick_rate.tick().await;

        // 2. Identify Available Assets
        // We collect keys every tick in case a new asset is added live
        let asset_keys: Vec<String> = orchestrator.trackers.iter()
            .map(|e| e.key().clone())
            .collect();

        // 3. Render
        terminal.draw(|f| {
            if asset_keys.is_empty() {
                let msg = "Waiting for bootstrapped assets...";
                f.render_widget(ratatui::widgets::Paragraph::new(msg), f.size());
            } else {
                // Bounds check for the index
                if active_index >= asset_keys.len() { active_index = 0; }
                
                let asset_id = &asset_keys[active_index];
                if let Some(tracker) = orchestrator.trackers.get(asset_id) {
                    // Call your ui.rs function
                    crate::ui::render_dashboard(f, tracker.value());
                }
            }
        })?;

        // 4. Input Handling (Arrow Keys to switch assets)
        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Right => {
                        if !asset_keys.is_empty() {
                            active_index = (active_index + 1) % asset_keys.len();
                        }
                    }
                    KeyCode::Left => {
                        if !asset_keys.is_empty() {
                            active_index = if active_index == 0 { asset_keys.len() - 1 } else { active_index - 1 };
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // 5. Restore Terminal on Exit
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// pub async fn run_tui(orchestrator: Arc<SpgOrchestrator>) -> Result<()> {
//     enable_raw_mode()?;
//     let mut stdout = std::io::stdout();
//     execute!(stdout, EnterAlternateScreen)?;
//     let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

//     let mut active_index = 0;

//     loop {
//         // 1. Get the list of available assets from the DashMap
//         let asset_keys: Vec<String> = orchestrator.trackers.iter()
//             .map(|e| e.key().clone())
//             .collect();

//         // 2. Draw the Dashboard
//         terminal.draw(|f| {
//             if asset_keys.is_empty() {
//                 let msg = "No assets bootstrapped yet. Check DB connection...";
//                 f.render_widget(ratatui::widgets::Paragraph::new(msg), f.size());
//             } else {
//                 // Safety: Ensure index is within bounds if an asset was removed
//                 let idx = active_index % asset_keys.len();
//                 let asset_id = &asset_keys[idx];
                
//                 if let Some(tracker) = orchestrator.trackers.get(asset_id) {
//                     crate::ui::render_dashboard(f, tracker.value());
//                 }
//             }
//         })?;

//         // 3. Handle Input (Blocking until a key is pressed or 100ms passes)
//         // This is the critical part for capturing arrows!
//         if event::poll(std::time::Duration::from_millis(100))? {
//             if let Event::Key(key) = event::read()? {
//                 match key.code {
//                     KeyCode::Char('q') => break,
//                     KeyCode::Right => {
//                         if !asset_keys.is_empty() {
//                             active_index = (active_index + 1) % asset_keys.len();
//                         }
//                     }
//                     KeyCode::Left => {
//                         if !asset_keys.is_empty() {
//                             active_index = if active_index == 0 { 
//                                 asset_keys.len() - 1 
//                             } else { 
//                                 active_index - 1 
//                             };
//                         }
//                     }
//                     _ => {}
//                 }
//             }
//         }
//     }

//     disable_raw_mode()?;
//     execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
//     Ok(())
// }