use serde::{Deserialize, Serialize};
use crate::prelude::{Id, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Asset {
    pub id: Option<Id>,       // e.g., "assets:US2000" or UUID
    pub symbol: String,       // "US2000"
    pub name: Option<String>, // "Russell 2000"
    pub asset_class: Option<String>, // "Index", "Forex", etc.
    pub currency: Option<String>,
    pub tick_size: Option<f64>,
    pub lot_size: Option<f64>,
    pub price_decimals: Option<u8>,
    pub exchange: Option<String>,
    pub timezone: Option<String>,
    pub active: Option<bool>,
    pub first_listed_ts: Option<Timestamp>,
}

impl Asset {
    pub fn new<S: Into<String>>(symbol: S) -> Self {
        Self {
            id: None,
            symbol: symbol.into(),
            name: None,
            asset_class: None,
            currency: None,
            tick_size: None,
            lot_size: None,
            price_decimals: None,
            exchange: None,
            timezone: None,
            active: Some(true),
            first_listed_ts: None,
        }
    }

    pub fn with_id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
}
