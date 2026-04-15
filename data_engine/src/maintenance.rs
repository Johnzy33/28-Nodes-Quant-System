

use std::sync::Arc;
use chrono::{ Utc};
use log::{info, warn};

use shared_models::db_models::{self as db};

use async_trait::async_trait; 

use tokio::net::{TcpStream};
use tokio::io::AsyncWriteExt;

use shared_models::data_model::IngestionCoordinatorConfig;
use shared_models::data_model::DataService;
use crate::traits::{DataServiceBase, DataMaintenanceExt};
use surrealdb_types::{Datetime};








#[async_trait]
impl DataMaintenanceExt for DataService {


    async fn send_startup_handshake(
        &self,
        dbs: &Arc<db::AppDatabases>,
        socket: &mut TcpStream,
        config: &IngestionCoordinatorConfig,
    )  -> db::AppResult<()> {
        // let offset = BROKER_OFFSET.load(Ordering::SeqCst);
        let offset = self.get_broker_offset();
        let mut sub_list = Vec::new();

        if offset == 0 {
            warn!("Handshake using Offset 0. This is usually only correct for London/GMT brokers.");
        }

        for asset in config.assets.iter().filter(|a| a.enabled) {
    
            let record_id = db::make_composite_id("assets", &[&asset.mt5_symbol, &config.data_source_id]);

    
            let hwm: Option<Datetime> = self.get_hwm(&dbs.disk, "market_data", &record_id.clone()).await?;

            // 3. Time Logic: Convert UTC DB time to MT5 Broker Time
            let final_hwm = match hwm {
                Some(dt) => {
                    // Surreal Datetime to Unix Timestamp (Seconds)
                    let utc_ts = dt.timestamp();
                    // Adjust for Broker Offset and subtract 10mins (buffer for safety/gaps)
                    (utc_ts + offset) - 300 
                }
                None => {
                    // Default: 24 hours ago in Broker Time
                    (Utc::now().timestamp() + offset) - 86400
                }
            };
            // 4. Construct JSON for MT5
            sub_list.push(serde_json::json!({
                "mt5": asset.mt5_symbol,
                "tf": asset.timeframe,
                "hwm": final_hwm,
                "system_symbol": asset.system_symbol.to_uppercase()
            }));

            // 5. Logging with proper formatting
           
            let displaytime = Datetime::from_timestamp(final_hwm, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "No Data".to_string());

            info!("Prepared SYNC for {} from HWM: {}", asset.system_symbol, displaytime);
        
        }

        //  Serialize and Prepare Binary Packet
        let handshake_json = serde_json::to_string(&sub_list)?;
        let json_bytes = handshake_json.as_bytes();
        let json_len = json_bytes.len() as u32;

        // Send Packet: [Type 255][Length u32 LE][Payload]
        let mut packet = Vec::with_capacity(5 + json_bytes.len());
        packet.push(255u8);
        packet.extend_from_slice(&json_len.to_le_bytes());
        packet.extend_from_slice(json_bytes);

        socket.write_all(&packet).await?;
        socket.flush().await?;
    
        info!("Handshake dispatched. {} bytes sent to MT5.", json_len);
        Ok(())
    }


}