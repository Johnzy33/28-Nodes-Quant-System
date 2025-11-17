use anyhow::{Result, Context};
use deadpool_postgres::Pool;
use chrono::NaiveDate;
use shared_models::{AssetInfo, FciSignalOutput, DcsScore};
use tokio_postgres::types::ToSql; 
use tokio_postgres::Row;
use tokio::sync::mpsc::Sender; 
use crate::tui::events::TuiEvent; // Assuming this path is correct for your project


// --- Helper for AssetInfo fields ---
const ASSET_INFO_FIELDS: &str = 
    "id, symbol, timezone, source, name, asset_class, currency, exchange, active";
// --- Helper for DCS Score fields ---
const DCS_SCORE_FIELDS: &str = 
    "trading_date, asset_id, dcs, dcs_classification, f_reversal, f_dm1_factor, f_commitment, f_sustainability";
// --- Helper for FCI Signal fields ---
const FCI_SIGNAL_FIELDS: &str = 
    "asset_id, trading_date, ps2_name, ps1_name, ps2_bias, ps1_bias, cs_name, predicted_day_type, signal_direction, fci_score, signal_confidence, is_vetoed, pcs_anchor_score, tcs_multiplier, p_continuation_raw";


pub async fn fetch_asset_list(pool: &Pool) -> Result<Vec<AssetInfo>> {
    let client = pool.get().await.context("Failed to get client from pool")?; 
    // SELECT ALL AssetInfo fields
    let query = format!("SELECT {} FROM assets WHERE active = TRUE ORDER BY symbol ASC", ASSET_INFO_FIELDS);
    
    let rows = client.query(query.as_str(), &[]).await?;
    
    // Map ALL AssetInfo fields manually
    let assets = rows.into_iter().map(|row| AssetInfo {
        id: row.get("id"),
        symbol: row.get("symbol"),
        timezone: row.get("timezone"),
        source: row.get("source"),
        name: row.get("name"),
        asset_class: row.get("asset_class"),
        currency: row.get("currency"),
        exchange: row.get("exchange"),
        active: row.get("active"),
    }).collect();

    Ok(assets)
}

// 💥 UPDATED FUNCTION SIGNATURE AND LOGIC
pub async fn run_core_data_refresh(
    pool: &Pool, 
    sender: Sender<TuiEvent> 
) -> Result<(), anyhow::Error> {
    
    // --- Phase 0: TUI Acknowledgment ---
    let _ = sender.send(TuiEvent::DataRefreshStatus("--- Establishing Database Connection... ---".to_string())).await;
    
    let client = pool.get().await.context("Failed to get client from pool")?;
    
    // --- Phase 1: Pre-Execution Log Messages ---
    let _ = sender.send(TuiEvent::DataRefreshStatus("==================================================".to_string())).await;
    let _ = sender.send(TuiEvent::DataRefreshStatus("         STARTING MASTER CORE DATA REFRESH         ".to_string())).await;
    let _ = sender.send(TuiEvent::DataRefreshStatus("==================================================".to_string())).await;
    let _ = sender.send(TuiEvent::DataRefreshStatus("PHASE 1: REFRESHING CORE VIEWS (Data Source) - (Running... )".to_string())).await;
    let _ = sender.send(TuiEvent::DataRefreshStatus("PHASE 2: CALCULATING CONFIDENCE METRICS (TCS/PCS/FCI) - (Running...)".to_string())).await;
    let _ = sender.send(TuiEvent::DataRefreshStatus("PHASE 3: CALCULATING DAY COUNT SCORES (DCS) - (Running...)".to_string())).await;
    
    // --- Phase 2: Execution (Blocking) ---
    match client.execute("CALL refresh_core_data()", &[]).await {
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
            // FIX: Removed the extra .await inside the sender.send() call.
            let _ = sender.send(TuiEvent::DataRefreshStatus(format!("CRITICAL DB ERROR: {:?}", e))).await;
            Err(e).context("Database call to refresh_core_data() failed")
        }
    }
}

pub async fn fetch_single_fci_signal(pool: &Pool, asset_id: &str, date: Option<NaiveDate>) -> Result<Option<FciSignalOutput>> {
    let client = pool.get().await.context("Failed to get client from pool")?;

    let params: &[&(dyn ToSql + Sync)] = &[&asset_id, &date];
    // SELECT ALL FciSignalOutput fields
    let query = format!("SELECT {} FROM get_trading_signal($1, $2, NULL)", FCI_SIGNAL_FIELDS);
    
    let rows = client.query(query.as_str(), params).await?;
    
    // Map ALL FciSignalOutput fields manually
    let result = rows.first().map(|row| FciSignalOutput {
        asset_id: row.get("asset_id"),
        trading_date: row.get("trading_date"),
        ps2_name: row.get("ps2_name"),
        ps1_name: row.get("ps1_name"),
        ps2_bias: row.get("ps2_bias"),
        ps1_bias: row.get("ps1_bias"),
        cs_name: row.get("cs_name"),
        predicted_day_type: row.get("predicted_day_type"),
        signal_direction: row.get("signal_direction"),
        fci_score: row.get("fci_score"),
        signal_confidence: row.get("signal_confidence"),
        is_vetoed: row.get("is_vetoed"),
        pcs_anchor_score: row.get("pcs_anchor_score"),
        tcs_multiplier: row.get("tcs_multiplier"),
        p_continuation_raw: row.get("p_continuation_raw"),
    });

    Ok(result)
}

