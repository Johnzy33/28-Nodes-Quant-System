#[derive(Debug, Default, Clone)]
pub struct DataRefreshState {
    // 💥 ADD THIS FIELD
    pub log_history: Vec<String>,
}