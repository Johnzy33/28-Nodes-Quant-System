use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::Surreal;
//use anyhow::{Result, Context};


// Re-export the query functions for external use
pub mod queries;
pub mod db_connect;
pub mod surreal_aggregator_function;
pub mod aggregators;
pub mod candle_pattern_cal_update;
pub mod aggregtions_pipline;
pub mod pre_compute;
pub mod sql_comput;
pub mod candle;
pub  mod schema;
pub  type DB = Surreal<Client>;

pub use aggregators::*;
pub use surreal_aggregator_function::define_surreal_functions;
pub use candle_pattern_cal_update::calculate_and_update_candle_patterns;
pub use aggregtions_pipline::run_all_aggregations;
pub use candle::candle_pattern_function;
pub use sql_comput::*;