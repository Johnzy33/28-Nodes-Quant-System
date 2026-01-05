use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use crate::price_grid::{GridLayer, PriceLevel};
use shared_models::{data_model as dm, get_trading_session};
use chrono::{DateTime, Utc, TimeZone, Datelike};
use shared_models::session_utils::TradingSession;
use crate::spg_tracker::SpgTracker;



// pub fn render_dashboard(f: &mut Frame, tracker: &SpgTracker) {
//     // 1. Setup Layout
//     let chunks = Layout::default()
//         .direction(Direction::Vertical)
//         .margin(1)
//         .constraints([
//             Constraint::Length(3), // Header
//             Constraint::Min(10),   // SPG Table
//             Constraint::Length(3), // Footer
//         ].as_ref())
//         .split(f.size());

//     // 2. Header: Asset and Price
//     let price_color = if tracker.last_price > 0.0 { Color::Green } else { Color::Yellow };
//     let header_block = Block::default()
//         .title(format!(" 📈 SPG LIVE: {} ", tracker.asset_id))
//         .borders(Borders::ALL)
//         .border_style(Style::default().fg(Color::Cyan));
    
//     let header_content = format!(" LIVE PRICE: {:.2} | CURRENT SESSION: {:?}", 
//         tracker.last_price, 
//         get_trading_session(chrono::Utc::now().timestamp_millis() as i64).unwrap_or(TradingSession::Unknown)
//     );
//     f.render_widget(ratatui::widgets::Paragraph::new(header_content).block(header_block), chunks[0]);

//     // 3. SPG Table
//     let header_cells = ["LAYER", "CONTEXT", "P1.00 (H)", "P0.50 (M)", "P0.00 (L)", "RANGE", "STATUS"]
//         .iter()
//         .map(|h| Cell::from(*h).style(Style::default().fg(Color::Gray)));
    
//     let header_row = Row::new(header_cells).height(1).bottom_margin(1);

//     let display_order = vec![
//         GridLayer::Yearly, GridLayer::PW2, GridLayer::PW1, GridLayer::PW,
//         GridLayer::PreviousDay, GridLayer::PS2, GridLayer::PS1, GridLayer::CS,
//     ];

//     let rows = display_order.iter().filter_map(|layer| {
//         tracker.active_grids.get(layer).map(|grid| {
//             let h = grid.get_price(PriceLevel::One).unwrap_or(grid.spg.high);
//             let m = grid.get_price(PriceLevel::MidPoint).unwrap_or((h + grid.spg.low) / 2.0);
//             let l = grid.get_price(PriceLevel::Zero).unwrap_or(grid.spg.low);
            
//             // Logic for Status and Coloring
//             let mut status = "INSIDE";
//             let mut row_style = Style::default();

//             if tracker.last_price > h {
//                 status = "↑ ABOVE";
//                 row_style = row_style.fg(Color::Green);
//             } else if tracker.last_price < l {
//                 status = "↓ BELOW";
//                 row_style = row_style.fg(Color::Red);
//             }

//             // Bold the Current Session row
//             if *layer == GridLayer::CS {
//                 row_style = row_style.add_modifier(Modifier::BOLD).bg(Color::Rgb(30, 30, 30));
//             }

//             Row::new(vec![
//                 Cell::from(format!("{:?}", layer)),
//                 Cell::from(derive_context_string(layer, grid.start_timestamp)),
//                 Cell::from(format!("{:.2}", h)),
//                 Cell::from(format!("{:.2}", m)),
//                 Cell::from(format!("{:.2}", l)),
//                 Cell::from(format!("{:.2}", grid.spg.range_distance)),
//                 Cell::from(status),
//             ]).style(row_style)
//         })
//     });

//     let table = Table::new(rows, [
//         Constraint::Percentage(15),
//         Constraint::Percentage(15),
//         Constraint::Percentage(14),
//         Constraint::Percentage(14),
//         Constraint::Percentage(14),
//         Constraint::Percentage(14),
//         Constraint::Percentage(14),
//     ])
//         .header(header_row)
//         .block(Block::default().borders(Borders::ALL).title(" Symmetric Price Grid Levels "));

