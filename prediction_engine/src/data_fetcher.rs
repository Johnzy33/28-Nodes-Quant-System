// prediction_engine/src/data_fetcher.rs

use deadpool_postgres::Pool;
use anyhow::{Context, Result};
use tokio_postgres::Row;

// --- 1. SQL Query Templates ---

/// SQL template for fetching TCS 1st Order (PS1 -> CS) data.
pub const TCS_1ST_ORDER_QUERY_TEMPLATE: &str = "
WITH LatestContext AS (
    SELECT DISTINCT ON (asset_id)
        asset_id, ps1_name, ps1_bias_7, cs_name
    FROM
        session_context
    WHERE
        asset_id = '{asset_id}' 
        AND trading_date = CURRENT_DATE -- Assuming current date for real-time analysis
    ORDER BY
        asset_id, session_end_ts DESC
)
SELECT
    lc.ps1_name AS current_ps1_session,
    lc.ps1_bias_7 AS current_ps1_bias,
    lc.cs_name AS current_cs_session,
    tcs.cs_bias AS predicted_cs_bias,
    tcs.lookback_period,
    tcs.p_transition_conditional,
    tcs.p_cs_base,
    tcs.tcs_score
FROM
    tcs_1st_order tcs
INNER JOIN
    LatestContext lc ON tcs.asset_id = lc.asset_id
WHERE
    tcs.ps1_name = lc.ps1_name
    AND tcs.ps1_bias = lc.ps1_bias_7
ORDER BY
    tcs.lookback_period, tcs.tcs_score DESC;
";

/// SQL template for fetching TCS 2nd Order (PS2 -> PS1 -> CS) data.
pub const TCS_2ND_ORDER_QUERY_TEMPLATE: &str = "
WITH LatestContext AS (
    SELECT DISTINCT ON (asset_id)
        asset_id, ps2_name, ps2_bias_7, ps1_name, ps1_bias_7, cs_name
    FROM
        session_context
    WHERE
        asset_id = '{asset_id}'
        AND trading_date = CURRENT_DATE
    ORDER BY
        asset_id, session_end_ts DESC
)
SELECT
    lc.ps2_name AS current_ps2_session,
    lc.ps2_bias_7 AS current_ps2_bias,
    lc.ps1_name AS current_ps1_session,
    lc.ps1_bias_7 AS current_ps1_bias,
    lc.cs_name AS current_cs_session,
    tcs.cs_bias AS predicted_cs_bias,
    tcs.lookback_period,
    tcs.p_transition_conditional,
    tcs.p_cs_base,
    tcs.tcs_score
FROM
    tcs_2nd_order tcs
INNER JOIN
    LatestContext lc ON tcs.asset_id = lc.asset_id
WHERE
    tcs.ps2_name = lc.ps2_name
    AND tcs.ps2_bias = lc.ps2_bias_7
    AND tcs.ps1_name = lc.ps1_name
    AND tcs.ps1_bias = lc.ps1_bias_7
ORDER BY
    tcs.lookback_period, tcs.tcs_score DESC;
";

// --- 2. Generic Fetching Function ---

/// A trait to constrain the types we can fetch, requiring the Row mapping function.
pub trait FromRow: Sized {
    fn from_row(row: &Row) -> Self;
}

// Implement the trait for your structs (if not already done in shared_models)
// Since we used impl X { fn from_row } above, let's skip the trait definition 
// for simplicity and use a function pointer in the fetcher.

/// Fetches TCS data using a connection pool and formats the query with the asset_id.
/// 
/// The mapper function is used to convert each tokio_postgres::Row into the target struct.
pub async fn fetch_tcs_data<T>(
    pool: &Pool,
    query_template: &str,
    asset_id: &str,
    row_mapper: fn(&Row) -> T, // Now using the correct Row type and signature
) -> Result<Vec<T>> {
    let client = pool.get().await.context("Failed to get connection from pool")?;

    // IMPORTANT: Replacing the asset_id directly in the query string is necessary 
    // because the asset_id is used within the CTE *inside* the SQL template.
    let query_string = query_template.replace("{asset_id}", asset_id);

    // Execute the query
    let rows = client.query(&query_string, &[]).await.context("Failed to execute SQL query")?;

    // Map the returned rows to the target struct
    let results = rows.iter().map(row_mapper).collect();
    
    Ok(results)
}