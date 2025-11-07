use anyhow::{Result, Context};
use deadpool_postgres::{Pool, Client};
use tokio_postgres::types::{ToSql, FromSql, Type};
use shared_models::metrics::{
    FromPgRow, DcsLatestScore, DailyDbsMetrics, PcsContextScore, 
    Pattern1stOrderOutcome, Pattern2ndOrderOutcome
};
use chrono::NaiveDate;

// --- Helper function to execute a query and map results ---

async fn execute_query_and_map<T: FromPgRow>(
    client: &Client,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Vec<T>> {
    
    let rows = client.query(sql, params)
        .await
        .context(format!("Failed to execute SQL query: {}", sql))?;
        
    rows.into_iter()
        .map(|row| T::from_row(row).context("Failed to map database row to Rust struct"))
        .collect::<Result<Vec<T>>>()
}

// --- Goal 2 & 3: DCS Lookups (Date or Latest) ---

/// Fetches the Daily Composite Score (DCS) for a specific date or the latest one available.
pub async fn get_dcs_by_date(pool: &Pool, asset_id: &str, date: Option<NaiveDate>) -> Result<Option<DcsLatestScore>> {
    let client = pool.get().await.context("Failed to get DB client for DCS lookup")?;
    
    let (sql, params): (String, Vec<&(dyn ToSql + Sync)>) = match date {
        // Query for specific date
        Some(d) => (
            format!("SELECT trading_date, asset_id, dcs, dcs_classification, f_reversal 
                     FROM daily_composite_score 
                     WHERE asset_id = $1 AND trading_date = $2"),
            vec![&asset_id, &d],
        ),
        // Query for the latest date
        None => (
            format!("SELECT trading_date, asset_id, dcs, dcs_classification, f_reversal 
                     FROM daily_composite_score 
                     WHERE asset_id = $1
                     ORDER BY trading_date DESC 
                     LIMIT 1"),
            vec![&asset_id],
        ),
    };
    
    let result = execute_query_and_map::<DcsLatestScore>(&client, &sql, &params).await?;
    Ok(result.into_iter().next())
}

// --- Goal 5: DBS Lookbacks (Date or Latest) ---

/// Fetches the Daily Bias Scores (DBS) for a specific date or the latest one available.
pub async fn get_dbs_by_date(pool: &Pool, asset_id: &str, date: Option<NaiveDate>) -> Result<Option<DailyDbsMetrics>> {
    let client = pool.get().await.context("Failed to get DB client for DBS lookup")?;

    let (sql, params): (String, Vec<&(dyn ToSql + Sync)>) = match date {
        // Query for specific date
        Some(d) => (
            format!("SELECT trading_date, asset_id, dbs_st_score, dbs_lt_score, dbs_ytd_score, prior_day_type, calculation_date
                     FROM daily_metrics_snapshot
                     WHERE asset_id = $1 AND trading_date = $2"),
            vec![&asset_id, &d],
        ),
        // Query for the latest date
        None => (
            format!("SELECT trading_date, asset_id, dbs_st_score, dbs_lt_score, dbs_ytd_score, prior_day_type, calculation_date
                     FROM daily_metrics_snapshot 
                     WHERE asset_id = $1
                     ORDER BY trading_date DESC 
                     LIMIT 1"),
            vec![&asset_id],
        ),
    };

    let result = execute_query_and_map::<DailyDbsMetrics>(&client, &sql, &params).await?;
    Ok(result.into_iter().next())
}

// --- Goal 1: PCS Lookups (Pattern or Latest) ---

/// Fetches PCS scores for a specific manual pattern key (Goal 1 - Manual Input).
pub async fn get_pcs_by_pattern(
    pool: &Pool, 
    asset_id: &str, 
    ps_name: &str, 
    cs_name: &str, 
    outcome: &str,
) -> Result<Vec<PcsContextScore>> {
    let client = pool.get().await.context("Failed to get DB client for PCS lookup")?;
    
    // NOTE: This assumes a view or table exists where PCS scores are aggregated by pattern.
    // For now, we query a view that joins the PCS scores with the 1st order pattern keys.
    // We filter by a sample lookback period (e.g., '1Y') if the table structure is flat.
    // However, if we assume the 'predictive_confidence_score' (PCS) is the target, we need 
    // to search using all its primary keys defined in the schema.
    
    let sql = format!(
        r#"
        SELECT asset_id, lookback_period, pcs_score, insight_label, p_day_type_conditional 
        FROM predictive_confidence_score
        WHERE asset_id = $1 
          AND ps_name = $2 
          AND cs_name = $3
          AND daily_outcome_7 = $4 
        ORDER BY lookback_period DESC
        "#
    );
    
    // We need to infer ps_bias_3 and cs_bias_3 for a full lookup, but since the user provided 
    // a simplified request (PS, CS, Outcome), we use placeholders for the full key. 
    // The actual table has 8 PK components, so the query must be more complex 
    // or rely on a simplified view. We will assume a simplified view/table is needed 
    // for this type of query, but for now we use the existing table structure:
    
    // WARNING: This query is intentionally incomplete due to missing bias values, 
    // but represents the user's request. It will likely return no results unless 
    // the user provides the bias values as well, or uses a simpler view.
    let sql = format!(
        r#"
        SELECT asset_id, lookback_period, pcs_score, insight_label, p_day_type_conditional 
        FROM predictive_confidence_score
        WHERE asset_id = $1 AND ps_name = $2 AND cs_name = $3
        ORDER BY lookback_period DESC
        "# // Simplified query to work with the user's manual input
    );

    let params: Vec<&(dyn ToSql + Sync)> = vec![&asset_id, &ps_name, &cs_name];
    
    execute_query_and_map::<PcsContextScore>(&client, &sql, &params).await
}

/// Fetches the latest PCS scores by querying the most recent completed pattern in the DCS table 
/// and attempting to find the corresponding PCS entries (Goal 1 - Latest).
pub async fn get_latest_pcs_scores(pool: &Pool, asset_id: &str) -> Result<Vec<PcsContextScore>> {
    // 1. Find the latest trading date in the DCS table.
    let latest_dcs = match get_dcs_by_date(pool, asset_id, None).await? {
        Some(dcs) => dcs,
        None => return Ok(Vec::new()),
    };
    
    // 2. Find the pattern associated with that latest trading date. 
    // CRITICAL: Since we don't have the "context" table logic ready, we use a placeholder 
    // query that assumes a common session pattern (AS->LN) and the most common outcome.
    // In a final system, a view like 'latest_completed_pattern_v' would provide this key.
    let ps_name = "NYPM"; // Placeholder for the preceding session of the session ending on the latest date
    let cs_name = "AS";   // Placeholder for the current session name
    
    // 3. Query the PCS table using the inferred latest pattern.
    let client = pool.get().await.context("Failed to get DB client for latest PCS lookup")?;
    
    let sql = format!(
        r#"
        SELECT asset_id, lookback_period, pcs_score, insight_label, p_day_type_conditional
        FROM predictive_confidence_score
        WHERE asset_id = $1 
          AND ps_name = $2 
          AND cs_name = $3 
        ORDER BY lookback_period DESC
        "#
    );

    let params: Vec<&(dyn ToSql + Sync)> = vec![&asset_id, &ps_name, &cs_name];
    
    execute_query_and_map::<PcsContextScore>(&client, &sql, &params).await
}


// --- Goal 4: 1st Order Outcome Lookup ---

/// Fetches the 1st order outcome probability for a specific pattern key.
pub async fn get_1st_order_outcome(
    pool: &Pool, 
    asset_id: &str, 
    ps_name: &str, 
    ps_bias: &str, 
    cs_name: &str, 
    cs_bias: &str, 
    outcome: &str,
) -> Result<Option<Pattern1stOrderOutcome>> {
    let client = pool.get().await.context("Failed to get DB client for 1st order lookup")?;

    let sql = format!(
        r#"
        SELECT asset_id, ps_name, ps_bias_3, cs_name, cs_bias_3, daily_outcome_7, total_attempts, success_count, p_day_type
        FROM us2000_day_type_1st_order -- Using the working asset-specific table provided by user
        WHERE asset_id = $1 AND ps_name = $2 AND ps_bias_3 = $3 
          AND cs_name = $4 AND cs_bias_3 = $5 AND daily_outcome_7 = $6
        LIMIT 1
        "#
    );
    
    let params: Vec<&(dyn ToSql + Sync)> = vec![
        &asset_id, &ps_name, &ps_bias, &cs_name, &cs_bias, &outcome
    ];
    
    let result = execute_query_and_map::<Pattern1stOrderOutcome>(&client, &sql, &params).await?;
    Ok(result.into_iter().next())
}

// --- Goal 6: 2nd Order Outcome Lookup ---

/// Fetches the 2nd order outcome probability for a specific pattern key.
pub async fn get_2nd_order_outcome(
    pool: &Pool, 
    asset_id: &str, 
    ps2_name: &str, 
    ps2_bias: &str, 
    ps1_name: &str, 
    ps1_bias: &str, 
    cs_name: &str, 
    cs_bias: &str, 
    outcome: &str,
) -> Result<Option<Pattern2ndOrderOutcome>> {
    let client = pool.get().await.context("Failed to get DB client for 2nd order lookup")?;

    let sql = format!(
        r#"
        SELECT asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name, cs_bias_7, daily_outcome_7, total_attempts, success_count, p_day_type
        FROM us2000_day_type_2nd_order -- Using the working asset-specific table provided by user
        WHERE asset_id = $1 AND ps2_name = $2 AND ps2_bias_7 = $3 
          AND ps1_name = $4 AND ps1_bias_7 = $5 AND cs_name = $6 
          AND cs_bias_7 = $7 AND daily_outcome_7 = $8
        LIMIT 1
        "#
    );

    let params: Vec<&(dyn ToSql + Sync)> = vec![
        &asset_id, &ps2_name, &ps2_bias, &ps1_name, &ps1_bias, &cs_name, &cs_bias, &outcome
    ];

    let result = execute_query_and_map::<Pattern2ndOrderOutcome>(&client, &sql, &params).await?;
    Ok(result.into_iter().next())
}