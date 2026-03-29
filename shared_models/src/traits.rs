


use async_trait::async_trait;
// use crate::market_data::MarketData_old;

#[async_trait]
pub trait MarketDataHandler: Send + Sync {
    // This is the zero-latency tap called by the ingestion engine
    // async fn on_price_update(&self, data: &MarketData_old);
}