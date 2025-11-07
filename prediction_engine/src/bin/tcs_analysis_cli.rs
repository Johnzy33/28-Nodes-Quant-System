use database_engine::runtime::setup_database_pool; // Your custom DB setup
use prediction_engine::data_fetcher::{
    TCS_1ST_ORDER_QUERY_TEMPLATE, TCS_2ND_ORDER_QUERY_TEMPLATE, 
    fetch_tcs_data,
};
use prediction_engine::analysis_core::generate_tcs_report; // The analysis core
use shared_models::tcs_models::{Tcs1stOrderData, Tcs2ndOrderData}; 
use anyhow::{Result, Context};
use chrono::Local; // For getting the analysis date

// Define a default asset for testing
const DEFAULT_ASSET_ID: &str = "assets:US100:FundedNext"; 

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Setup Input Parameters
    let asset_id = std::env::args().nth(1).unwrap_or(DEFAULT_ASSET_ID.to_string());
    let analysis_date = Local::now().date_naive().to_string();

    println!("--- Starting TCS Analysis for Asset: {} on {} ---", asset_id, analysis_date);
    
    // 2. Initialize the database connection pool (from your database_engine crate)
    let pool = setup_database_pool().await.context("FATAL: Could not initialize database pool.")?;

    // 3. Data Acquisition
    
    // 3a. Fetch 1st Order Data
    let tcs_1st_order_data: Vec<Tcs1stOrderData> = fetch_tcs_data(
        &pool,
        TCS_1ST_ORDER_QUERY_TEMPLATE,
        &asset_id,
        Tcs1stOrderData::from_row, 
    ).await.context("Failed to fetch 1st Order TCS data")?;

    // 3b. Fetch 2nd Order Data
    let tcs_2nd_order_data: Vec<Tcs2ndOrderData> = fetch_tcs_data(
        &pool,
        TCS_2ND_ORDER_QUERY_TEMPLATE,
        &asset_id,
        Tcs2ndOrderData::from_row, 
    ).await.context("Failed to fetch 2nd Order TCS data")?;
    
    if tcs_1st_order_data.is_empty() || tcs_2nd_order_data.is_empty() {
        println!("⚠️ Warning: Insufficient data for full cross-analysis. Check database for current context.");
        return Ok(());
    }
    

    println!("✅ Data Acquisition Complete. Proceeding to Analysis.");

    // 4. Core Analysis and Report Generation
    let final_report = generate_tcs_report(
        &asset_id,
        tcs_1st_order_data,
        tcs_2nd_order_data,
        &analysis_date,
    );

    // 5. Output Results (Print the formatted Markdown report)
    println!("\n=======================================================");
    println!("               TCS ANALYSIS REPORT OUTPUT              ");
    println!("=======================================================\n");
    println!("{}", final_report.final_markdown_report);

    Ok(())
}