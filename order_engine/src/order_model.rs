
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide { Buy, Sell }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus { Pending, Filled, Rejected, Cancelled }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub internal_id: Uuid,        // Our tracking ID
    pub external_id: Option<i64>, // The Broker/MT5 Ticket ID
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: f64,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
}

impl Order {
    pub fn new(symbol: &str, side: OrderSide, quantity: f64) -> Self {
        Self {
            internal_id: Uuid::new_v4(),
            external_id: None,
            symbol: symbol.to_string(),
            side,
            quantity,
            status: OrderStatus::Pending,
            created_at: Utc::now(),
        }
    }
}