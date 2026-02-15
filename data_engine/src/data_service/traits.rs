
// data_engine/src/data_service/traits.rs
use anyhow::Result;
use async_trait::async_trait;
use shared_models::data_model::*;
use shared_models::{SessionContextData, EightContextData};
use crate::persistable::Persistable;
use crate::data_views::WatermarkValue;
use chrono::Duration;

use rdkafka::consumer::{Consumer, StreamConsumer, CommitMode};
use rdkafka::config::ClientConfig;
use rdkafka::message::{Message, OwnedMessage};
use rdkafka::TopicPartitionList;
use rdkafka::producer::FutureProducer;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::watch;
use anyhow::{Context, };
use log::{info, error, debug, warn};
use sqlx::{PgPool, QueryBuilder, Postgres};
use std::error::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

use crate::watchdog::WatchdogState;
use shared_models::data_model::DataService;
use shared_models::market_data::MarketData;
use shared_models::traits::MarketDataHandler;
use shared_models::time_utils;
use shared_models::data_model::IngestionCoordinatorConfig;
use tokio_util::sync::CancellationToken;
use smol_str::SmolStr;
use shared_models::db_models::AppDatabases;

#[async_trait]
pub trait DataServiceBase {
    async fn get_market_data_high_watermark(&self, asset_id: &str) -> Result<Option<i64>>;
    async fn fetch_all_active_asset_ids(&self) -> Result<Vec<String>>;
    async fn refresh_session_base_mv(&self) -> Result<()>;
    async fn refresh_daily_base_mv(&self) -> Result<()>;
    async fn refresh_bar_base_mv(&self) -> Result<()>;
    async fn sync_asset_data(&self, asset_id: &str) -> Result<()>;
    async fn run(
        &self,
    addr: &str, 
    data_service: Arc<DataService>,
    //config: IngestionCoordinatorConfig
    state: WatchdogState
) -> Result<(), Box<dyn Error>> ;

    async fn run_new(
            &self, 
            addr: &str, 
            dbs: Arc<AppDatabases>,
            data_service: Arc<DataService>, // This is the Arc holding 'self'
            state: WatchdogState
        //  config: IngestionCoordinatorConfig
        ) -> Result<(), Box<dyn Error>> ;
}

#[async_trait]
pub trait DataPersistExt {
    async fn persist_data<T>(&self, items: &[T]) -> Result<()> 
    where T: Persistable + Send + Sync;
}

#[async_trait]
pub trait DataViewExt {
    async fn get_hwm<T>(
        &self, 
        table: &str, 
        column: &str, 
        asset_id: &str
    ) -> Result<Option<T>> where
        T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,;
    
    async fn fetch_buffered<T, H>(
        &self, 
        query_base: &str, 
        asset_id: &str, 
        hwm: Option<H>, 
        buffer: Duration,
        filter_col: &str, // Explicitly pass the column name (e.g., "month_start")
    ) -> Result<Vec<T>>
    where 
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
        H: Into<WatermarkValue> + Copy + Send,;
    async fn calculate_session_and_context(&self, asset_id: &str,lookback_hours: Option<i64>) -> Result<(Vec<ClassifiedSession>, Vec<SessionContextData>)>;
    async fn calculate_8hr_context_and_blocks(&self, asset_id: &str) -> Result<(Vec<Classified8HrBlock>, Vec<EightContextData>)>;
    async fn daily_views(&self, asset_id: &str,lookback_days: Option<i64>) -> Result<Vec<ClassifiedDailyView>>;
    async fn weekly_views(&self, asset_id: &str, lookback: Option<i64>) -> Result<Vec<ClassifiedWeeklyView>>;
    async fn monthly_views(&self, asset_id: &str, lookback_days: Option<i64>) -> Result<Vec<ClassifiedMonthlyView>>;
    async fn yearly_views(&self, asset_id: &str,lookback_years: Option<i32> ) -> Result<Vec<ClassifiedYearlyView>>;
}

#[async_trait]
pub trait DataOrchestratorExt {
    async fn assets_master_etl(&self) -> Result<()>;
     fn print_summary(&self, total: usize, success: usize, errors: &[String]);
}

#[async_trait]
pub trait DataIngestionExt {
    // Helper to setup the consumer since DataService needs to initialize it
    fn create_consumer(&self, group_id: &str) -> Result<StreamConsumer>;

