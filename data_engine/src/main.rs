
use shared_models::data_model::DataService;
use shared_models::db_models as db;
use data_engine::traits::DataServiceBase;
use database_engine::runtime;
use dotenvy;
use std::sync::Arc;
use data_engine::watchdog::{Mt5Watchdog,WatchdogState};
use log::{ error};
// use std::error::Error;




#[tokio::main]
async fn main()-> db::AppResult<()>{
    env_logger::init();
    dotenvy::dotenv().ok();

    // 1. Setup Surreal instead of Postgres
    let dbs = runtime::setup_database().await?;

    let data_service = Arc::new(DataService::new(Arc::clone(&dbs)));

    let state = WatchdogState::new();
    let watchdog_state = state.clone();
    
    tokio::spawn(async move {
        let watchdog = Mt5Watchdog::new(watchdog_state);
        if let Err(e) = watchdog.run().await {
            error!("Watchdog error: {}", e);
        }
    });

    // 3. Run the service
    data_service.run("127.0.0.1:9090", Arc::clone(&dbs), Arc::clone(&data_service), state).await

}