use serde::{Deserialize, Serialize};
use crate::prelude::{Id, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
    IOC,
    FOK,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    Pending,
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Order {
    pub id: Option<Id>,
    pub asset_id: Id,
    pub client_order_id: Option<String>, // external client id
    pub side: OrderSide,
    pub qty: f64,
    pub price: Option<f64>, // None for market orders
    pub order_type: OrderType,
    pub status: Option<OrderStatus>,
    pub created_ts: Option<Timestamp>,
    pub updated_ts: Option<Timestamp>,
    pub tags: Option<Vec<String>>,
    pub meta: Option<serde_json::Value>, // arbitrary metadata
}

impl Order {
    pub fn new(asset_id: Id, side: OrderSide, qty: f64, order_type: OrderType) -> Self {
        Self {
            id: None,
            asset_id,
            client_order_id: None,
            side,
            qty,
            price: None,
            order_type,
            status: Some(OrderStatus::Pending),
            created_ts: None,
            updated_ts: None,
            tags: None,
            meta: None,
        }
    }

    pub fn is_buy(&self) -> bool {
        matches!(self.side, OrderSide::Buy)
    }
}
