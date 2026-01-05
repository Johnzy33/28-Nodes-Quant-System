

use crate::market_classification::{MarketType};
use crate::market_classification::MarketRatios;





#[cfg(test)]
mod tests {
    use super::*;

    // Helper to keep tests clean
    fn classify(o: f64, h: f64, l: f64, c: f64) -> MarketType {
      let data =  MarketRatios::new(o, h, l, c);
      let data_type = MarketType::get_classification(&data);
      return  data_type;
    }

    #[test]
    fn test_path_a_large_body_logic() {
        // 1. Healthy Bullish: Range 12, Body 8 (66%), Upper Wick 2 (16%)
        // Body >= 25% and Upper Wick < 30%
        assert_eq!(classify(100.0, 110.0, 98.0, 108.0), MarketType::Bullish);

        // 2. Failed Bullish: Range 22, Body 10 (45%), Upper Wick 10 (45%)
        // Body >= 25% but Upper Wick >= 30%
        assert_eq!(classify(100.0, 120.0, 98.0, 110.0), MarketType::FailedBullish);

        // 3. Healthy Bearish: Range 22, Body 15 (68%), Lower Wick 5 (22%)
        // Body >= 25% and Lower Wick < 30%
        assert_eq!(classify(100.0, 102.0, 80.0, 85.0), MarketType::Bearish);
    }

    #[test]
    fn test_path_b_small_body_logic() {
        // 1. Hard Bullish Reversal: Range 32, Body 1 (3%), Lower Wick 30 (93%)
        // Body < 25%, Lower Wick >= 60%
        assert_eq!(classify(100.0, 101.0, 69.0, 101.0), MarketType::BullishReversal);

        // 2. Pure Indecision: Range 10, Body 1 (10%), Upper Wick 4.5, Lower Wick 4.5
        // Body < 25%, Wick Diff < 15%
        assert_eq!(classify(100.0, 105.0, 95.0, 100.5), MarketType::PureIndecision);

        // 3. Soft Bearish Reversal: Range 20, Body 2 (10%), Upper 15 (75%), Lower 3 (15%)
        // Body < 25%, Upper Wick dominates
        assert_eq!(classify(100.0, 115.0, 95.0, 98.0), MarketType::BearishReversal);
    }

    #[test]
    fn test_edge_cases() {
        // System Start / No Movement
        assert_eq!(classify(100.0, 100.0, 100.0, 100.0), MarketType::Other);

        // Extremely Tight Range (Epsilon check)
        assert_eq!(classify(100.0, 100.000000001, 100.0, 100.0), MarketType::Other);
    }
}


#[cfg(test)]
mod tests_new {
    use super::*;


     // 1. Setup Environment
    


    #[test]
    fn debug_market_data() {
        // Define a set of scenarios: (Open, High, Low, Close, Label)
        let scenarios = vec![
            (100.0, 110.0, 98.0, 108.0, "Healthy Bullish"),
            (100.0, 120.0, 98.0, 110.0, "Failed Bullish (Large Wick)"),
            (100.0, 105.0, 95.0, 100.5, "Doji / Indecision"),
            (100.0, 101.0, 70.0, 101.0, "Hammer (Bullish Reversal)"),
            (100.0, 130.0, 99.0, 105.0, "Shooting Star (Bearish Reversal)"),
            (100.0, 110.0, 90.0, 102.1, "Borderline: 21% Body / 40% Upper Wick"),
            (100.0, 100.0, 100.0, 100.0, "System Start (No Range)"),
        ];

        println!("\n{:<30} | {:<15} | {:<15}", "Scenario Label", "MarketType", "Body Ratio");
        println!("{}", "-".repeat(65));
        
        for (o, h, l, c, label) in scenarios {
            let r_data = MarketRatios::new(o, h, l, c);
            let data = MarketType::get_classification(&r_data);
            let body_ratio = r_data.body_ratio;
            
            println!("{:<30} | {:<15?} | {:.2}%", label, data, body_ratio * 100.0);
        }
    }
}


