
use crate::order_model::Order;
use anyhow::Result;

#[async_trait::async_trait]
pub trait OrderExecutor: Send + Sync {
    async fn execute(&self, order: &Order) -> Result<ExecutionReceipt, String>;
}

pub struct ExecutionReceipt {
    pub ticket: i64,
    pub fill_price: f64,
}