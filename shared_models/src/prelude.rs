// Use the native SurrealDB types directly for seamless integration
use surrealdb::{RecordId};
use surrealdb::sql::{Datetime};

pub type Id = RecordId;
pub type Timestamp = Datetime;