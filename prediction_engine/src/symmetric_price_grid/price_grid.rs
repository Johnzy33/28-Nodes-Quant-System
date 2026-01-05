use crate::symmetric_price_grid::anchored_grid::AnchoredGrid;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum PriceLevel {

    ExitLower,      
    MidPointLower,  
    Zero,           
    DiscountLower,  
    MidPoint,       
    DiscountUpper,  
    One,          
    MidPointUpper,  
    ExitUpper,     
}

impl PriceLevel {
    
    pub fn multiplier(&self) -> f64 {
        match self {
            PriceLevel::ExitLower      => -0.50,
            PriceLevel::MidPointLower  => -0.25,
            PriceLevel::Zero           => 0.0,
            PriceLevel::DiscountLower  => 0.4,
            PriceLevel::MidPoint       => 0.5,
            PriceLevel::DiscountUpper  => 0.6,
            PriceLevel::One            => 1.0,
            PriceLevel::MidPointUpper  => 1.25,
            PriceLevel::ExitUpper      => 1.5,
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum GridLayer {
    Yearly,       
    Critical,       
    PW, // The just concluded week (Prev Week)
    PW1,// The high and low of the week two back and prev week i.e a rang of pw + 2 weeks back
    PW2,         // range of 3 weeks back. 
    MultiDay,      
    PreviousDay,    // just the prev day high and low
    CS, // the current session, this is what the live tick updates
    PS1,// the high and low of the prev session
    PS2// range of the high and low of the past two session. contains high low from ps2 to ps1
}

#[derive(Debug, Clone,)]
pub struct LayerManager<T> {
    pub history: Vec<T>,        // The raw data (ClassifiedSession, WeeklyView, etc.)
    pub grids: Vec<AnchoredGrid>, // The calculated SPG grids
    pub layer_type: GridLayer,
}



impl<T> LayerManager<T> {
    /// Creates a new LayerManager with pre-allocated capacity
    pub fn new(layer_type: GridLayer, capacity: usize) -> Self {
        Self {
            history: Vec::with_capacity(capacity),
            grids: Vec::with_capacity(capacity),
            layer_type,
        }
    }

    /// Primary entry point for live price updates
    /// If is_static is true, this is a no-op (preserving the historical anchor)
    pub fn update_live(&mut self, price: f64, is_static: bool) {
        if !is_static {
            // Index 0 is always the "Current" (CS) grid
            if let Some(active) = self.grids.get_mut(0) {
                active.update(price);
            }
        }
    }

    /// Helper to reset the layer during a full database sync
    pub fn reset(&mut self) {
        self.history.clear();
        self.grids.clear();
    }
}