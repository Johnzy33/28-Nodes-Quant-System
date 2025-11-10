use ratatui::widgets::ListState;
use shared_models::asset_models::AssetInfo;
use std::collections::HashMap;
use chrono::Datelike; 

// --- Application State Structs ---

/// Defines the current primary view or analysis mode.
#[derive(Debug, PartialEq, Clone)]
pub enum Mode {
    Dashboard,
    TcsAssetSelection,
    TcsOrderSelection,
    TcsChainSelection,
    TcsBiasInput, 
    TcsReportView,
    QueryMenu,
}

// Helper struct for the Order Selection list
#[derive(Debug, Clone)]
pub struct OrderOption {
    pub label: &'static str,
    pub description: &'static str,
}

// Helper struct for the Chain Selection list
#[derive(Debug, Clone)]
pub struct ChainOption {
    pub label: &'static str,
    pub description: &'static str,
}

// Define the core bias options based on your function's output (7 states)
pub const BIAS_STATE_OPTIONS: [&str; 7] = [
    "Bullish", 
    "Bearish", 
    "Consolidation", 
    "Bullish_Reversal", 
    "Bearish_Reversal", 
    "Pure_Indecision", 
    "Other"
];

/// The central application state struct.
pub struct AppState {
    pub mode: Mode,
    
    // Global Data (Step 1)
    pub assets: Vec<AssetInfo>, 
    pub asset_list_state: ListState,
    
    // TCS Order Selection Data (Step 2)
    pub order_options: Vec<OrderOption>, 
    pub order_list_state: ListState,     
    
    // TCS Chain Selection Data (Step 3)
    pub chain_options: Vec<ChainOption>, 
    pub chain_list_state: ListState,     

    // Parameters for current analysis
    pub selected_asset_id: Option<String>,
    pub selected_asset_symbol: Option<String>,
    pub selected_order_type: Option<String>, 
    pub selected_chain: Option<String>,      
    pub analysis_date_str: String,
    
    // --- Step 4: Bias Input State (Selection-based) ---
    // Keys are session names (PS1, PS2, etc.), values are selected BIAS_STATE_OPTIONS
    pub bias_input: HashMap<String, String>, 
    pub bias_fields_order: Vec<String>,      // List of sessions to set bias for (e.g., PS1, PS2)
    
    pub bias_list_state: ListState,         // State for the selectable bias list
    pub active_session_field: String,       // Tracks which session (PS1/PS2) is being edited
    
    // Output content
    pub report_content: Vec<String>, 
}

impl Default for AppState {
    fn default() -> Self {
        let order_options = vec![
            OrderOption { label: "1st Order", description: "Primary service only (PS1)." },
            OrderOption { label: "2nd Order", description: "Secondary service only (PS2)." },
            OrderOption { label: "Crossed Order", description: "Both 1st and 2nd Order combined (Full Report)." },
            OrderOption { label: "Crossed Asset", description: "Analysis comparing two different assets." },
        ];
        let mut order_list_state = ListState::default();
        order_list_state.select(Some(0));

        // --- UPDATED CHAIN OPTIONS: Include all 5 sequential transitions ---
        let chain_options = vec![
            // All 5 sequential paths: PS1 -> CS
            ChainOption { label: "TCS_NYPM_AS", description: "Predict AS from NY PM (NYPM) context." },
            ChainOption { label: "TCS_AS_LN", description: "Predict London (LN) from Asia (AS) context." },
            ChainOption { label: "TCS_LN_NYAM", description: "Predict NY AM (NYAM) from London (LN) context." },
            ChainOption { label: "TCS_NYAM_NYL", description: "Predict NY Lunch (NYL) from NY AM (NYAM) context." },
            ChainOption { label: "TCS_NYL_NYPM", description: "Predict NY PM (NYPM) from NY Lunch (NYL) context." },
            
            ChainOption { label: "TCS_DEFAULT", description: "Dynamic context based on session_context." },
        ];
        let mut chain_list_state = ListState::default();
        chain_list_state.select(Some(0));
        
        // --- Step 4 Initialization ---
        // Default to the fields needed for 1st Order + a single bias input
        let bias_fields_order = vec!["PS1".to_string()]; 
        let mut bias_input = HashMap::new();
        for field in &bias_fields_order {
            bias_input.insert(field.clone(), BIAS_STATE_OPTIONS[0].to_string());
        }
        
        let mut bias_list_state = ListState::default();
        bias_list_state.select(Some(0));

        let default_active_session_field = bias_fields_order.get(0).cloned().unwrap_or_default();

        AppState {
            mode: Mode::Dashboard,
            assets: Vec::new(),
            asset_list_state: ListState::default(),
            
            order_options, 
            order_list_state, 
            
            chain_options,     
            chain_list_state,  

            selected_asset_id: None,
            selected_asset_symbol: None,
            selected_order_type: None,
            selected_chain: None,
            analysis_date_str: chrono::Local::now().date_naive().to_string(),
            
            bias_input,
            bias_fields_order, 
            bias_list_state,
            active_session_field: default_active_session_field, 
            
            report_content: vec!["Press 't' to start TCS Analysis.".to_string()],
        }
    }
}

