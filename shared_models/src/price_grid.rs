

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
    Weekly,         
    MultiDay,      
    PreviousDay,    
}