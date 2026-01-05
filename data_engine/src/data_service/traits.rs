
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

use crate::watchdog::WatchdogState;
use shared_models::data_model::DataService;
use shared_models::market_data::MarketData;
use shared_models::traits::MarketDataHandler;
use shared_models::time_utils;
use crate::producer_config::IngestionCoordinatorConfig;

#[async_trait]
pub trait DataServiceBase {
    async fn get_market_data_high_watermark(&self, asset_id: &str) -> Result<Option<i64>>;
    async fn fetch_all_active_asset_ids(&self) -> Result<Vec<String>>;
    async fn refresh_session_base_mv(&self) -> Result<()>;
    async fn refresh_daily_base_mv(&self) -> Result<()>;
    async fn refresh_bar_base_mv(&self) -> Result<()>;
    async fn sync_asset_data(&self, asset_id: &str) -> Result<()>;
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
    async fn run_ingestion(
        &self,
        config: Arc<IngestionCoordinatorConfig>,
        orchestrator: Arc<dyn MarketDataHandler>,
        watchdog_state: WatchdogState,
        sync_tx: watch::Sender<bool>,
    ) -> Result<()>;

    // Internal logic helpers
    async fn execute_ingestion_batch(
        &self,
        batch: &mut Vec<MarketData>,
        last_msg: &Option<OwnedMessage>,
        watchdog_state: &WatchdogState,
        consumer: &StreamConsumer,
    ) -> Result<()>;

     async fn check_actual_lag(&self, consumer: &StreamConsumer, topics: &[String]) -> bool ;
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
}