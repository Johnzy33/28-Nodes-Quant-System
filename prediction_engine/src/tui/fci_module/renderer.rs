use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, List, ListItem, ListState, Table, Row, Cell};
use shared_models::FciSignalOutput;
use crate::tui::state::AppState;
use crate::tui::fci_module::state::SignalModeOperation;

/// Top-level FCI dashboard renderer, dispatches based on operation mode
pub fn render_dashboard(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    match state.fci.operation_mode {
        SignalModeOperation::SingleAsset => render_single_asset_dashboard(f, state, area),
        SignalModeOperation::AllAssets => render_all_assets_dashboard(f, state, area),
    }
}

// Renders the modern split view (Asset list left, Signal details right)
fn render_single_asset_dashboard(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    // 1. Asset Selector (Left Panel)
    let asset_list_items: Vec<ListItem> = state.global.asset_list.iter()
        .map(|asset| {
            ListItem::new(format!("{} - {}", &asset.symbol, asset.name.as_deref().unwrap_or("Unknown Name")))
        })
        .collect();
        
    let asset_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" ASSETS ", Style::default().fg(Color::Cyan).bold()));
        
    let mut list_state = ListState::default();
    list_state.select(Some(state.global.selected_asset_index));
    f.render_stateful_widget(
        List::new(asset_list_items).block(asset_block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED).fg(Color::Cyan))
            .highlight_symbol(">> "), 
        chunks[0], 
        &mut list_state
    );

    // 2. Signal Summary (Right Panel)
    let summary_title = format!(" CURRENT FCI SIGNAL (Mode: Single Asset | 'T' to Toggle All) ");
    let summary_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(summary_title, Style::default().fg(Color::LightBlue)));
        
    let signal_text = if let Some(signal) = &state.fci.last_single_signal {
        render_fci_signal_text(signal)
    } else {
        vec![Line::from("No signal calculated yet. Select an asset and press ENTER.")]
    };
    
    let summary_paragraph = Paragraph::new(signal_text);
    f.render_widget(summary_block, chunks[1]);
    // Inner margin for clean look
    f.render_widget(summary_paragraph, chunks[1].inner(Margin::new(1, 1)));
}


// Renders the modern list view for all actionable assets (NOW A TABLE)
fn render_all_assets_dashboard(f: &mut Frame<'_>, state: &AppState, area: Rect) {
   
    
    let title = format!(" 🚀 ALL ASSETS FCI SIGNALS (Actionable | 'T' to Toggle Single) ");
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, Style::default().fg(Color::Cyan).bold()));
    
    // 0. Initial message check
    if state.fci.all_signals_list.is_empty() {
        let msg = if state.global.is_loading {
            "Running batch job (this may take a moment)..."
        } else {
            "Press 'R' to run the batch job for all assets and display top actionable signals."
        };
        let p = Paragraph::new(msg).alignment(Alignment::Center).fg(Color::Gray);
        f.render_widget(block, area);
        f.render_widget(p, area.inner(Margin::new(1, 1)));
        return;
    }

    // --- START TABLE RENDERING ---
    // 1. UPDATED HEADER (Matching your request)
    let header = [
        "ASSET", "DATE","PS2 NAME","PS1 NAME", "PS2 BIAS",  "PS1 BIAS", "CS", "PRED DAY", 
        "DIR", "FCI_SCORE",  "PCS", "TCS", "P.CONT", "VETOED","CONF" // P.CONT added here
    ];
    
    // 2. Create Rows (MAPPED TO NEW HEADER ORDER, LIFETIME FIX)
    let rows: Vec<Row> = state.fci.all_signals_list.iter().map(|signal| {
        let dir_style = match signal.signal_direction.as_str() {
            "Long" => Style::default().fg(Color::Green),
            "Short" => Style::default().fg(Color::Red),
            _ => Style::default().fg(Color::Yellow),
        };
        let veto_style = if signal.is_vetoed { Style::default().fg(Color::Red) } else { Style::default().fg(Color::Green) };
        let conf_style = match signal.signal_confidence.as_str() {
            "High" => Style::default().fg(Color::Green).bold(),
            "Medium" => Style::default().fg(Color::Yellow),
            _ => Style::default().fg(Color::Red),
        };
        
        Row::new(vec![
            // 1. ASSET
            Cell::from(signal.asset_id.as_deref().unwrap_or("N/A")),
            // 2. DATE
            Cell::from(signal.trading_date.map(|d| d.to_string()).unwrap_or_else(|| "N/A".to_string())), 

            // 3. PS2 NAME (Pattern)
            Cell::from(signal.ps2_name.as_deref().unwrap_or("-")),
            // 4. PS1 NAME (Pattern)
            Cell::from(signal.ps1_name.as_deref().unwrap_or("-")),
            
            // 5. PS2 BIAS (Pattern)
            Cell::from(signal.ps2_bias.as_deref().unwrap_or("-")).style(Style::default().fg(Color::DarkGray)),
            // 6. PS1 BIAS (Pattern)
            Cell::from(signal.ps1_bias.as_deref().unwrap_or("-")).style(Style::default().fg(Color::DarkGray)),
            
            // 7. CS NAME
            Cell::from(signal.cs_name.to_string()),
            // 8. PRED DAY
            Cell::from(signal.predicted_day_type.to_string()).style(Style::default().fg(Color::LightCyan)),
            
            // 9. DIR (Direction)
            Cell::from(signal.signal_direction.to_string()).style(dir_style), 
            // 10. FCI_SCORE
            Cell::from(format!("{:.2}", signal.fci_score)).style(Style::default().fg(Color::Yellow)),
           

            // 12. PCS (Metrics)
            Cell::from(format!("{:.1}", signal.pcs_anchor_score)).style(Style::default().fg(Color::LightBlue)),
            // 13. TCS (Metrics)
            Cell::from(format!("{:.2}", signal.tcs_multiplier)).style(Style::default().fg(Color::LightBlue)),
            // 14. P.CONT (P_Continuation_Raw) - Added after TCS
            Cell::from(signal.p_continuation_raw.map_or_else(|| "-".to_string(), |v| format!("{:.1}", v))).style(Style::default().fg(Color::LightCyan)),
            
            // 15. VETOED
            Cell::from(if signal.is_vetoed { "Yes" } else { "No" }).style(veto_style),
             // 11. CONF (Confidence)
            Cell::from(signal.signal_confidence.to_string()).style(conf_style),
        ])
    }).collect();

    // 3. Define Table and Layout (COMPACTED WIDTHS - Adjusting to new column count)
    // We now have 15 columns (14 visible + Min(0) padding)
    let widths = [
        Constraint::Percentage(10), // ASSET (Decreased slightly to fit others)
        Constraint::Length(10),     // DATE
        Constraint::Length(4),      // PS2 NAME
        Constraint::Length(4),      // PS1 NAME
        Constraint::Length(17),      // PS2 BIAS
        Constraint::Length(17),      // PS1 BIAS
        Constraint::Length(5),      // CS NAME
        Constraint::Length(17),      // PRED DAY
        Constraint::Length(7),      // DIR
        Constraint::Length(7),      // FCI_SCORE
        Constraint::Length(6),      // PCS
        Constraint::Length(6),      // TCS
        Constraint::Length(6),      // P.CONT (New length)
        Constraint::Length(5),      // VETOED
         Constraint::Length(17),      // CONF
        Constraint::Min(0)          // Padding
    ];

    let table = ratatui::widgets::Table::new(rows, widths)
        .header(
            Row::new(header)
                .style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray))
                .bottom_margin(1),
        )
        .block(block) 
        .column_spacing(1);
        
    // 4. Render Table
    f.render_widget(table, area);
    // --- END TABLE RENDERING ---
}