impl AppState {
    /// Resets session fields (PS1, PS2) based on the selected Order Type.
    fn initialize_bias_input(&mut self) {
        let fields = match self.selected_order_type.as_deref() {
            Some("1st Order") => vec!["PS1".to_string()], 
            
            // --- CRITICAL FIX: Include both PS2 and PS1 for 2nd Order analysis ---
            Some("2nd Order") => vec!["PS2".to_string(), "PS1".to_string()], 
            
            Some("Crossed Order") => vec!["PS1".to_string(), "PS2".to_string()], 
            Some("Crossed Asset") => vec!["ASSET_1_PS".to_string(), "ASSET_2_PS".to_string()], 
            _ => vec!["PS1".to_string()],
        };

        self.bias_fields_order = fields;
        self.bias_input.clear();
        for field in &self.bias_fields_order {
            // Re-initialize all new fields to the first bias state option (Bullish)
            self.bias_input.insert(field.clone(), BIAS_STATE_OPTIONS[0].to_string());
        }
        self.active_session_field = self.bias_fields_order.get(0).cloned().unwrap_or_default();
        self.bias_list_state.select(Some(0)); 
    }
    
    pub fn next_asset(&mut self) {
        if self.assets.is_empty() { return; }
        let i = self.asset_list_state.selected().map_or(0, |i| (i + 1) % self.assets.len());
        self.asset_list_state.select(Some(i));
    }
    
    pub fn previous_asset(&mut self) {
        if self.assets.is_empty() { return; }
        let i = self.asset_list_state.selected().map_or(self.assets.len() - 1, |i| {
            if i == 0 { self.assets.len() - 1 } else { i - 1 }
        });
        self.asset_list_state.select(Some(i));
    }

    pub fn select_asset(&mut self) {
        if let Some(i) = self.asset_list_state.selected() {
            if let Some(asset) = self.assets.get(i) {
                self.selected_asset_id = Some(asset.id.clone());
                self.selected_asset_symbol = Some(asset.symbol.clone());
                self.mode = Mode::TcsOrderSelection; 
                self.report_content = vec![
                    format!("Selected Asset: {}", asset.symbol),
                    "---".to_string(),
                    "Step 2/5: Choose Analysis Order.".to_string(),
                ];
            }
        }
    }

    pub fn next_order(&mut self) {
        let i = self.order_list_state.selected().map_or(0, |i| (i + 1) % self.order_options.len());
        self.order_list_state.select(Some(i));
    }

    pub fn previous_order(&mut self) {
        let i = self.order_list_state.selected().map_or(self.order_options.len() - 1, |i| {
            if i == 0 { self.order_options.len() - 1 } else { i - 1 }
        });
        self.order_list_state.select(Some(i));
    }

    pub fn select_order(&mut self) {
        if let Some(i) = self.order_list_state.selected() {
            let order_type = self.order_options[i].label;
            self.selected_order_type = Some(order_type.to_string());
            self.mode = Mode::TcsChainSelection; 
            self.report_content = vec![
                format!("Asset: {}", self.selected_asset_symbol.as_deref().unwrap_or("N/A")),
                format!("Order Type: {}", order_type),
                "---".to_string(),
                "Step 3/5: Choose Predictive Chain/Context.".to_string(),
            ];
        }
    }

    pub fn next_chain(&mut self) {
        let i = self.chain_list_state.selected().map_or(0, |i| (i + 1) % self.chain_options.len());
        self.chain_list_state.select(Some(i));
    }

    pub fn previous_chain(&mut self) {
        let i = self.chain_list_state.selected().map_or(self.chain_options.len() - 1, |i| {
            if i == 0 { self.chain_options.len() - 1 } else { i - 1 }
        });
        self.chain_list_state.select(Some(i));
    }

