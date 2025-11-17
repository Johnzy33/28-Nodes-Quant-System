// prediction_engine/src/tui/fci_module/state.rs

use shared_models::FciSignalOutput;
use chrono::NaiveDate;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SignalModeOperation {
    SingleAsset, // Default for Dashboard
    AllAssets,   // A specific command to run the batch query
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InputFocus {
    AssetSelector, // Focus when navigating the left-hand asset list
    // Backtesting focus modes would go here later
}

pub struct FciState {
    pub operation_mode: SignalModeOperation,
    pub focus: InputFocus,
    pub last_single_signal: Option<FciSignalOutput>, // Result for single view
    pub all_signals_list: Vec<FciSignalOutput>,      // Result for all-assets view
    // Backtesting input params go here later
    // pub backtest_params: BacktestParams, 
}

impl Default for FciState {
    fn default() -> Self {
        Self {
            operation_mode: SignalModeOperation::SingleAsset,
            focus: InputFocus::AssetSelector,
            last_single_signal: None,
            all_signals_list: Vec::new(),
        }
    }
}
