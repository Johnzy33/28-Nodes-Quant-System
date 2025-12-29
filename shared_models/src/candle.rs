

// shared_models/src/candle.rs (or similar)
#[derive(Debug, Clone, Copy)]
pub struct Candle {
    pub timestamp: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    // Note: Volume is optional for this model, but often included.
}