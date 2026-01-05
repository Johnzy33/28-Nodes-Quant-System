use database_engine::runtime;
use shared_models::MarketType;
use data_engine::traits::DataViewExt;
use shared_models::data_model::DataService;

pub fn main(){}

#[cfg(test)]
mod integration_tests {
    use shared_models::data_model;
    use dotenvy;



    use super::*; // Import your repo/structs
    
    #[tokio::test]
    async fn debug_real_market_classification() -> Result<(), Box<dyn std::error::Error>> {

         // 1. Setup Environment
      //  env_logger::init();
        dotenvy::dotenv().ok();

        // 1. Initialize your Repository (adjust to your actual struct name)
        // Assuming your repo uses the root/root credentials internally
        let pool = runtime::setup_database_pool().await?;
        let data_service = data_model::DataService::new(pool);

        // 2. Target a specific asset and lookback (e.g., last 180 hours)
        let asset_id = "assets:US30:FundedNext"; 
        let lookback = Some(180);

        println!("\nFetching real data for {}...", asset_id);

        // 3. Call your function
        let (classified_sessions, _) = data_service
            .calculate_session_and_context(asset_id, lookback)
            .await?;

        if classified_sessions.is_empty() {
            println!("No data found for asset: {}", asset_id);
            return Ok(());
        }

        // 4. Print the "Fine Line" audit table
        println!("\n{:<15} | {:<12} | {:<10} | {:<10} | {:<10} | {:<15}", 
                 "Session Name", "End Time", "Body %", "UpWick %", "LoWick %", "Classification");
        println!("{}", "-".repeat(90));

        let session_count = classified_sessions.len();
        for s in classified_sessions {
            let range = (s.high - s.low).max(1e-10);
            let body_size = (s.close - s.open).abs();
            let body_ratio = (body_size / range) * 100.0;
            
            let real_body_high = s.close.max(s.open);
            let real_body_low = s.close.min(s.open);
            let up_wick = ((s.high - real_body_high) / range) * 100.0;
            let lo_wick = ((real_body_low - s.low) / range) * 100.0;

            // Using ANSI colors for better visibility in terminal
            let color = match s.session_type {
                MarketType::Bullish | MarketType::BullishReversal => "\x1b[32m", // Green
                MarketType::Bearish | MarketType::BearishReversal => "\x1b[31m", // Red
                MarketType::FailedBullish | MarketType::FailedBearish => "\x1b[33m", // Yellow
                _ => "\x1b[0m", // Reset
            };

            println!(
                "{:<15} | {:<12} | {:>9.1}% | {:>9.1}% | {:>9.1}% | {}{:?}\x1b[0m",
                s.session_name,
                s.trading_date,
                body_ratio,
                up_wick,
                lo_wick,
                color,
                s.session_type
            );
        }

        println!("\nTotal Sessions Processed: {}", session_count);
        Ok(())
    }

    #[tokio::test]
    async fn test_market_bias_story() -> Result<(), Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();
        let pool = runtime::setup_database_pool().await?;
        let data_service = data_model::DataService::new(pool);

        let asset_id = "assets:US100:FundedNext"; 
        let lookback = Some(180);

        let (classified_sessions, _) = data_service
            .calculate_session_and_context(asset_id, lookback)
            .await?;

        println!("\n{:<15} | {:<12} | {:<15} | {:<15}", 
                "Session", "Date", "Current State", "Inverse (Bias)");
        println!("{}", "-".repeat(65));

        for s in classified_sessions {
            let current = s.session_type;
            let bias = current.inverse();

            let color = match bias {
                MarketType::Bullish | MarketType::BullishReversal => "\x1b[32m", // Green
                MarketType::Bearish | MarketType::BearishReversal => "\x1b[31m", // Red
                _ => "\x1b[0m",
            };

            println!(
                "{:<15} | {:<12} | {:<15?} | {}{:?}\x1b[0m",
                s.session_name,
                s.trading_date,
                current,
                color,
                bias
            );
        }

        Ok(())
    }
}

