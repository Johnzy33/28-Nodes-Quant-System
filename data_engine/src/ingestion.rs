// data_engine/src/ingestion.rs

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use csv_async::{AsyncDeserializer};
use futures_util::stream::StreamExt;
use serde::Deserialize;
use tokio::fs::File;
use tokio_util::compat::TokioAsyncReadCompatExt;

use database_engine::{
    db_connect::connect,
    queries::{create_or_get_asset, ingest_market_data},
};
use shared_models::{Asset, MarketData};
use surrealdb::sql::Datetime;

// CsvRecord struct
#[derive(Debug, Deserialize)]
pub struct CsvRecord {
    #[serde(rename = "Date")]
    pub date: String,
    #[serde(rename = "Time")]
    pub time: String,
    #[serde(rename = "Open")]
    pub open: f64,
    #[serde(rename = "High")]
    pub high: f64,
    #[serde(rename = "Low")]
    pub low: f64,
    #[serde(rename = "Close")]
    pub close: f64,
    #[serde(rename = "Volume")]
    pub volume: f64,
}

/// Ingests market data from a CSV file into the database.
pub async fn ingest_from_csv(
    file_path: &str,
    asset_symbol: &str,
    asset_name: Option<String>,
    asset_class: Option<String>,
    currency: Option<String>,
    exchange: Option<String>,
    timezone: Option<String>,
) -> Result<()> {
    let db = connect().await.context("Failed to connect to the database")?;

    // Create a complete Asset struct with all the required fields
    let asset_to_create = Asset {
        id: None,
        symbol: asset_symbol.to_string(),
        name: asset_name,
        asset_class: asset_class,
        currency: currency,
        tick_size: Some(0.01),
        lot_size: Some(0.01),
        price_decimals: Some(2),
        exchange: exchange,
        timezone: timezone,
        active: Some(true),
        first_listed_ts: Some(Datetime::from(Utc::now()).into()),
    };

    let asset: Asset = create_or_get_asset(&db, asset_to_create)
        .await
        .context("Failed to create or get asset")?;
    let asset_id = asset.id.context("Asset ID was not created")?;

    let file = File::open(file_path).await.context("Failed to open CSV file")?;
    let mut deserializer = AsyncDeserializer::from_reader(file.compat());
    let mut records = deserializer.deserialize::<CsvRecord>();

    let mut records_to_ingest = Vec::new();

    while let Some(record_result) = records.next().await {
        let raw_record = record_result.context("Failed to deserialize CSV record")?;
        let full_date_str = format!("{} {}", raw_record.date, raw_record.time);
        let naive_datetime = NaiveDateTime::parse_from_str(&full_date_str, "%Y.%m.%d %H:%M")
            .context("Failed to parse date string")?;
        let utc_datetime = DateTime::<Utc>::from_naive_utc_and_offset(naive_datetime, Utc);

        let market_data = MarketData {
            id: None,
            asset_id: asset_id.clone(),
            ts: Datetime::from(utc_datetime),
            open: Some(raw_record.open),
            high: Some(raw_record.high),
            low: Some(raw_record.low),
            close: raw_record.close,
            volume: Some(raw_record.volume),
            seq: Some(0),
            source: Some("Dukascopy".to_string()),
        };

        records_to_ingest.push(market_data);
    }

    if records_to_ingest.is_empty() {
       // println!("No records found to ingest.");
        return Ok(());
    }

    //println!("Attempting to ingest {} records into the database...", records_to_ingest.len());

  //  println!("First record to be sent: {:?}", records_to_ingest.get(0));


  
    ingest_market_data(&db, records_to_ingest)
        .await
        .context("Failed to ingest market data")?;

    println!("Data ingestion successful. ✅");
    Ok(())
}