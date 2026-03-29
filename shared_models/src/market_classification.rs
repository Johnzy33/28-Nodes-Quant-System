
use std::f64;
// use sqlx::{Type}; 
use strum_macros::{Display, EnumString};
use serde::{Deserialize, Serialize};



#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash,EnumString,Display)]
// #[derive(Type)]
#[derive(Default)]

pub enum MarketType {
    #[default]
    Other,
    Bullish,
    Bearish,
    FailedBearish,
    FailedBullish,
    BullishReversal,
    BearishReversal,
    PureIndecision,
}


#[derive(Debug, Clone, Serialize, Deserialize,PartialEq,)]
pub struct MarketRatios {
    pub body_ratio: f64,
    pub up_wick_ratio: f64,
    pub lo_wick_ratio: f64,
    pub total_wick_ratio: f64,
    pub is_bullish_candle: bool,
    pub range: f64,
    pub body_size: f64,
}

impl MarketRatios {

    pub fn new(open: f64, high: f64, low: f64, close: f64) -> Self {
        let range = (high - low).max(1e-10);
        let body_size = (close - open).abs();
        let real_body_high = close.max(open);
        let real_body_low = close.min(open);

        let up_wick_ratio = (high - real_body_high) / range;
        let lo_wick_ratio = (real_body_low - low) / range;

        Self {
            body_ratio: body_size / range,
            up_wick_ratio,
            lo_wick_ratio,
            total_wick_ratio: up_wick_ratio + lo_wick_ratio,
            is_bullish_candle: close > open,
            range,
            body_size,
        }
    }
}


impl MarketType {
    pub fn get_classification(r: &MarketRatios) -> Self {
        if r.range <= 1e-10 { return MarketType::Other; }

        // --- 1. THE 25% GATE ---
        if r.body_ratio >= 0.25 {
            // Path A: Large Body
            let dynamic_multiplier = if r.body_ratio < 0.35 { 1.5 } 
                                     else if r.body_ratio < 0.50 { 2.0 } 
                                     else { 2.5 };

            if r.total_wick_ratio > (r.body_ratio * dynamic_multiplier) {
                return MarketType::PureIndecision;
            }

            if r.is_bullish_candle {
                if r.up_wick_ratio >= 0.30 { return MarketType::FailedBullish; }
                return MarketType::Bullish;
            } else {
                if r.lo_wick_ratio >= 0.30 { return MarketType::FailedBearish; }
                return MarketType::Bearish;
            }
        } else {
            // Path B: Small Body
            if r.lo_wick_ratio >= 0.60 { return MarketType::BullishReversal; }
            if r.up_wick_ratio >= 0.60 { return MarketType::BearishReversal; }

            if (r.up_wick_ratio - r.lo_wick_ratio).abs() < 0.15 {
                return MarketType::PureIndecision;
            }

            if r.lo_wick_ratio > r.up_wick_ratio { MarketType::BullishReversal }
            else { MarketType::BearishReversal }
        }
    }

    pub fn inverse(&self) -> Self {
        match self {
            MarketType::Bullish => MarketType::Bullish,
            MarketType::Bearish => MarketType::Bearish,
            MarketType::FailedBullish => MarketType::Bearish,
            MarketType::FailedBearish => MarketType::Bullish,
            MarketType::BullishReversal => MarketType::Bullish,
            MarketType::BearishReversal => MarketType::Bearish,
            _ => MarketType::PureIndecision,
        }
    }
}


#[derive(Debug, PartialEq, Clone, Copy)]
pub struct MarketClassification {
    pub classification: MarketType,
}

