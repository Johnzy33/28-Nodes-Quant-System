use serde::{Deserialize, Serialize};
use crate::prelude::{Id, Timestamp};
use surrealdb::sql::Datetime;
use std::fmt;


pub const DEFAULT_DOJI_BODY_RATIO: f64 = 0.1;
pub const DEFAULT_BODY_WICK_RATIO_LONG: f64 = 0.5;
pub const DEFAULT_BODY_WICK_RATIO_SHORT: f64 = 0.3;
pub const DEFAULT_UPPER_VS_LOWER_RATIO: f64 = 0.6;
pub const DEFAULT_EPS: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandlePattern {
    BullishHammer,
    BearishHammer,
    BullishShootingStar,
    BearishShootingStar,
    BullishLongBody,
    BearishLongBody,
    MildBullish,
    MildBearish,
    DojiSpinningTop,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Candle {
    pub id: Option<Id>,
    pub ts: Datetime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}
impl Candle {
    pub fn new(ts: Datetime, open: f64, high: f64, low: f64, close: f64) -> Self {
        Self {
            id: None,
            ts,
            open,
            high,
            low,
            close,
        }
    }
}


impl CandlePattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            CandlePattern::BullishHammer => "Bullish Hammer",
            CandlePattern::BearishHammer => "Bearish Hammer",
            CandlePattern::BullishShootingStar => "Bullish Shooting Star",
            CandlePattern::BearishShootingStar => "Bearish Shooting Star",
            CandlePattern::BullishLongBody => "Bullish Long Body",
            CandlePattern::BearishLongBody => "Bearish Long Body",
            CandlePattern::MildBullish => "Mild Bullish",
            CandlePattern::MildBearish => "Mild Bearish",
            CandlePattern::DojiSpinningTop => "Doji/SpinningTop",
            CandlePattern::Unknown => "Unknown",
        }
    }
}

impl fmt::Display for CandlePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn pattern_from_ohlc(
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    doji_body_ratio: f64,
    body_wick_ratio_long: f64,
    body_wick_ratio_short: f64,
    upper_vs_lower_ratio: f64,
    eps: f64,
) -> String {
    let full_range = high - low;
    let body_range = (close - open).abs();

    if full_range < eps {
        return CandlePattern::Unknown.to_string();
    }

    let upper_wick = high - close.max(open);
    let lower_wick = open.min(close) - low;

    let body_ratio = body_range / full_range;
    let upper_wick_ratio = upper_wick / full_range;
    let lower_wick_ratio = lower_wick / full_range;

    let is_bullish = close > open;

    // Doji or Spinning Top
    if body_ratio <= doji_body_ratio {
        return CandlePattern::DojiSpinningTop.to_string();
    }

    // Hammer/Shooting Star
    if body_ratio < body_wick_ratio_short {
        if upper_wick_ratio / (lower_wick_ratio + eps) < upper_vs_lower_ratio {
            return if is_bullish {
                CandlePattern::BullishHammer
            } else {
                CandlePattern::BearishHammer
            }.to_string();
        } else if lower_wick_ratio / (upper_wick_ratio + eps) < upper_vs_lower_ratio {
            return if is_bullish {
                CandlePattern::BullishShootingStar
            } else {
                CandlePattern::BearishShootingStar
            }.to_string();
        }
    }

    // Long Body
    if body_ratio >= body_wick_ratio_long {
        if is_bullish {
            return CandlePattern::BullishLongBody.to_string();
        } else {
            return CandlePattern::BearishLongBody.to_string();
        }
    }

    // Mild Body
    if is_bullish {
        CandlePattern::MildBullish.to_string()
    } else {
        CandlePattern::MildBearish.to_string()
    }
}