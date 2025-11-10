use anyhow::{anyhow, Result};
use deadpool_postgres::{Client, Pool};
use database_engine::runtime::setup_database_pool; 
use shared_models::asset_models::AssetInfo; 
use tokio_postgres::{Row, types::Type};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap; 

/// Represents the session context (A1) from the session_context table.
#[derive(Debug, Clone)]
pub struct CurrentSessionContext {
    pub cs_name: Option<String>,
    pub cs_bias_7: Option<String>, 
    pub ps1_name: Option<String>,
    pub ps1_bias_7: Option<String>, 
    pub ps2_name: Option<String>,
    pub ps2_bias_7: Option<String>, 
    pub found_session_end_ts: Option<String>,
}

/// Represents a single aggregated TCS score (B1 or B2).
#[derive(Debug, Clone)]
pub struct TcsScore {
    pub order: u8, 
    pub score: f64,
    pub bias_strength: f64, 
    pub predicted_session: String, 
    pub predicted_bias: String,    
    pub lookback_period: String,            // e.g., "1Y", "6M", "ALL"
    pub p_transition_conditional: f64,      // P(Conditional)
    pub p_cs_base: f64,                     // P(Base)
}

impl TcsScore {
    /// Helper to deserialize a database row into TcsScore, including all probability components.
    pub fn from_row(row: &Row, order: u8) -> Result<Self> {
        Ok(TcsScore {
            order,
            score: row.get("tcs_score"),
            predicted_session: row.get("cs_name"),
            predicted_bias: row.get("cs_bias"),
            bias_strength: row.get("tcs_score"), // TEMP: Placeholder using score
            // Map new fields
            lookback_period: row.get("lookback_period"),
            p_transition_conditional: row.get("p_transition_conditional"),
            p_cs_base: row.get("p_cs_base"),
        })
    }
}


// --- SQL Query Definitions ---

const Q_A2_ASSET_LIST: &str = r#"
    SELECT id, symbol, name FROM assets WHERE active = TRUE ORDER BY symbol;
"#;

// Q_A1: Fetch the latest context for the asset before the given date.
const Q_A1_SESSION_CONTEXT: &str = r#"
    SELECT
        cs_name, 
        CAST(cs_bias_7 AS TEXT) as cs_bias_7, 
        ps1_name, 
        CAST(ps1_bias_7 AS TEXT) as ps1_bias_7, 
        ps2_name, 
        CAST(ps2_bias_7 AS TEXT) as ps2_bias_7, 
        session_end_ts
    FROM
        session_context
    WHERE
        asset_id = $1 AND trading_date <= $2
    ORDER BY
        trading_date DESC, session_end_ts DESC
    LIMIT 1;
"#;

// Q_B1: CORRECT LOGIC - Returns ALL possible transitions (cs_bias, cs_name) 
// and includes Lookback Period and probability components.
const Q_B1_TCS_1ST_ORDER_TEMPLATE: &str = r#"
    SELECT 
        lookback_period,
        tcs_score, 
        cs_bias, 
        cs_name,
        p_transition_conditional,
        p_cs_base
    FROM tcs_1st_order
    WHERE asset_id = $1
      AND ps1_name = $2
      AND ps1_bias = $3 
    ORDER BY lookback_period DESC, tcs_score DESC;  -- <-- FIX: Order by lookback and score
"#;

// Q_B2: CORRECT LOGIC - Returns ALL possible transitions for the 2nd order context.
const Q_B2_TCS_2ND_ORDER_TEMPLATE: &str = r#"
    SELECT 
        lookback_period,
        tcs_score, 
        cs_bias, 
        cs_name,
        p_transition_conditional,
        p_cs_base
    FROM tcs_2nd_order
    WHERE asset_id = $1
      AND ps2_name = $2
      AND ps2_bias = $3
      AND ps1_name = $4
      AND ps1_bias = $5
    ORDER BY lookback_period DESC, tcs_score DESC; -- <-- FIX: Order by lookback and score
"#;


// --- Database Executor ---

/// Manages the connection pool for the CLI operations.
pub struct DbExecutor {
    pool: Pool,
}

impl DbExecutor {
    pub async fn new() -> Result<Self> {
        let pool = setup_database_pool().await?;
        Ok(DbExecutor { pool })
    }

    async fn get_client(&self) -> Result<Client> {
        self.pool.get().await.map_err(|e| anyhow!("Failed to get database client: {}", e))
    }

