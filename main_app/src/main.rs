
use data_engine::ingestion::ingest_from_csv;
use anyhow::Result;
use database_engine::{
    db_connect::connect,
    schema,
};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Specify all the required asset details
    let file_path = "./US100.csv";
    let asset_symbol = "US1000";
    let asset_name = Some("USTec 100 Index".to_string());
    let asset_class = Some("Index".to_string());
    let currency = Some("USD".to_string());
    let exchange = Some("NASDAQ".to_string());
    let timezone = Some("UTC".to_string());
    
    println!("Connecting to the database...");
    println!("Starting data ingestion from '{}' for asset '{}'...", file_path, asset_symbol);

       let db = connect().await?;
      //  schema::apply_schema(&db).await?;
    
    // 2. Call the ingestion function with all the details
    if let Err(e) = ingest_from_csv(
        file_path, 
        asset_symbol,
        asset_name,
        asset_class,
        currency,
        exchange,
        timezone,
    ).await {
        eprintln!("Error during ingestion: {:?}", e);
    }
    
    Ok(())
}