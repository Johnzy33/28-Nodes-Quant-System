
use anyhow::Result;
use surrealdb::{RecordId};
use database_engine::{
    db_connect::connect,
    sql_comput::define_aggregation_views as aggregators,
    candle::candle_pattern_function
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Connecting to the database...");
    let db = connect().await?;

   // println!("Starting aggregation pipeline...");
    let asset_id = RecordId::from(("assets", "b5ccsuo1bci9upec54ic"));

    aggregators(&db, &asset_id).await?;
    //    println!("Defining Candle Pattern function...");
    //     candle_pattern_function(&db).await?;
    Ok(())
}