// --- Helper function to format FCI signal text (Modern Aesthetic) ---

fn render_fci_signal_text(signal: &FciSignalOutput) -> Vec<Line> {
    let dir_color = match signal.signal_direction.as_str() {
        "Long" => Color::Green,
        "Short" => Color::Red,
        _ => Color::Yellow,
    };
    
    // Modern Layout: Aligning key metrics
    let score_span = Span::styled(format!("{:.2}", signal.fci_score), Style::default().fg(Color::Yellow).bold());
    let direction_span = Span::styled(&signal.signal_direction, Style::default().fg(dir_color).bold());

    vec![
        Line::from(vec![
            Span::styled("Direction: ", Style::default().fg(Color::Gray)),
            direction_span,
            Span::raw(format!(" ({})", signal.predicted_day_type)),
            Span::raw(" | Score: ").fg(Color::Gray),
            score_span,
        ]),
        Line::from(format!("Confidence: {}", signal.signal_confidence)).fg(Color::Gray),
        Line::from(""),
        Line::from(format!("Pattern: {} ({}) -> {} ({}) -> {} (Current)",
            signal.ps2_name.as_deref().unwrap_or("N/A"),
            signal.ps2_bias.as_deref().unwrap_or("N/A"),
            signal.ps1_name.as_deref().unwrap_or("N/A"),
            signal.ps1_bias.as_deref().unwrap_or("N/A"),
            signal.cs_name,
        )).fg(Color::DarkGray),
        Line::from(""),
        Line::from(vec![
            Span::styled("Is Vetoed: ", Style::default().fg(Color::Gray)),
            Span::styled(
                if signal.is_vetoed { "YES" } else { "NO" },
                Style::default().fg(if signal.is_vetoed { Color::Red } else { Color::Green }).bold()
            ),
            Span::raw(" | P. Continuation (Raw %): ").fg(Color::Gray),
            Span::raw(format!("{:.2}", signal.p_continuation_raw.unwrap_or(0.0))).fg(Color::LightCyan),
        ]),
        Line::from(format!("PCS Anchor: {:.2} | TCS Multiplier: {:.2}", 
            signal.pcs_anchor_score, signal.tcs_multiplier
        )).fg(Color::DarkGray),
    ]
}