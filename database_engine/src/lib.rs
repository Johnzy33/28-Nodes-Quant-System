use surrealdb::engine::remote::ws::{Client};
use surrealdb::Surreal;
//use anyhow::{Result, Context};
//use db_connect::connect;

// Re-export the query functions for external use

pub type DB = Surreal<Client>;
pub mod runtime;


