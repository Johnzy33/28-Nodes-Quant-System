// prediction_engine/src/tui/renderer.rs

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, List, ListItem};
use crate::tui::state::{AppState, ActiveMode};
use crate::tui::fci_module::renderer as fci_renderer; // Use a sub-module for FCI rendering
use crate::tui::data_refresh_module::renderer as refresh_renderer; // ADD THIS IMPORT
use crate::tui::csv_ingest_module::renderer as csv_renderer; // <-- ADD THIS IMPORT

/// The primary drawing function that dispatches to specific system renderers.
pub fn render_ui(f: &mut Frame<'_>, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main Content
            Constraint::Length(3), // Footer (Status Bar)
        ])
        .split(f.area());

    render_header(f, state, chunks[0]);
    render_footer(f, state, chunks[2]);

    // Dispatch rendering to the active mode's specific renderer
    match state.active_mode {
        ActiveMode::MainMenu => render_main_menu(f, state, chunks[1]),
        ActiveMode::FciSystem => fci_renderer::render_dashboard(f, state, chunks[1]),
        ActiveMode::DataRefresh => refresh_renderer::render_dashboard(f, state, chunks[1]), // UPDATE THIS LINE
        ActiveMode::CsvIngest => csv_renderer::render_dashboard(f, state, chunks[1]),
        ActiveMode::DcsSystem => { /* render_dcs_view(f, state, chunks[1]) */ }
        _ => {}
    }
}

// --- Shared Renderers (Header, Footer, Main Menu) ---

fn render_header(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    let title = format!(" | Trading System CLI | Mode: {:?} |", state.active_mode);
    let help_text = " Global: (D)ash | (F)ci | (S)core | (Q)uit ";

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let title_block = Block::default()
        .borders(Borders::BOTTOM)
        .title(title)
        .title_style(Style::default().fg(Color::LightBlue).bold());

    f.render_widget(title_block, area);

    let help_paragraph = Paragraph::new(Line::from(Span::raw(help_text).fg(Color::Gray)))
        .alignment(Alignment::Right);
        
    f.render_widget(help_paragraph, header_chunks[1]);
}

fn render_footer(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    let status_style = if state.global.is_loading {
        Style::default().bg(Color::Yellow).fg(Color::Black).bold()
    } else {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    };

    let db_status_color = if state.global.db_connected { Color::Green } else { Color::Red };
    let db_status = Span::styled(" [DB Status] ", Style::default().bg(db_status_color).fg(Color::Black)).bold();
    let status_message = Span::raw(format!("STATUS: {}", state.global.status_message));
    let status_line = Line::from(vec![db_status, status_message]);
    
    let status = Paragraph::new(status_line)
        .block(Block::default().borders(Borders::TOP))
        .style(status_style);

    f.render_widget(status, area);
}

fn render_main_menu(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Main Dashboard (Hub) ", Style::default().fg(Color::Cyan).bold()));
    
    let items: Vec<ListItem> = state.main_menu.menu_items.iter()
        .map(|&i| ListItem::new(i))
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED).fg(Color::Cyan))
        .highlight_symbol(">> ");

    // We need a ListState for the main menu as well (assume one exists in MainMenuState if we follow strict modularity, but here we mock it)
    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(state.main_menu.selected_index));

    f.render_stateful_widget(list, area, &mut list_state);
}
