use anyhow::{Result, Context, anyhow};
use sqlx::{PgPool, FromRow}; // Use sqlx::PgPool and FromRow
use chrono::NaiveDate;
use shared_models::{AssetMetadata, FciSignalOutput, DcsScore};
use tokio::sync::mpsc::Sender; 
use crate::tui::events::TuiEvent; 

// --- Helper for AssetInfo fields (Assumed to be defined in this file or imported) ---
const ASSET_INFO_FIELDS: &str = 
    "id, symbol, timezone, source, name, asset_class, currency, exchange, active";
// --- Helper for DCS Score fields ---
const DCS_SCORE_FIELDS: &str = 
    "trading_date, asset_id, dcs, dcs_classification, f_reversal, f_dm1_factor, f_commitment, f_sustainability";
// --- Helper for FCI Signal fields ---
const FCI_SIGNAL_FIELDS: &str = 
    "asset_id, trading_date, ps2_name, ps1_name, ps2_bias, ps1_bias, cs_name, predicted_day_type, signal_direction, fci_score, signal_confidence, is_vetoed, pcs_anchor_score, tcs_multiplier, p_continuation_raw";


// NOTE: All fetching structs (AssetInfo, FciSignalOutput, DcsScore) 
// must now derive sqlx::FromRow for these functions to compile.

// --- 1. Asset List Fetch ---
pub async fn fetch_asset_list(pool: &PgPool) -> Result<Vec<AssetMetadata>> {
    let query = format!("SELECT {} FROM assets WHERE active = TRUE ORDER BY symbol ASC", ASSET_INFO_FIELDS);
    
    // 🎯 SQLX: Use query_as for automatic mapping
    let assets = sqlx::query_as::<_, AssetMetadata>(&query)
        .fetch_all(pool)
        .await
        .context("Failed to fetch asset list")?;

    Ok(assets)
}

// --- 2. Core Data Refresh (Stored Procedure Execution) ---
pub async fn run_core_data_refresh(
    pool: &PgPool, // 🎯 SQLX: Use PgPool
    sender: Sender<TuiEvent> 
) -> Result<(), anyhow::Error> {
    
    let _ = sender.send(TuiEvent::DataRefreshStatus("--- Establishing Database Connection... ---".to_string())).await;
    
    // --- Phase 1: Pre-Execution Log Messages ---
    let _ = sender.send(TuiEvent::DataRefreshStatus("==================================================".to_string())).await;
    let _ = sender.send(TuiEvent::DataRefreshStatus("         STARTING MASTER CORE DATA REFRESH         ".to_string())).await;
    let _ = sender.send(TuiEvent::DataRefreshStatus("==================================================".to_string())).await;
    let _ = sender.send(TuiEvent::DataRefreshStatus("PHASE 1: REFRESHING CORE VIEWS (Data Source) - (Running... )".to_string())).await;
    let _ = sender.send(TuiEvent::DataRefreshStatus("PHASE 2: CALCULATING CONFIDENCE METRICS (TCS/PCS/FCI) - (Running...)".to_string())).await;
    let _ = sender.send(TuiEvent::DataRefreshStatus("PHASE 3: CALCULATING DAY COUNT SCORES (DCS) - (Running...)".to_string())).await;
    
    // --- Phase 2: Execution (Blocking) ---
    // 🎯 SQLX: Execute CALL directly against the pool
    match sqlx::query("CALL refresh_core_data()").execute(pool).await {
        Ok(_) => {
            // --- Phase 3: Post-Execution Success ---
            let _ = sender.send(TuiEvent::DataRefreshStatus("PHASE 1-3 EXECUTION COMPLETE.".to_string())).await;
            let _ = sender.send(TuiEvent::DataRefreshStatus("==================================================".to_string())).await;
            let _ = sender.send(TuiEvent::DataRefreshStatus("     MASTER CORE DATA REFRESH SUCCESSFULLY COMPLETED!    ".to_string())).await;
            let _ = sender.send(TuiEvent::DataRefreshStatus("==================================================".to_string())).await;
            Ok(())
        }
        Err(e) => {
            // --- Phase 3: Post-Execution Failure ---
            let _ = sender.send(TuiEvent::DataRefreshStatus(format!("CRITICAL DB ERROR: {:?}", e))).await;
            Err(anyhow::Error::from(e).context("Database call to refresh_core_data() failed"))
        }
    }
}

// --- 3. Single FCI Signal Fetch ---
pub async fn fetch_single_fci_signal(pool: &PgPool, asset_id: &str, date: Option<NaiveDate>) -> Result<Option<FciSignalOutput>> {
    
    let query = format!("SELECT {} FROM get_trading_signal($1, $2, NULL)", FCI_SIGNAL_FIELDS);
    
    // 🎯 SQLX: Use query_as and fetch_optional
    let result = sqlx::query_as::<_, FciSignalOutput>(&query)
        .bind(asset_id)
        .bind(date)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch single FCI signal")?;

    Ok(result)
}

// --- 4. All Latest FCI Signals Fetch ---
pub async fn fetch_all_fci_signals(pool: &PgPool) -> Result<Vec<FciSignalOutput>> {
    
    let sql = "
        WITH AllAssets AS (
            SELECT DISTINCT asset_id FROM session_context
        )
        SELECT
            t1.*
        FROM
            (
                SELECT 
                    (get_trading_signal(
                        a.asset_id::TEXT, 
                        NULL::DATE,  
                        NULL::TEXT   
                    )).*
                FROM AllAssets a
            ) t1
        WHERE 
            t1.signal_direction <> 'Neutral' 
            AND t1.is_vetoed = FALSE
        ORDER BY 
            t1.fci_score DESC;
    ";
    
    // 🎯 SQLX: Use query_as and fetch_all
    let signals = sqlx::query_as::<_, FciSignalOutput>(sql)
        .fetch_all(pool)
        .await
        .context("Failed to fetch all FCI signals")?;
    
    Ok(signals)
}

// --- 5. Single DCS Score Fetch ---
pub async fn fetch_single_dcs_score(pool: &PgPool, asset_id: &str, date: NaiveDate) -> Result<Option<DcsScore>> {
    
    let query = format!(r#"SELECT {} FROM daily_composite_score WHERE asset_id = $1 AND trading_date = $2"#, DCS_SCORE_FIELDS);
    
    // 🎯 SQLX: Use query_as, bind, and fetch_optional
    let result = sqlx::query_as::<_, DcsScore>(&query)
        .bind(asset_id)
        .bind(date)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch single DCS score")?;

    Ok(result)
}

// --- 6. All Latest DCS Scores Fetch ---
pub async fn fetch_all_dcs_scores_latest(pool: &PgPool) -> Result<Vec<DcsScore>> {
    
    let query = format!(r#"
        SELECT {}
        FROM (
            SELECT
                *,
                ROW_NUMBER() OVER (PARTITION BY asset_id ORDER BY trading_date DESC) as rn
            FROM daily_composite_score
        ) AS ranked_scores
        WHERE rn = 1
        ORDER BY dcs DESC;
    "#, DCS_SCORE_FIELDS); 
    
    // 🎯 SQLX: Use query_as and fetch_all
    let results = sqlx::query_as::<_, DcsScore>(&query)
        .fetch_all(pool)
        .await
        .context("Failed to fetch all latest DCS scores")?;
    
    Ok(results)
}