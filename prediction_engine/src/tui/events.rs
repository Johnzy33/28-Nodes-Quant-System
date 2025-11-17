use crossterm::event::KeyEvent;

/// Defines all events that can be sent to the main TUI thread.
#[derive(Debug)]
pub enum TuiEvent {
    /// A user key press event (sent from the event polling thread)
    Key(KeyEvent),
    /// A tick event (sent from the timer thread)
    Tick,
    /// Completion event for the core data refresh task
    DataRefreshStatus(String),
    DataRefreshCompleted(Result<(), anyhow::Error>),
    /// Completion event for the FCI signal calculation
    FciSignalCompleted(Result<(), anyhow::Error>),

    /// Completion event for the CSV Ingestion task
    CsvIngestionCompleted(Result<(), anyhow::Error>),
    CsvIngestionAssetStatus(String),
    /// Quit command
    Quit,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HandleAction {
    Continue,
    Quit,
}