//     f.render_widget(table, chunks[1]);
// }

pub fn render_dashboard(f: &mut Frame, tracker: &SpgTracker) {
    // 1. Setup Layout (3 sections now)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // SPG Table
            Constraint::Length(3), // Footer (New)
        ])
        .split(f.size());

    // 2. Header: Dynamic Asset Title
    // Using tracker.asset_id here ensures the title updates when you switch assets
    let header_block = Block::default()
        .title(format!(" 📈 SPG LIVE: {} ", tracker.asset_id)) 
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    
    let header_content = format!(" LIVE PRICE: {:.2} | STATUS: {}", 
        tracker.last_price,
        if tracker.last_price > 0.0 { "ACTIVE" } else { "WAITING FOR TICK..." }
    );
    f.render_widget(ratatui::widgets::Paragraph::new(header_content).block(header_block), chunks[0]);

    // 3. SPG Table (Logic remains the same as your previous version)
    let header_cells = ["LAYER", "CONTEXT", "P1.00 (H)", "P0.50 (M)", "P0.00 (L)", "RANGE", "STATUS"]
        .iter().map(|h| Cell::from(*h).style(Style::default().fg(Color::Gray)));
    
    let header_row = Row::new(header_cells).height(1).bottom_margin(1);
    let display_order = vec![
        GridLayer::Yearly, GridLayer::PW2, GridLayer::PW1, GridLayer::PW,
        GridLayer::PreviousDay, GridLayer::PS2, GridLayer::PS1, GridLayer::CS,
    ];

    let rows = display_order.iter().filter_map(|layer| {
        tracker.active_grids.get(layer).map(|grid| {
            let h = grid.get_price(PriceLevel::One).unwrap_or(grid.spg.high);
            let m = grid.get_price(PriceLevel::MidPoint).unwrap_or((h + grid.spg.low) / 2.0);
            let l = grid.get_price(PriceLevel::Zero).unwrap_or(grid.spg.low);
            
            let (status, color) = if tracker.last_price > h { ("↑ ABOVE", Color::Green) }
                else if tracker.last_price < l { ("↓ BELOW", Color::Red) }
                else { ("INSIDE", Color::White) };

            let mut row_style = Style::default().fg(color);
            if *layer == GridLayer::CS {
                row_style = row_style.add_modifier(Modifier::BOLD).bg(Color::Rgb(40, 40, 40));
            }

            Row::new(vec![
                Cell::from(format!("{:?}", layer)),
                Cell::from(derive_context_string(layer, grid.start_timestamp)),
                Cell::from(format!("{:.2}", h)),
                Cell::from(format!("{:.2}", m)),
                Cell::from(format!("{:.2}", l)),
                Cell::from(format!("{:.2}", grid.spg.range_distance)),
                Cell::from(status),
            ]).style(row_style)
        })
    });

    let table = Table::new(rows, [
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(14),
        Constraint::Percentage(14),
        Constraint::Percentage(14),
        Constraint::Percentage(14),
        Constraint::Percentage(14),
    ])
    .header(header_row)
    .block(Block::default().borders(Borders::ALL).title(" Symmetric Price Grid Levels "));

    f.render_widget(table, chunks[1]);

    // 4. Footer: Navigation Controls
    let footer_content = " [Left/Right] Switch Asset | [Q] Quit Dashboard ";
    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    
    f.render_widget(
        ratatui::widgets::Paragraph::new(footer_content)
            .block(footer_block)
            .alignment(ratatui::layout::Alignment::Center), 
        chunks[2]
    );
}

fn derive_context_string(layer: &GridLayer, ts: u64) -> String {
    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ts as i64).unwrap_or_default();
    match layer {
        GridLayer::CS | GridLayer::PS1 | GridLayer::PS2 => {
            format!("{:?}", get_trading_session(ts as  i64).unwrap_or(TradingSession::Unknown))
        }
        _ => dt.format("%Y-%m-%d").to_string(),
    }
}