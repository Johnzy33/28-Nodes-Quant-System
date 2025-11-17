// prediction_engine/src/tui/state.rs

use chrono::NaiveDate;
use database_engine::runtime::Pool; 
use crate::tui::fci_module::state::FciState;
use crate::tui::data_refresh_module::state::DataRefreshState; // ADD THIS IMPORT 
use  crate::tui::csv_ingest_module::state::CsvIngestState;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ActiveMode {
    MainMenu,
    FciSystem,
    DcsSystem,
    DataRefresh,
    QueryMenu,
    CsvIngest, // <--- 2. ADD THIS VARIANT,
}

pub struct MainMenuState { pub selected_index: usize, pub menu_items: Vec<&'static str>, }
pub struct DcsState { pub analysis_date: NaiveDate, }

pub struct GlobalState {
    pub pool: Pool, // Use the consistent Pool type
    pub asset_list: Vec<shared_models::AssetInfo>,
    pub selected_asset_index: usize,
    pub is_loading: bool,
    pub status_message: String,
    pub db_connected: bool,
}

pub struct AppState {
    pub active_mode: ActiveMode,
    pub global: GlobalState,
    pub main_menu: MainMenuState,
    pub fci: FciState,
    pub dcs: DcsState,
    pub data_refresh: DataRefreshState, // ADD THIS FIELD
    pub csv_ingest: CsvIngestState, // <--- 3. ADD THIS FIELD
}

impl AppState {
    pub fn new(pool: Pool) -> Self { // Function signature uses the consistent Pool type
        let today = NaiveDate::from_ymd_opt(2025, 11, 11).unwrap();
        Self {
            active_mode: ActiveMode::MainMenu,
            global: GlobalState {
                pool, asset_list: Vec::new(), selected_asset_index: 0, is_loading: false, status_message: "Initializing...".to_string(), db_connected: true,
            },
            main_menu: MainMenuState { selected_index: 0, menu_items: vec!["FCI Signal System", "DCS Score System", "Refresh Core Pipeline", "Query Metrics", "Ingest CSV"], },
            fci: FciState::default(), dcs: DcsState { analysis_date: today },
            data_refresh: DataRefreshState::default(), // INITIALIZE HERE
            csv_ingest: CsvIngestState::default(), // <--- 3. ADD THIS FIELD
        }
    }
    pub fn get_selected_asset_id(&self) -> Option<String> { self.global.asset_list.get(self.global.selected_asset_index).map(|a| a.id.clone()) }
    pub fn select_next_asset(&mut self) { if !self.global.asset_list.is_empty() { self.global.selected_asset_index = (self.global.selected_asset_index + 1) % self.global.asset_list.len(); } }
    pub fn select_previous_asset(&mut self) { if !self.global.asset_list.is_empty() { if self.global.selected_asset_index == 0 { self.global.selected_asset_index = self.global.asset_list.len() - 1; } else { self.global.selected_asset_index -= 1; } } }
}
