use crate::symmetric_grid::SymmetricPriceGrid; 
use crate::price_grid::{PriceLevel, GridLayer};


#[derive(Debug, Clone)]
pub struct AnchoredGrid {
    // 1. Context
    pub layer: GridLayer,
    
    // 2. The core math grid
    pub spg: SymmetricPriceGrid,
    
    // 3. Time bounds (using u64 for timestamps, e.g., milliseconds since epoch)
    pub start_timestamp: u64,
    pub end_timestamp: u64,
}

impl AnchoredGrid {
    /// Creates a new AnchoredGrid instance.
    pub fn new(
        layer: GridLayer, 
        high: f64, 
        low: f64, 
        start_ts: u64, 
        end_ts: u64
    ) -> Result<Self, String> {
        // Validate anchors before creating the SPG
        if low >= high {
            return Err(format!("Invalid anchors for {:?}: low ({}) must be less than high ({})", layer, low, high));
        }

        let spg = SymmetricPriceGrid::new(high, low);

        Ok(AnchoredGrid {
            layer,
            spg,
            start_timestamp: start_ts,
            end_timestamp: end_ts,
        })
    }

    /// Helper function to retrieve a level price from the underlying SPG
    pub fn get_price(&self, level: PriceLevel) -> Option<f64> {
        self.spg.get_level_price(level)
    }

    pub fn update(&mut self, price: f64) {
        let mut changed = false;
        let mut new_high = self.spg.high;
        let mut new_low = self.spg.low;

        if price > new_high {
            new_high = price;
            changed = true;
        }
        if price < new_low {
            new_low = price;
            changed = true;
        }

        if changed {
            // Re-initialize the SPG with new anchors to shift all levels
            self.spg = SymmetricPriceGrid::new(new_high, new_low);
        }
    }

    
}




