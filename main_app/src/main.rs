// use anyhow::Result;
// use database_engine::runtime::setup_database_pool; 

// use data_engine::consumer_ingestion::run_multi_topic_consumer; 
// use log::info;
// use dotenvy;


// #[tokio::main]
// async fn main() -> Result<()> {
//     // Initialize logging 
//     env_logger::init(); 
//     dotenvy::dotenv().ok();

//     info!("Starting multi-topic Kafka Consumer Service...");

//     // Setup Database Pool
//     let pool = setup_database_pool().await?;
    
   
//     if let Err(e) = run_multi_topic_consumer(pool).await {
//         log::error!("CRITICAL: Multi-Topic Consumer terminated with error: {:?}", e);
//         return Err(e);
//     }

//     info!("Consumer service shut down gracefully.");
    
//     Ok(())
// }

fn main(){}