
use anyhow::{Result, Context};
use crate::DB;

pub const SCHEMA: &str = r#"
DEFINE TABLE assets SCHEMAFULL;
DEFINE FIELD id ON assets TYPE record<assets>;
DEFINE FIELD symbol ON assets TYPE string;
DEFINE FIELD name ON assets TYPE string;
DEFINE FIELD asset_class ON assets TYPE string;
DEFINE FIELD currency ON assets TYPE string;
DEFINE FIELD tick_size ON assets TYPE float;
DEFINE FIELD lot_size ON assets TYPE float;
DEFINE FIELD price_decimals ON assets TYPE int;
DEFINE FIELD exchange ON assets TYPE string;
DEFINE FIELD timezone ON assets TYPE string;
DEFINE FIELD active ON assets TYPE bool;
DEFINE FIELD first_listed_ts ON assets TYPE datetime;

DEFINE TABLE market_data SCHEMAFULL;
DEFINE FIELD id ON market_data TYPE record<market_data>;
DEFINE FIELD asset_id ON market_data TYPE record<assets>;
DEFINE FIELD ts ON market_data TYPE datetime;
DEFINE FIELD open ON market_data TYPE float;
DEFINE FIELD high ON market_data TYPE float;
DEFINE FIELD low ON market_data TYPE float;
DEFINE FIELD close ON market_data TYPE float;
DEFINE FIELD volume ON market_data TYPE float;
DEFINE FIELD seq ON market_data TYPE int;
DEFINE FIELD source ON market_data TYPE string;

DEFINE TABLE orders SCHEMAFULL;
DEFINE FIELD id ON orders TYPE record<orders>;
DEFINE FIELD asset_id ON orders TYPE record<assets>;
DEFINE FIELD client_order_id ON orders TYPE string;
DEFINE FIELD side ON orders TYPE string;
DEFINE FIELD qty ON orders TYPE float;
DEFINE FIELD price ON orders TYPE float;
DEFINE FIELD order_type ON orders TYPE string;
DEFINE FIELD status ON orders TYPE string;
DEFINE FIELD created_ts ON orders TYPE datetime;
DEFINE FIELD updated_ts ON orders TYPE datetime;
DEFINE FIELD tags ON orders TYPE array;
DEFINE FIELD meta ON orders TYPE object;

DEFINE TABLE predictions SCHEMAFULL;
DEFINE FIELD id ON predictions TYPE record<predictions>;
DEFINE FIELD asset_id ON predictions TYPE record<assets>;
DEFINE FIELD ts ON predictions TYPE datetime;
DEFINE FIELD horizon_ms ON predictions TYPE int;
DEFINE FIELD predicted_price ON predictions TYPE float;
DEFINE FIELD confidence ON predictions TYPE float;
DEFINE FIELD model_id ON predictions TYPE string;
DEFINE FIELD input_market_data_id ON predictions TYPE record<market_data>;
DEFINE FIELD notes ON predictions TYPE string;

DEFINE TABLE strategies SCHEMAFULL;
DEFINE FIELD id ON strategies TYPE record<strategies>;
DEFINE FIELD name ON strategies TYPE string;
DEFINE FIELD description ON strategies TYPE string;
DEFINE FIELD applicable_assets ON strategies TYPE array<record<assets>>;
DEFINE FIELD params ON strategies TYPE object;
DEFINE FIELD active ON strategies TYPE bool;
DEFINE FIELD created_ts ON strategies TYPE datetime;
"#;

pub async fn apply_schema(db: &DB) -> Result<()> {
    db.query(SCHEMA)
        .await
        .context("Failed to apply database schema")?;
    Ok(())
}