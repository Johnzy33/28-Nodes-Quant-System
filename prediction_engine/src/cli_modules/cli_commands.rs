use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, CommandFactory};
use chrono::NaiveDate;
use crate::tui_modules::tui_main::run_tui;

use super::core_logic::DbExecutor;

// Helper struct for parsing CLI arguments in the execute_command function
#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Defines the subcommands available in non-interactive CLI mode.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Execute a full TCS analysis report for a specific asset and context.
    Run {
        asset_id: String,
        /// The date for analysis (YYYY-MM-DD).
        #[arg(long)]
        date: String, 
        /// The predictive chain/context to use (e.g., TCS_NYL_NYPM_AS).
        #[arg(long)]
        chain_path: String, // <-- NEW: Holds the chain string
        #[arg(long)]
        ps1_bias: String,
        #[arg(long)]
        ps2_bias: Option<String>,
        #[arg(long, default_value = "markdown")]
        output: String,
    },
    /// Directly query the database for raw TCS data or context.
    Query {
        #[arg(long)]
        table: String,
        #[arg(long)]
        asset: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// Display system configuration and table health.
    Config, 
}

/// Executes the chosen CLI command when running in direct mode.
pub async fn execute_command(command: Option<Commands>) -> Result<()> {
    // 1. Initialize the DB connection pool once
    let executor = DbExecutor::new().await?;

    if let Some(cmd) = command {
        match cmd {
            Commands::Run { 
                asset_id, 
                date, // Date string (YYYY-MM-DD)
                chain_path, // Chain path string (e.g., TCS_NYL_NYPM_AS)
                ps1_bias, 
                ps2_bias, 
                output 
            } => {
                // Parse the date string argument
                let analysis_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                    .map_err(|e| anyhow!("Invalid date format for 'date'. Please use YYYY-MM-DD: {}", e))?;
                    
                let assets = executor.fetch_asset_list().await?;
                let asset_symbol = assets.iter()
                    .find(|a| a.id == asset_id)
                    .map(|a| a.symbol.clone())
                    .unwrap_or_else(|| "UNKNOWN_ASSET".to_string());
                
                // --- FIX: Pass the correct 'chain_path' variable and 'analysis_date' ---
                match executor.generate_tcs_report(asset_symbol, asset_id, analysis_date, ps1_bias, ps2_bias, chain_path).await {
                    Ok(report) => {
                        println!("\n--- TCS Report Output ({}) ---\n", output.to_uppercase());
                        println!("{}", report);
                        println!("\n------------------------------");
                    },
                    Err(e) => {
                        eprintln!("\n[ERROR] Failed to generate TCS report: {}", e);
                    }
                }
            }
            
            Commands::Query { table, asset, limit } => {
                println!("[INFO] Executing QUERY command: table={}, asset={:?}, limit={}", table, asset, limit);
                
                executor.run_db_query(&table, asset, limit).await
                    .map_err(|e| anyhow!("Query command failed: {}", e))?;
            }

            Commands::Config => {
                println!("[INFO] Displaying Configuration...");
                println!("Database Executor is initialized and connected.");
            }
        }
    } else {
        // Fallback to TUI mode
        run_tui(executor).await?;
    }
    Ok(())
}