
use std::collections::HashMap;
use crate::price_grid::{PriceLevel};


#[derive(Debug, Clone)]
pub struct SymmetricPriceGrid {
    pub high: f64,
    pub low: f64,
    pub range_distance: f64,
    pub levels: HashMap<PriceLevel, f64>,
}

impl SymmetricPriceGrid {
    
    pub fn new(high: f64, low: f64) -> Self {
       
        if low >= high {
           
            panic!("SPG 'low' must be less than 'high'.");
        }

        let range_distance = high - low;
        let mut levels = HashMap::new();

        
        for &level in [
            PriceLevel::ExitLower,
            PriceLevel::MidPointLower,
            PriceLevel::Zero,
            PriceLevel::DiscountLower,
            PriceLevel::MidPoint,
            PriceLevel::DiscountUpper,
            PriceLevel::One,
            PriceLevel::MidPointUpper,
            PriceLevel::ExitUpper,
        ].iter() {
            let multiplier = level.multiplier();
            let price = low + (multiplier * range_distance);
            levels.insert(level, price);
        }

        SymmetricPriceGrid {
            high,
            low,
            range_distance,
            levels,
        }
    }

   
    pub fn get_level_price(&self, level: PriceLevel) -> Option<f64> {
        self.levels.get(&level).copied()
    }
}