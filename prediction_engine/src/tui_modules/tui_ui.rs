use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Paragraph, List, ListItem},
    Frame,
};
use super::tui_state::{AppState, Mode};

/// The UI drawing function.
pub fn ui(frame: &mut Frame, state: &mut AppState) {
    let size = frame.size();
    
    // Define main layout: Header (Fixed), Content (Min), Footer (Fixed)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header 
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Footer 
        ])
        .split(size);

    // --- Header Block ---
    let header_title = format!("🔮 Analysis Dashboard | Mode: {:?}", state.mode);
    let header_block = Block::default()
        .title(Line::from(header_title).style(Style::default().bold()))
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(header_block, chunks[0]);

    // --- Content Block (Metrics and Controls) ---
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // Main Panel (List/Report)
            Constraint::Percentage(30), // Status/Controls Panel
        ])
        .split(chunks[1]);

    // --- 1. Main Panel Content (Dynamic) ---
    let main_panel_title: String;
    let main_content_widget: Paragraph;

    match state.mode {
        Mode::Dashboard => {
            main_panel_title = "Shell Core Dashboard".to_string();
            main_content_widget = Paragraph::new(state.report_content.iter().map(|s| Line::from(s.clone())).collect::<Vec<Line>>());
        }
        Mode::TcsAssetSelection => {
            main_panel_title = "TCS Analysis: Step 1/5 - Select Asset".to_string();
            
            let list_items: Vec<ListItem> = state.assets.iter().map(|asset| {
                let name = asset.name.as_ref().map(|n| format!(" ({})", n)).unwrap_or_default();
                ListItem::new(format!("{} - {}", asset.symbol, name))
            }).collect();

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title(Line::from(main_panel_title.clone())))
                .highlight_style(Style::default().bg(Color::Blue).bold())
                .highlight_symbol(">> ");
            
            frame.render_stateful_widget(list, content_chunks[0], &mut state.asset_list_state);
            return; 
        }
        Mode::TcsOrderSelection => {
            main_panel_title = format!("TCS Analysis: Step 2/5 - Select Order ({})", state.selected_asset_symbol.as_deref().unwrap_or("N/A"));
            
            let list_items: Vec<ListItem> = state.order_options.iter().map(|option| {
                ListItem::new(format!("{} - {}", option.label, option.description)).style(Style::default().fg(Color::LightBlue))
            }).collect();

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title(Line::from(main_panel_title.clone())))
                .highlight_style(Style::default().bg(Color::Blue).bold())
                .highlight_symbol(">> ");
            
            frame.render_stateful_widget(list, content_chunks[0], &mut state.order_list_state);
            return;
        }
        Mode::TcsChainSelection => {
            main_panel_title = format!("TCS Analysis: Step 3/5 - Select Chain ({})", state.selected_asset_symbol.as_deref().unwrap_or("N/A"));
            
            let list_items: Vec<ListItem> = state.chain_options.iter().map(|option| {
                ListItem::new(format!("{} - {}", option.label, option.description)).style(Style::default().fg(Color::LightBlue))
            }).collect();

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title(Line::from(main_panel_title.clone())))
                .highlight_style(Style::default().bg(Color::Blue).bold())
                .highlight_symbol(">> ");
            
            frame.render_stateful_widget(list, content_chunks[0], &mut state.chain_list_state);
            return;
        }
        Mode::TcsBiasInput => {
            main_panel_title = "TCS Analysis: Step 4/5 - Set Session Biases".to_string();
            
            // Layout split for the main panel: [Selection Area] [Info Area]
            let bias_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(80), // Selection Area (Session Fields & Bias States)
                    Constraint::Percentage(20), // Info Area
                ])
                .split(content_chunks[0]);
                
            let selection_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40), // Session Fields List
                    Constraint::Percentage(60), // Bias States List
                ])
                .split(bias_chunks[0]);

            // --- A. Draw Session Fields List (PS, CS, PS2, etc.) ---
            let session_items: Vec<ListItem> = state.bias_fields_order.iter().map(|field_name| {
                let current_bias = state.bias_input.get(field_name).cloned().unwrap_or("...".to_string());
                let text = format!("{}: {}", field_name, current_bias);
                let style = if field_name == &state.active_session_field {
                    Style::default().fg(Color::Yellow).bold()
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(text).style(style)
            }).collect();
            
            let session_list = List::new(session_items)
                .block(Block::default().title("Sessions (TAB to switch active)").borders(Borders::ALL))
                .highlight_style(Style::default().fg(Color::Green).bold()); 

            frame.render_widget(session_list, selection_chunks[0]);


            // --- B. Draw Bias States List (Bullish, Consolidation, etc.) ---
            let bias_state_items: Vec<ListItem> = super::tui_state::BIAS_STATE_OPTIONS.iter().map(|&s| {
                ListItem::new(s).style(Style::default().fg(Color::LightBlue))
            }).collect();

            let bias_list = List::new(bias_state_items)
                .block(Block::default().title("Bias State Options (↑/↓ to select, ENTER to apply)").borders(Borders::ALL))
                .highlight_style(Style::default().bg(Color::Blue).bold())
                .highlight_symbol(">> ");
                
            frame.render_stateful_widget(bias_list, selection_chunks[1], &mut state.bias_list_state);
            
            
            // --- C. Draw Info Area ---
            let info_panel = Paragraph::new(state.report_content.iter().map(|s| Line::from(s.clone())).collect::<Vec<Line>>())
                .block(Block::default().title("Context").borders(Borders::ALL));
            
            frame.render_widget(info_panel, bias_chunks[1]);
            
            return;
        }

        Mode::TcsReportView => {
            main_panel_title = format!("TCS Report: {}", state.selected_asset_symbol.as_deref().unwrap_or("N/A"));
            main_content_widget = Paragraph::new(state.report_content.iter().map(|s| Line::from(s.clone())).collect::<Vec<Line>>());
        }
        Mode::QueryMenu => {
            main_panel_title = "Data Management: Query Menu".to_string();
            main_content_widget = Paragraph::new(vec![Line::from("Query Menu (Planned)")]);
        }
    }
    
    // Only renders the Paragraph if a List or Input wasn't rendered above
    let main_panel = main_content_widget.wrap(ratatui::widgets::Wrap { trim: false })
        .block(Block::default().title(Line::from(main_panel_title)).borders(Borders::ALL));
    frame.render_widget(main_panel, content_chunks[0]);


    // --- 2. Status/Controls Panel ---
    
    let current_asset = state.selected_asset_symbol.as_deref().unwrap_or("[NONE]");
    let current_asset_id = state.selected_asset_id.as_deref().unwrap_or("N/A");
    
    let mut status_text = vec![
        Line::from(format!("Database: Connected")).green(),
        Line::from("--- Parameters ---").blue(),
        Line::from(format!("Asset: {} ({})", current_asset, current_asset_id)).yellow().bold(),
        Line::from(format!("Order: {}", state.selected_order_type.as_deref().unwrap_or("N/A"))).yellow(),
        Line::from(format!("Chain: {}", state.selected_chain.as_deref().unwrap_or("N/A"))).yellow(),
        Line::from(format!("Date: {}", state.analysis_date_str)).yellow().bold(),
        Line::from("--- Shell Core Controls ---").blue(),
        Line::from("[T] Start TCS Analysis"),
        Line::from("[P] PCS Analysis (Planned)"),
        Line::from("[M] Data Management (Planned)"),
        Line::from("[D] Dashboard View"),
    ];

    // Contextual Controls
    if state.mode == Mode::TcsAssetSelection || state.mode == Mode::TcsOrderSelection || state.mode == Mode::TcsChainSelection {
        status_text.push(Line::from("--- List Selection ---").magenta());
        status_text.push(Line::from("[↑/↓, j/k] Navigate"));
        status_text.push(Line::from("[Enter] Select Item"));
    } else if state.mode == Mode::TcsReportView {
        status_text.push(Line::from("--- Report Controls ---").magenta());
        let rerun_line = Line::from("[R] Re-run Report").green().bold();
        status_text.push(rerun_line);
    } else if state.mode == Mode::TcsBiasInput {
        status_text.push(Line::from("--- Bias Input ---").magenta());
        status_text.push(Line::from("[TAB] Switch Session Field"));
        status_text.push(Line::from("[↑/↓] Select Bias State"));
        status_text.push(Line::from("[Enter] Apply Selected Bias"));
        let confirm_line = Line::from("[F] Finalize Biases").green().bold();
        status_text.push(confirm_line);
    }
    
    status_text.push(Line::from("---").blue());
    status_text.push(Line::from("[Q/Esc] Quit Application"));
    

    let status_panel = Paragraph::new(status_text)
        .block(Block::default().title("Status & Controls").borders(Borders::ALL));
    frame.render_widget(status_panel, content_chunks[1]);

    // --- Footer Block ---
    let footer_text = Paragraph::new(Line::from(">> Ready for input."))
        .block(Block::default().borders(Borders::TOP))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer_text, chunks[2]);
}