use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, List, ListItem};
use ratatui::layout::{Margin, Layout, Constraint};
use ratatui::text::{Text, Line, Span};
use crate::tui::state::AppState;

pub fn render_dashboard(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    let title = Span::styled(" ♻️ CORE DATA REFRESH PIPELINE ", Style::default().fg(Color::LightBlue).bold());
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title);

    f.render_widget(block, area);

    let inner_area = area.inner(Margin::new(1, 1));
    
    // 1. Define the layout: top for the center status message, bottom for the log history
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top: Status message line
            Constraint::Min(0),    // Bottom: Log history panel
        ])
        .split(inner_area);
        
    let status_area = chunks[0];
    let log_area = chunks[1];


    // --- RENDER STATUS MESSAGE (Top Line) ---

    let refresh_msg = if state.global.is_loading {
        // Use the real-time global status message for immediate feedback
        Line::from(vec![
            Span::styled("STATUS: ", Style::default().fg(Color::Yellow)),
            Span::raw(state.global.status_message.clone()), // Use the message sent from the worker
        ])
    } else {
        // Show the prompt to start
        Line::from(vec![
            Span::styled("STATUS: ", Style::default().fg(Color::Cyan)),
            Span::raw("Ready. Press "),
            Span::styled("R", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" to start the full data refresh and re-run all signals."),
        ])
    };
        
    let paragraph = Paragraph::new(refresh_msg).alignment(Alignment::Center);
    f.render_widget(paragraph, status_area);


    // --- RENDER LOG HISTORY (Main Panel) ---

    // Map the log history from the state into Ratatui ListItems
    let log_items: Vec<ListItem> = state.data_refresh.log_history // 💥 Use data_refresh state here
        .iter()
        .map(|m| {
            let style = if m.contains("STARTING MASTER") || m.contains("--- PHASE") {
                Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)
            } else if m.contains("SUCCESSFULLY COMPLETED") {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else if m.contains("ERROR") || m.contains("CRITICAL") {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Text::from(Line::from(Span::styled(m, style))))
        })
        .collect();

    // Create the List widget
    let log_list = List::new(log_items)
        .block(Block::default().borders(Borders::TOP).title("Pipeline Refresh Log"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD)); 
        
    // Calculate the index for scrolling to keep the latest message in view
    let scroll_index = if log_list.len() > log_area.height as usize {
        log_list.len() as usize - log_area.height as usize
    } else {
        0
    };


    f.render_stateful_widget(
        log_list, 
        log_area, 
        &mut ratatui::widgets::ListState::default().with_offset(scroll_index).with_selected(None)
    );
}