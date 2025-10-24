
pub mod ingestion;
pub mod config;
pub mod runtime;
pub mod schema_setup;

pub use config::{ConsumerConfig};
pub use runtime::{setup_database_pool};
pub use schema_setup::apply_schema;
pub use deadpool_postgres::Pool;


