use crate::DB;
use anyhow::{Result, Context};
use shared_models::{Asset, MarketData, Id};
use surrealdb::sql::{Thing, Datetime};

/// Creates a new asset record or returns the existing one if it already exists.
pub async fn create_or_get_asset(db: &DB, asset: Asset) -> Result<Asset> {
    // 1. Check if the asset already exists using a SELECT statement
    let mut result = db
        .query("SELECT * FROM assets WHERE symbol = $symbol")
        .bind(("symbol", asset.symbol.clone()))
        .await
        .context("Failed to query for asset")?;

    let existing_assets: Vec<Asset> = result
        .take(0)
        .context("Failed to deserialize existing asset")?;

    // 2. If the asset exists, return the first one found
    if let Some(existing_asset) = existing_assets.into_iter().next() {
        return Ok(existing_asset);
    }

    // 3. If the asset does not exist, create a new record using the provided data
    let created_asset: Option<Asset> = db
        .create("assets")
        .content(asset)
        .await
        .context("Failed to create new asset")?;

    // 4. Return the newly created asset
    created_asset.context("Created asset was not returned")
}

// -----------------------------------------------------------------------------------------------------------------------

/// Ingests a vector of MarketData structs into the `market_data` table.
pub async fn ingest_market_data(db: &DB, records: Vec<MarketData>) -> Result<()> {
    db.query("BEGIN TRANSACTION;").await.context("Failed to start transaction")?;

    for record in records {
        let created: Option<MarketData> = db.create("market_data")
            .content(record)
            .await
            .context("Failed to insert single record")?;
    }

    db.query("COMMIT TRANSACTION;").await.context("Failed to commit transaction")?;

    println!("Bulk ingestion query executed successfully. ✅");
    Ok(())
}

// -----------------------------------------------------------------------------------------------------------------------

/// Fetches a specific number of recent market data records for an asset.
pub async fn fetch_market_data(db: &DB, asset_id: Id, limit: u32) -> Result<Vec<MarketData>> {
    let mut query_result = db
        .query("SELECT * FROM market_data WHERE asset_id = $asset_id ORDER BY ts DESC LIMIT $limit")
        .bind(("asset_id", asset_id))
        .bind(("limit", limit))
        .await
        .context("Failed to fetch market data")?;
    
    // The query returns a `QueryResult`, so we use `take(0)` to get the first result set.
    let records: Vec<MarketData> = query_result.take(0).context("Failed to get market data from query result")?;
    
    Ok(records)
}