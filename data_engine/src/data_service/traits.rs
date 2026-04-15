

use anyhow::Result;
use async_trait::async_trait;
use shared_models::data_model::*;
use surrealdb_types::SurrealValue;

use std::sync::Arc;
// use std::error::Error;
use tokio::net::{TcpStream};
use crate::watchdog::WatchdogState;
use shared_models::data_model::DataService;
use shared_models::market_data::{MarketData};
use shared_models::data_model::IngestionCoordinatorConfig;
use tokio_util::sync::CancellationToken;

use shared_models::db_models::{AppDatabases};
use shared_models::db_models as db;
use shared_models::data_model as dm;
use surrealdb::types::{RecordId};
use shared_models::db_models::MonitorUpdates;

use surrealdb::Connection;

#[async_trait]
pub trait DataServiceBase {

    async fn run(
        &self, 
        addr: &str, 
        dbs: Arc<AppDatabases>,
        data_service: Arc<DataService>, // This is the Arc holding 'self'
        state: WatchdogState
      
    ) -> db::AppResult<()> ;

    async fn get_hwm<T, C: Connection>(
        &self,
        dbs: &surrealdb::Surreal<C>,
        table: &str,
        asset_id: &RecordId, // This is the assets:['TICKER', 'SOURCE'] part
    ) -> surrealdb::Result<Option<T>>
    where
        T: serde::de::DeserializeOwned + Send + Sync + SurrealValue,;
}



#[async_trait]
pub trait DataViewExt {

}


#[async_trait]
pub trait DataIngestionExt {

    async fn broker_ingestion(
        &self,
        mut socket: TcpStream,
        dbs: Arc<AppDatabases>,
        shutdown: CancellationToken,
        state: WatchdogState
    ) -> db::AppResult<()>;

    async fn run_startup_gap_detection(
        &self, 
        socket: &mut TcpStream, 
        state: &WatchdogState
    ) -> db::AppResult<()> ;

   

    async fn request_manual_sync(
        &self, 
        socket: &mut TcpStream, 
        asset_id: &RecordId, 
        start_ts_ms: i64
    ) -> Result<()>;

    async fn process_tick_header(
        &self, 
        socket: &mut TcpStream, 
        context: &IngestionContext,
        state: &WatchdogState,
        dbs: &Arc<AppDatabases> 
    )  -> db::AppResult<()> ;

    async fn process_dna_header(
        &self, 
        socket: &mut TcpStream, 
        dbs: Arc<AppDatabases>,
    ) -> db::AppResult<()>;



    async fn process_tick_backfill_headerbatch(
        &self,
        socket: &mut TcpStream,
        context: &IngestionContext,
        dbs: &Arc<AppDatabases>,
    ) -> db::AppResult<()>;

    async fn handle_backfill_aggregation(
        &self,
        tick: &dm::MultiplexedTick,
        context: &IngestionContext,
        dbs: &Arc<AppDatabases>,
    ) -> db::AppResult<bool>;


    async fn flush_backfill_asset(
        &self, 
        asset_id: &RecordId, 
        dbs: &Arc<AppDatabases>
    ) ;

    async fn handle_sync_complete(
        &self,
        socket: &mut TcpStream,
        dbs: &Arc<db::AppDatabases>,
        state: &WatchdogState,
    ) -> db::AppResult<()>;
    

    async fn process_session_header(
        &self,
        socket: &mut TcpStream,
        context: &IngestionContext,
    )  -> db::AppResult<()> ;

    async fn process_negotiation_header(
        &self, 
        socket: &mut TcpStream, 
        context: &IngestionContext
    )  -> db::AppResult<()> ;

    async fn ingest_tick(
        &self,
        dbs: Arc<AppDatabases>, 
        update: MonitorUpdates, 
        symbol: &str,
        source: &str,
        table: &str,
        broker_offset_seconds: i64 
    ) -> surrealdb::Result<()> ;

    fn handle_socket_error(
        &self, 
        e: std::io::Error
    );

    fn get_asset_record_id(
        &self, 
        asset_bytes: &[u8], 
        source: &str
    ) -> RecordId ;


    async fn perform_mirror_cycle(
        &self,
        dbs: &Arc<AppDatabases>,
        state: &WatchdogState,
        is_shutdown: bool
    );
    async fn ingest_market_data<C: Connection>( // Add <C: Connection>
        &self,
        db: &surrealdb::Surreal<C>, // Change Any to C
        data: Vec<db::MarketData>,
    ) -> surrealdb::Result<()>;

    fn transform_to_standard_market_data(
        &self, 
        batch: Vec<MarketData>
    ) -> Vec<db::MarketData>;
}




#[async_trait]
pub trait DataMaintenanceExt {

    async fn send_startup_handshake(
        &self,
        dbs: &Arc<AppDatabases>,
        socket: &mut TcpStream,
        config: &IngestionCoordinatorConfig,
    )  -> db::AppResult<()>;
}