    pub async fn fetch_asset_list(&self) -> Result<Vec<AssetInfo>> {
        let client = self.get_client().await?;
        
        let rows = client.query(Q_A2_ASSET_LIST, &[]).await
            .map_err(|e| anyhow!("Failed to execute asset list query: {}", e))?;

        let assets = rows.into_iter()
            .map(|row| AssetInfo::from_row(&row)) 
            .collect();
            
        Ok(assets)
    }

    // --- A1 Implementation: Session Context Fetcher ---
    pub async fn fetch_session_context(&self, asset_id: &str, date: NaiveDate) -> Result<CurrentSessionContext> {
        let client = self.get_client().await?;
        
        let rows = client.query(Q_A1_SESSION_CONTEXT, &[&asset_id, &date]).await
            .map_err(|e| anyhow!("Failed to fetch session context: {}", e))?;
            
        let row = rows.into_iter().next()
            .ok_or_else(|| anyhow!("No session context found for asset '{}' on or before {}", asset_id, date))?;

        Ok(CurrentSessionContext {
            cs_name: row.try_get("cs_name").ok(),
            cs_bias_7: row.try_get("cs_bias_7").ok(),
            ps1_name: row.try_get("ps1_name").ok(),
            ps1_bias_7: row.try_get("ps1_bias_7").ok(),
            ps2_name: row.try_get("ps2_name").ok(),
            ps2_bias_7: row.try_get("ps2_bias_7").ok(),
            found_session_end_ts: row.try_get("session_end_ts").ok().map(|ts: chrono::DateTime<Utc>| ts.to_string()),
        })
    }

    /// **UPDATED:** Maps the user-selected chain string (PS1 -> CS format) to the PS1/PS2 names.
    /// PS1 is extracted from the label, and PS2 is determined by the fixed circular path.
    fn get_chain_context_names(chain_type: &str) -> Result<(String, Option<String>)> {
        // Defines the known circular dependency: PS1 -> PS2 (The session that precedes PS1)
        let ps1_to_ps2_map: HashMap<&str, &str> = HashMap::from([
            // PS1 -> PS2
            ("NYPM", "NYL"),    // Predicting AS
            ("AS", "NYPM"),     // Predicting LN
            ("LN", "AS"),       // Predicting NYAM
            ("NYAM", "LN"),     // Predicting NYL
            ("NYL", "NYAM"),    // Predicting NYPM
        ]);
        
        let parts: Vec<&str> = chain_type.split('_').collect();

        // 1. Handle TCS_DEFAULT separately (it uses A1 context names)
        if chain_type == "TCS_DEFAULT" {
            return Ok(("DEFAULT_PS1".to_string(), Some("DEFAULT_PS2".to_string())));
        }

        // 2. Extract PS1 Name from the label (e.g., TCS_LN_NYAM -> PS1=LN)
        // PS1 is the second element in the label after "TCS"
        let ps1_name = parts.get(1).copied().ok_or_else(|| {
            anyhow!("Invalid chain label format: Missing PS1 name.")
        })?;
        
        // 3. Look up the corresponding PS2 name using the fixed circular path
        let ps2_name = ps1_to_ps2_map.get(ps1_name).copied();
        
        if ps2_name.is_none() {
            return Err(anyhow!("Internal logic error: PS1 name '{}' from chain '{}' does not have a defined sequential PS2.", 
                ps1_name, chain_type));
        }

        Ok((ps1_name.to_string(), ps2_name.map(|s| s.to_string())))
    }

    
    // --- NEW HELPER: Runs a specific B1 or B2 query using the provided biases ---
    async fn run_tcs_transition_query(
        &self, 
        client: &Client, 
        asset_id: &str, 
        query: &str, 
        order: u8, 
        // These are the *session names* determined by the TUI Chain selection/A1 context
        ps1_name: &str, 
        ps2_name: Option<&str>,
        // These are the *user-defined* or *auto-used* biases
        ps1_bias: &str,
        ps2_bias: Option<&str>,
    ) -> Result<Vec<TcsScore>> {
        let mut scores = Vec::new();

        match order {
            1 => {
                let params: [&(dyn tokio_postgres::types::ToSql + Sync); 3] = [
                    &asset_id,
                    &ps1_name, 
                    &ps1_bias, 
                ];
                let rows = client.query(query, &params).await?;
                for row in rows {
                    scores.push(TcsScore::from_row(&row, 1)?);
                }
            }
            2 => {
                // Ensure all required fields for Q_B2 are present
                if let (Some(p2_name), Some(p2_bias)) = (ps2_name, ps2_bias) {
                    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 5] = [
                        &asset_id,
                        &p2_name,
                        &p2_bias,
                        &ps1_name,
                        &ps1_bias,
                    ];
                    let rows = client.query(query, &params).await?;
                    for row in rows {
                        scores.push(TcsScore::from_row(&row, 2)?);
                    }
                } else {
                     return Err(anyhow!("Cannot run 2nd Order query: Missing PS2 name or bias in Chain mapping."));
                }
            }
            _ => return Err(anyhow!("Invalid order for TCS query: {}", order)),
        }
        
