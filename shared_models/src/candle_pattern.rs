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
) -> CandlePattern {
    let full_range = high - low;
    let body_range = (close - open).abs();

    if full_range < eps {
        return CandlePattern::Unknown;
    }

    let upper_wick = high - close.max(open);
    let lower_wick = open.min(close) - low;

    let body_ratio = body_range / full_range;
    let upper_wick_ratio = upper_wick / full_range;
    let lower_wick_ratio = lower_wick / full_range;

    let is_bullish = close > open;

    // Doji or Spinning Top
    if body_ratio <= doji_body_ratio {
        return CandlePattern::DojiSpinningTop;
    }

    // Hammer/Shooting Star
    if body_ratio < body_wick_ratio_short {
        if upper_wick_ratio / (lower_wick_ratio + eps) < upper_vs_lower_ratio {
            return if is_bullish {
                CandlePattern::BullishHammer
            } else {
                CandlePattern::BearishHammer
            };
        } else if lower_wick_ratio / (upper_wick_ratio + eps) < upper_vs_lower_ratio {
            return if is_bullish {
                CandlePattern::BullishShootingStar
            } else {
                CandlePattern::BearishShootingStar
            };
        }
    }

    // Long Body
    if body_ratio >= body_wick_ratio_long {
        if is_bullish {
            return CandlePattern::BullishLongBody;
        } else {
            return CandlePattern::BearishLongBody;
        }
    }

    // Mild Body
    if is_bullish {
        CandlePattern::MildBullish
    } else {
        CandlePattern::MildBearish
    }
}

/// Helper function to use default constants
pub fn classify_with_defaults(open: f64, high: f64, low: f64, close: f64) -> CandlePattern {
    pattern_from_ohlc(
        open,
        high,
        low,
        close,
        DEFAULT_DOJI_BODY_RATIO,
        DEFAULT_BODY_WICK_RATIO_LONG,
        DEFAULT_BODY_WICK_RATIO_SHORT,
        DEFAULT_UPPER_VS_LOWER_RATIO,
        DEFAULT_EPS,
    )
}