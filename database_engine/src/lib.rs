use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::Surreal;
//use anyhow::{Result, Context};
//use db_connect::connect;

// Re-export the query functions for external use
pub mod queries;
pub mod db_connect;
pub  mod schema;
pub type DB = Surreal<Client>;