        Ok(scores)
    }
    
    /// Generates a complete TCS Predictive Analysis Report (Queries A1, B1, B2).
    pub async fn generate_tcs_report(
        &self, 
        asset_symbol: String, 
        asset_id: String, 
        date: NaiveDate, 
        ps1_bias_input: String, 
        ps2_bias_input: Option<String>,
        selected_chain: String, // <-- The TUI selected chain path
    ) -> Result<String> {
        
        let client = self.get_client().await?;

        // 1. Fetch Context (A1) to get latest timestamp and DB-provided session names/biases
        let context = self.fetch_session_context(&asset_id, date).await?;
        
        // 2. Determine PS1_NAME and PS2_NAME based on TUI selection (Overriding A1 names)
        let (query_ps1_name, query_ps2_name) = if selected_chain.starts_with("TCS_") && selected_chain != "TCS_DEFAULT" {
            // Use the mapped names from the TUI selection
            Self::get_chain_context_names(&selected_chain)?
        } else {
            // Fallback (for TCS_DEFAULT or unexpected chain): Use the names found in the A1 context
            (context.ps1_name.clone().unwrap_or_else(|| "N/A".to_string()), 
             context.ps2_name.clone())
        };
        
        // 3. Determine Bias Values (PS1_BIAS and PS2_BIAS)
        // Use context bias if TUI input is "AUTO" (for TCS_DEFAULT)
        let final_ps1_bias = if ps1_bias_input == "AUTO" {
            context.ps1_bias_7.clone().unwrap_or_else(|| "Unknown".to_string())
        } else {
            ps1_bias_input
        };

        let final_ps2_bias = match ps2_bias_input {
            Some(bias) if bias == "AUTO" => context.ps2_bias_7.clone(),
            other => other,
        };

        // Ensure names are valid for query
        let query_ps1_name_ref = query_ps1_name.as_str();
        let query_ps2_name_ref = query_ps2_name.as_deref();


        let mut scores = Vec::new();

        // 4. Execute 1st Order Transition (Query B1)
        scores.extend(self.run_tcs_transition_query(
            &client,
            &asset_id,
            Q_B1_TCS_1ST_ORDER_TEMPLATE,
            1, // Order 1
            query_ps1_name_ref, // Use Chain/A1 derived PS1 name
            None, 
            &final_ps1_bias, // Use AUTO or User-defined PS1 bias
            None,
        ).await?);


        // 5. Execute 2nd Order Transition (Query B2) only if a PS2 bias was provided/auto-set.
        if let Some(ref ps2_bias) = final_ps2_bias {
            scores.extend(self.run_tcs_transition_query(
                &client,
                &asset_id,
                Q_B2_TCS_2ND_ORDER_TEMPLATE,
                2, // Order 2
                query_ps1_name_ref,
                query_ps2_name_ref, // Use Chain/A1 derived PS2 name
                &final_ps1_bias, 
                Some(ps2_bias), // Use AUTO or User-defined PS2 bias
            ).await?);
        }
        
        // 6. Handle No Scores Found
        if scores.is_empty() {
            return Err(anyhow!("No TCS scores found for asset '{}' with chain '{}'. PS1 Bias: {}, PS2 Bias: {}.",
                asset_id,
                selected_chain,
                final_ps1_bias,
                final_ps2_bias.as_deref().unwrap_or("N/A"),
            ));
        }

        // 7. Format Report (Ensure report reflects the *actual* biases used)
        let mut report = String::new();
        // --- Report Header ---
        report.push_str(&format!("TCS Predictive Analysis Report: {} ({})\n", asset_symbol, asset_id));
        report.push_str(&format!("Date of Analysis: {}\n", date));
        report.push_str(&format!("Chain Selected: {}\n", selected_chain)); 
        report.push_str(&format!("Latest Session Time: {}\n\n", context.found_session_end_ts.unwrap_or_else(|| "N/A".to_string())));
        
        // --- User-Defined/Auto-Used Bias Input ---
        report.push_str("== Biases Used for Prediction ==\n");
        report.push_str(&format!("* Primary Service 1 (PS1: {}) Bias: {}\n", query_ps1_name_ref, final_ps1_bias));
        
        if let Some(ps2_bias) = &final_ps2_bias {
            let ps2_name_used = query_ps2_name_ref.unwrap_or("N/A");
            report.push_str(&format!("* Primary Service 2 (PS2: {}) Bias: {}\n", ps2_name_used, ps2_bias));
        } else {
            report.push_str("* Primary Service 2 (PS2) Bias: N/A\n");
        }
        report.push_str("\n");
        
        // --- Session Context (A1) ---
        report.push_str("== Session Context (A1) ==\n");
        // This still prints the A1 context (latest market state), which is important historical data
        
        // Use .as_ref().map(|s| s.as_str()) to safely borrow the Option<String> contents
        let cs_name = context.cs_name.as_ref().map(|s| s.as_str()).unwrap_or("N/A (Missing Data)");
        let cs_bias_str = context.cs_bias_7.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
        report.push_str(&format!("* Composite Service (CS) / Current Session: {} (Bias: {})\n", cs_name, cs_bias_str));

        let ps1_name_context = context.ps1_name.as_ref().map(|s| s.as_str()).unwrap_or("N/A (Missing Data)");
        let ps1_bias_str = context.ps1_bias_7.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
        report.push_str(&format!("* Primary Service 1 (PS1) / Previous Session: {} (Bias: {})\n", ps1_name_context, ps1_bias_str));

        let ps2_name_context = context.ps2_name.as_ref().map(|s| s.as_str()).unwrap_or("N/A (Missing Data)");
        if context.ps2_name.is_some() { 
            let ps2_bias_str = context.ps2_bias_7.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
            report.push_str(&format!("* Primary Service 2 (PS2) / Second-Previous Session: {} (Bias: {})\n", ps2_name_context, ps2_bias_str));
        } else {
            report.push_str("* Primary Service 2 (PS2) / Second-Previous Session: N/A\n");
        }
        report.push_str("\n");
        
        // --- TCS Score Summary ---
        report.push_str("== Predicted TCS Transitions (B1 & B2) ==\n");

        // Helper function to generate a clean ASCII table for console display
        let generate_ascii_table = |scores: Vec<&TcsScore>, ps1: &str, ps2: Option<&str>| -> String {
            if scores.is_empty() {
                return "   No transition scores found.\n".to_string();
            }

            // Define column headers and minimum widths
            let headers = vec![
                ("PERIOD", 6), 
                ("TCS_SCORE", 12), 
                ("P(COND)", 8), 
                ("P(BASE)", 8), 
                ("PREDICTED_BIAS", 20), 
            ];

            let mut output = String::new();
            let mut widths: Vec<usize> = headers.iter().map(|h| h.1).collect();
            
            // Adjust PREDICTED_BIAS column width based on the longest bias name
            for score in scores.iter() {
                let bias_len = score.predicted_bias.len();
                widths[4] = widths[4].max(bias_len + 1); // +1 for padding
            }

            // Print Header
            let header_line = headers.iter().enumerate()
                .map(|(i, h)| format!("{:<width$}", h.0, width = widths[i]))
                .collect::<Vec<_>>()
                .join(" | ");
            output.push_str(&header_line);
            output.push('\n');
            output.push_str(&"=".repeat(header_line.len()));
            output.push('\n');
            
            // Print Data Rows
            for score in scores {
                let row_line = format!(
                    "{:<width1$} | {:<width2$} | {:<width3$} | {:<width4$} | {:<width5$}",
                    score.lookback_period,
                    format!("{:.4}", score.score),
                    format!("{:.2}", score.p_transition_conditional),
                    format!("{:.2}", score.p_cs_base),
                    score.predicted_bias,
                    width1 = widths[0],
                    width2 = widths[1],
                    width3 = widths[2],
                    width4 = widths[3],
                    width5 = widths[4],
                );
                output.push_str(&row_line);
                output.push('\n');
            }
            output
        };


        // --- 1st Order ---
        let b1_scores: Vec<&TcsScore> = scores.iter().filter(|s| s.order == 1).collect();
        report.push_str(&format!("--- 1st Order Transitions (PS1: {} -> CS) ---\n", query_ps1_name_ref));
        report.push_str(&generate_ascii_table(b1_scores, query_ps1_name_ref, None));
        report.push_str("\n");


        // --- 2nd Order ---
        let b2_scores: Vec<&TcsScore> = scores.iter().filter(|s| s.order == 2).collect();
        if !b2_scores.is_empty() {
            let ps2_name_str = query_ps2_name_ref.unwrap_or("N/A");
            report.push_str(&format!("--- 2nd Order Transitions (PS2: {} -> PS1: {} -> CS) ---\n", ps2_name_str, query_ps1_name_ref));
            report.push_str(&generate_ascii_table(b2_scores, query_ps1_name_ref, query_ps2_name_ref));
            report.push_str("\n");
        }
        
        // 7. Concluding Remarks
        report.push_str("--------------------------------------------------\n");
        report.push_str("The highest TCS_SCORE indicates the most likely transition.\n");

        Ok(report)
    }

    // --- E1 Implementation: Generic Table View (UNCHANGED) ---
    pub async fn run_db_query(&self, table_name: &str, asset_id: Option<String>, limit: u32) -> Result<()> {
        let client = self.get_client().await?;
        let where_clause = match asset_id {
            Some(id) => format!("WHERE asset_id = '{}'", id),
            None => "WHERE 1=1".to_string(),
        };
        
        let sql = format!(
            "SELECT * FROM {} {} ORDER BY 1 LIMIT {}",
            table_name,
            where_clause,
            limit
        );
        
        println!("\nExecuting SQL: {}", sql);

        let rows = client.query(&sql, &[]).await
            .map_err(|e| anyhow!("SQL Execution Error on table '{}': {}", table_name, e))?;

        if rows.is_empty() {
            println!("No results found in table '{}' with the given filters.", table_name);
            return Ok(());
        }
        self.print_query_results(&rows)?;
        Ok(())
    }
    
    fn print_query_results(&self, rows: &[Row]) -> Result<()> {
        let columns = rows[0].columns();
        let col_names: Vec<String> = columns.iter().map(|c| c.name().to_string()).collect();
        let mut column_widths: Vec<usize> = col_names.iter().map(|n| n.len()).collect();

        // Calculate maximum width for each column based on data and header
        for row in rows.iter() {
            for (i, col) in columns.iter().enumerate() {
                let value_str = match col.type_() {
                    &Type::NUMERIC | &Type::FLOAT8 => {
                        row.try_get::<&str, f64>(col.name()).map(|val| format!("{:.4}", val)).unwrap_or_else(|_| "NULL".to_string())
                    },
                    &Type::DATE => {
                        row.try_get::<&str, chrono::NaiveDate>(col.name()).map(|val| format!("{}", val)).unwrap_or_else(|_| "NULL".to_string())
                    },
                    &Type::BOOL => {
                        row.try_get::<&str, bool>(col.name()).map(|val| format!("{}", val)).unwrap_or_else(|_| "NULL".to_string())
                    },
                    _ => {
                        row.try_get::<&str, String>(col.name()).unwrap_or_else(|_| "NULL".to_string())
                    }
                };
                column_widths[i] = column_widths[i].max(value_str.len() + 2);
            }
        }
        
        // Print Header
        let header = col_names.iter().enumerate()
            .map(|(i, name)| format!("{:<width$}", name, width = column_widths[i]))
            .collect::<Vec<_>>()
            .join("| ");
        println!("\n{}", header);
        println!("{}", "-".repeat(header.len() + (col_names.len() - 1) * 2)); 

        // Print Data Rows
        for row in rows.iter() {
            let line = columns.iter().enumerate()
                .map(|(i, col)| {
                    let value_str = match col.type_() {
                        &Type::NUMERIC | &Type::FLOAT8 => row.try_get::<&str, f64>(col.name()).map(|v| format!("{:.4}", v)).unwrap_or_else(|_| "NULL".to_string()),
                        &Type::DATE => row.try_get::<&str, chrono::NaiveDate>(col.name()).map(|v| v.to_string()).unwrap_or_else(|_| "NULL".to_string()),
                        &Type::BOOL => row.try_get::<&str, bool>(col.name()).map(|v| v.to_string()).unwrap_or_else(|_| "NULL".to_string()),
                        &Type::TEXT | &Type::VARCHAR | &Type::BPCHAR | &Type::NAME => row.try_get::<&str, String>(col.name()).map(|v| v.to_string()).unwrap_or_else(|_| "NULL".to_string()),
                        _ => format!("{{{:?}}}", col.type_()),
                    };
                    format!("{:<width$}", value_str, width = column_widths[i])
                })
                .collect::<Vec<_>>()
                .join("| ");
            println!("{}", line);
        }
        
        println!("({} rows)", rows.len());

        Ok(())
    }
}