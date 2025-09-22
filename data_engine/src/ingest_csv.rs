
use crate::ingest_from_csv;
use anyhow::Result;


#[tokio::main]
pub async fn ingest_main() -> Result<()> {
    // 1. Specify all the required asset details
    let file_path = "./US100.csv";
    let asset_symbol = "USTECH";
    let asset_name = Some("USTECH 100 Index".to_string());
    let asset_class = Some("Index".to_string());
    let currency = Some("USD".to_string());

    let exchange = Some("DUKASCOPY".to_string());
    let timezone = Some("UTC".to_string());
    
    println!("Connecting to the database...");
    println!("Starting data ingestion from '{}' for asset '{}'...", file_path, asset_symbol);

    
    
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