    // The main ingestion entry point
    // async fn run_ingestion(
    //     &self,
    //     config: Arc<IngestionCoordinatorConfig>,
    //     orchestrator: Arc<dyn MarketDataHandler>,
    //     watchdog_state: WatchdogState,
    //     sync_tx: watch::Sender<bool>,
    // ) -> Result<()>;

    // Internal logic helpers
    async fn execute_ingestion_batch(
        &self,
        batch: &mut Vec<MarketData>,
        // last_msg: &Option<OwnedMessage>,
        // watchdog_state: &WatchdogState,
        // consumer: &StreamConsumer,
    ) -> Result<()>;

     async fn check_actual_lag(&self, consumer: &StreamConsumer, topics: &[String]) -> bool ;
    
    async fn handle_mt5_ingestion(
        &self,
    mut socket: TcpStream,
   // data_service: Arc<DataService>,
    //config: Arc<IngestionCoordinatorConfig>,
    shutdown: CancellationToken,
    state: WatchdogState,
) -> Result<(), Box<dyn Error>>;

    async fn handle_mt5_ingestion_new(
        &self,
        mut socket: TcpStream,
        dbs: Arc<AppDatabases>,
        shutdown: CancellationToken,
        state: WatchdogState
    ) -> Result<(), Box<dyn Error>> ;

    async fn request_manual_sync(&self, socket: &mut TcpStream, asset_id: &str, start_ts_ms: i64) -> Result<()>;
    async fn process_tick_header(
        &self, 
        socket: &mut TcpStream, 
        context: &IngestionContext,
        state: &WatchdogState,
        batch: &mut Vec<MarketData>
    ) -> Result<(), Box<dyn Error>> ;

     async fn process_dna_header(
        &self, 
        socket: &mut TcpStream, 
        dbs: Arc<AppDatabases>,
    ) -> Result<(), Box<dyn Error>> ;

     async fn process_bar_header(
        &self,
        socket: &mut TcpStream,
        context: &IngestionContext,
        batch: &mut Vec<MarketData>,
    ) -> Result<(), Box<dyn Error>> ;

     async fn process_session_header(
        &self,
        socket: &mut TcpStream,
        context: &IngestionContext,
    ) -> Result<(), Box<dyn Error>> ;

    async fn process_negotiation_header(
        &self, 
        socket: &mut TcpStream, 
        context: &IngestionContext
    ) -> Result<(), Box<dyn Error>> ;

    fn handle_socket_error(&self, e: std::io::Error);
    async fn flush_batch_if_needed(&self, batch: &mut Vec<MarketData>) -> Result<(), Box<dyn Error>>;

    fn resolve_asset_id(&self, asset_bytes: &[u8], context: &IngestionContext) -> SmolStr;
    fn resolve_asset_id_str(&self, name: &str, context: &IngestionContext) -> SmolStr;


 async fn handle_tick(&self, tick_data: MarketData,bid: f64, ask: f64, broker_m1_vol: i64) -> Option<MarketData>;
 async fn flush_live_aggregator(&self) -> Result<()>;
 fn log_aggregator_status(&self);
 fn check_heartbeat(&self) -> Vec<MarketData>;
}


// #[async_trait]
// pub trait DataIngestionExt {
//     fn create_consumer(&self, group_id: &str) -> Result<StreamConsumer>;
//     async fn run_ingestion(&self, config: Arc<IngestionCoordinatorConfig>, orchestrator: Arc<dyn MarketDataHandler>, watchdog: WatchdogState, sync_tx: watch::Sender<bool>) -> Result<()>;
//     async fn execute_ingestion_batch(&self, batch: &mut Vec<MarketData>, last_msg: &Option<OwnedMessage>, watchdog: &WatchdogState, consumer: &StreamConsumer) -> Result<()>;
// }

#[async_trait]
pub trait DataMaintenanceExt {
    async fn publish_startup_sync(&self, producer: Arc<FutureProducer>, config: Arc<IngestionCoordinatorConfig>) -> Result<()>;
    async fn start_maintenance_loop(&self, producer: Arc<FutureProducer>, config: Arc<IngestionCoordinatorConfig>) -> Result<()>;
    // Internal helper for the loop
    async fn dispatch_heal_command(&self, producer: &FutureProducer, config: &IngestionCoordinatorConfig, hours: i64) -> Result<()>;
    async fn send_startup_handshake(
        &self,
    socket: &mut TcpStream,
    //data_service: &DataService,
    config: &IngestionCoordinatorConfig,
) -> Result<(), Box<dyn Error>> ;
}