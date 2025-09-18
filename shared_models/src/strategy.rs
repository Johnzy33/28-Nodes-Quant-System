use serde::{Deserialize, Serialize};
use crate::prelude::{Id, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Strategy {
    pub id: Option<Id>,
    pub name: String,
    pub description: Option<String>,
    pub applicable_assets: Option<Vec<Id>>, // array<record<assets>>
    pub params: Option<serde_json::Value>, // arbitrary JSON config for the strategy
    pub active: Option<bool>,
    pub created_ts: Option<Timestamp>,
}

impl Strategy {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            id: None,
            name: name.into(),
            description: None,
            applicable_assets: None,
            params: None,
            active: Some(true),
            created_ts: None,
        }
    }
}