    pub fn select_chain(&mut self) {
        if let Some(i) = self.chain_list_state.selected() {
            let chain_type = self.chain_options[i].label;
            self.selected_chain = Some(chain_type.to_string());
            
            // --- Logic for TCS_DEFAULT ---
            if chain_type == "TCS_DEFAULT" {
                // For default, we automatically set PS1/PS2 to use the context data
                self.bias_fields_order = vec!["PS1".to_string(), "PS2".to_string()];
                self.bias_input.clear();
                self.bias_input.insert("PS1".to_string(), "AUTO".to_string());
                self.bias_input.insert("PS2".to_string(), "AUTO".to_string());
                
                // Skip bias input, go straight to report view
                self.confirm_bias_input(); 
                
                // Update message for clarity
                self.report_content.push("NOTE: Default chain uses session_context biases.".to_string());

            } else {
                // --- Standard Chain Path (Requires Bias Input) ---
                self.initialize_bias_input();
                
                // Transition to Step 4
                self.mode = Mode::TcsBiasInput; 
                self.report_content = vec![
                    format!("Asset: {}", self.selected_asset_symbol.as_deref().unwrap_or("N/A")),
                    format!("Order Type: {}", self.selected_order_type.as_deref().unwrap_or("N/A")),
                    format!("Chain Selected: {}", chain_type),
                    "---".to_string(),
                    "Step 4/5: Select bias for each session (TAB to switch session field).".to_string(),
                ];
            }
        }
    }
    
    /// Switches the active session field (e.g., PS2 -> PS1) in the bias input stage.
    pub fn next_session_field(&mut self) {
        if self.bias_fields_order.is_empty() { return; }
        
        let current_index = self.bias_fields_order.iter().position(|f| f == &self.active_session_field)
            .unwrap_or(0);
        
        let new_index = (current_index + 1) % self.bias_fields_order.len();

        self.active_session_field = self.bias_fields_order[new_index].clone();
        
        // Ensure the bias selection list highlights the current bias for the new field
        if let Some(current_bias) = self.bias_input.get(&self.active_session_field) {
            if let Some(idx) = BIAS_STATE_OPTIONS.iter().position(|&s| s == current_bias) {
                self.bias_list_state.select(Some(idx));
            } else {
                self.bias_list_state.select(Some(0));
            }
        }
    }
    
    /// Moves the highlight in the BIAS_STATE_OPTIONS list up.
    pub fn previous_bias_state(&mut self) {
        let current_index = self.bias_list_state.selected().unwrap_or(0);
        let len = BIAS_STATE_OPTIONS.len();
        let new_index = if current_index == 0 { len - 1 } else { current_index - 1 };
        self.bias_list_state.select(Some(new_index));
    }
    
    /// Moves the highlight in the BIAS_STATE_OPTIONS list down.
    pub fn next_bias_state(&mut self) {
        let current_index = self.bias_list_state.selected().unwrap_or(0);
        let new_index = (current_index + 1) % BIAS_STATE_OPTIONS.len();
        self.bias_list_state.select(Some(new_index));
    }
    
    /// Confirms the selected bias state for the active session field and moves to the next field.
    pub fn select_bias_state(&mut self) {
        if let Some(i) = self.bias_list_state.selected() {
            let selected_state = BIAS_STATE_OPTIONS[i].to_string();
            // Store the selected bias in the HashMap
            self.bias_input.insert(self.active_session_field.clone(), selected_state);
            
            // Move to the next session field automatically
            self.next_session_field();
        }
    }

    /// Confirms all biases and transitions to the report view.
    pub fn confirm_bias_input(&mut self) {
        self.mode = Mode::TcsReportView; 
        self.report_content = vec![
            format!("Asset: {}", self.selected_asset_symbol.as_deref().unwrap_or("N/A")),
            format!("Order Type: {}", self.selected_order_type.as_deref().unwrap_or("N/A")),
            format!("Chain Selected: {}", self.selected_chain.as_deref().unwrap_or("N/A")),
            "---".to_string(),
            "Session Biases Confirmed:".to_string(),
        ];
        
        for field_name in &self.bias_fields_order {
            let value = self.bias_input.get(field_name).cloned().unwrap_or_default();
            self.report_content.push(format!("  {}: {}", field_name, value));
        }
        
        self.report_content.push("---".to_string());
        self.report_content.push("Step 5/5: Press [R] to Run Full Report.".to_string());
    }
}