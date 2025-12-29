
pub mod ingestion;
pub mod runtime;
//pub mod producer_config;

//pub use producer_config::*;
pub use runtime::{setup_database_pool};
pub use deadpool_postgres::Pool;