pub async fn fetch_all_fci_signals(pool: &Pool) -> Result<Vec<FciSignalOutput>> {
    let client = pool.get().await?;
    
    // Using the Stored Procedure Query to fetch the latest signals for all assets
    let sql = "
        WITH AllAssets AS (
            -- Step 1: Find all unique asset IDs present in your data
            SELECT DISTINCT asset_id
            FROM session_context
        )
        SELECT
            t1.*
        FROM
            -- Step 2: Call the get_trading_signal function for each asset
            (
                SELECT 
                    (get_trading_signal(
                        a.asset_id::TEXT, 
                        NULL::DATE,  -- NULL date targets the most recent session
                        NULL::TEXT   -- NULL session targets the most recent session
                    )).*
                FROM AllAssets a
            ) t1
        -- Filter out neutral or vetoed signals to only see actionable ones
        WHERE 
            t1.signal_direction <> 'Neutral' 
            AND t1.is_vetoed = FALSE
        ORDER BY 
            t1.fci_score DESC;
    ";
    
    let stmt = client.prepare(sql).await?;
    
    let rows = client.query(&stmt, &[]).await?;

    let mut signals = Vec::with_capacity(rows.len());

    for row in rows {
        // --- CRITICAL FIX: USING FIELD NAMES INSTEAD OF FRAGILE INDICES ---
        // This relies on the column names returned by get_trading_signal 
        // matching the field names in FciSignalOutput.
        let signal = FciSignalOutput {
            // Note: The field names below MUST match the actual column names in the DB function output.
            asset_id: row.get("asset_id"),
            trading_date: row.get("trading_date"),
            fci_score: row.get("fci_score"),
            signal_direction: row.get("signal_direction"),
            signal_confidence: row.get("signal_confidence"),
            is_vetoed: row.get("is_vetoed"),
            cs_name: row.get("cs_name"),
            ps1_name: row.get("ps1_name"),
            ps1_bias: row.get("ps1_bias"),
            ps2_name: row.get("ps2_name"),
            ps2_bias: row.get("ps2_bias"),
            predicted_day_type: row.get("predicted_day_type"),
            p_continuation_raw: row.get("p_continuation_raw"),
            pcs_anchor_score: row.get("pcs_anchor_score"),
            tcs_multiplier: row.get("tcs_multiplier"),
        };
        signals.push(signal);
    }

    Ok(signals)
}
pub async fn fetch_single_dcs_score(pool: &Pool, asset_id: &str, date: NaiveDate) -> Result<Option<DcsScore>> {
    let client = pool.get().await.context("Failed to get client from pool")?;
    
    // SELECT ALL DcsScore fields
    let query = format!(r#"SELECT {} FROM daily_composite_score WHERE asset_id = $1 AND trading_date = $2"#, DCS_SCORE_FIELDS);
    
    let params: &[&(dyn ToSql + Sync)] = &[&asset_id, &date];
    
    let rows = client.query(query.as_str(), params).await?;
    
    // Map ALL DcsScore fields manually
    let result = rows.first().map(|row| DcsScore {
        trading_date: row.get("trading_date"),
        asset_id: row.get("asset_id"),
        dcs: row.get("dcs"),
        dcs_classification: row.get("dcs_classification"),
        f_reversal: row.get("f_reversal"),
        f_dm1_factor: row.get("f_dm1_factor"),
        f_commitment: row.get("f_commitment"),
        f_sustainability: row.get("f_sustainability"),
    });

    Ok(result)
}

pub async fn fetch_all_dcs_scores_latest(pool: &Pool) -> Result<Vec<DcsScore>> {
    let client = pool.get().await.context("Failed to get client from pool")?;
    
    // Select all DCS fields in the outer query
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
    "#, DCS_SCORE_FIELDS); // Inject the full list of fields
    
    let rows = client.query(query.as_str(), &[]).await?;
    
    let results = rows.into_iter().map(|row| DcsScore {
        trading_date: row.get("trading_date"),
        asset_id: row.get("asset_id"),
        dcs: row.get("dcs"),
        dcs_classification: row.get("dcs_classification"),
        f_reversal: row.get("f_reversal"),
        f_dm1_factor: row.get("f_dm1_factor"),
        f_commitment: row.get("f_commitment"),
        f_sustainability: row.get("f_sustainability"),
    }).collect();
    
    Ok(results)
}