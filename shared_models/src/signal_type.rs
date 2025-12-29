use sqlx::{Type}; 
use std::error::Error as StdError; 
use std::fmt;
use strum_macros::{Display, EnumString};

// ====================================================================
//  Custom Error Definition
// ====================================================================

#[derive(Debug)]
pub struct SignalTypeError(String);

impl fmt::Display for SignalTypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid signal type: {}", self.0)
    }
}

/// Allows SignalTypeError to be converted into the necessary boxed trait object for SQLx.
impl StdError for SignalTypeError {}


// ====================================================================
//  Trading and Quality Enums
// ====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradingBias {
    Buy,
    Sell,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalQuality {
    High,    
    Medium,
    Low,    
}


// ====================================================================
//  TradeSignal Struct
// ====================================================================

#[derive(Debug, Clone)]
pub struct TradeSignal {
    pub instrument: String,
    pub current_price: f64,
    pub bias: TradingBias,
    pub quality: SignalQuality,
    pub entry_level: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub reason: String,
}


// ====================================================================
//  Market Classification Enums (SQLx Mapped)
// ====================================================================


#[derive(Debug, PartialEq, Clone, Copy, EnumString, Display)]
#[derive(Type)]
#[sqlx(type_name = "TEXT")]
pub enum MarketType {
    Bullish,
    Bearish,
    FailedBearish,
    FailedBullish,
    BullishReversal,
    BearishReversal,
    PureIndecision,
    Other,
}

#[derive(Debug, PartialEq, Clone, Copy, EnumString, Display)]
#[derive(Type)]
#[sqlx(type_name = "TEXT")]
pub enum MarketSubtype {
    FailedBearish,
    FailedBullish,
    BullishReversal,
    BearishReversal,
    PureIndecision,
    Other,
}

// ====================================================================
//  MarketClassification Struct
// ====================================================================

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct MarketClassification {
    pub classification: MarketType,
}