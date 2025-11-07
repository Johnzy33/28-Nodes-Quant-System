use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use prediction_engine::metrics_service;
use database_engine::runtime::setup_database_pool;
use chrono::NaiveDate;
use log::{info, error};

// --- CLI Structure ---

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Look up metrics based on a specific date or the latest available data.
    DateLookup {
        #[command(subcommand)]
        metric: DateMetric,
    },
    /// Look up metrics based on specific session patterns.
    PatternLookup {
        #[command(subcommand)]
        metric: PatternMetric,
    },
}

#[derive(Subcommand, Debug)]
enum DateMetric {
    /// Daily Composite Score (DCS) for a date, or the latest.
    Dcs {
        #[arg(long, default_value = "assets:US2000:FundedNext")]
        asset: String,
        #[arg(long)]
        date: Option<String>, // NaiveDate is parsed from string
    },
    /// Daily Bias Score (DBS) lookback factors for a date, or the latest.
    Dbs {
        #[arg(long, default_value = "assets:US2000:FundedNext")]
        asset: String,
        #[arg(long)]
        date: Option<String>, // NaiveDate is parsed from string
    },
    /// Predictive Confidence Score (PCS) for the latest completed pattern.
    PcsLatest {
        #[arg(long, default_value = "assets:US2000:FundedNext")]
        asset: String,
    },
}

#[derive(Subcommand, Debug)]
enum PatternMetric {
    /// Predictive Confidence Score (PCS) for a specific pattern key.
    Pcs {
        #[arg(long, default_value = "assets:US2000:FundedNext")]
        asset: String,
        #[arg(long)]
        ps_name: String, // Preceding Session Name (e.g., NYPM)
        #[arg(long)]
        cs_name: String, // Current Session Name (e.g., AS)
        #[arg(long)]
        outcome: String, // Daily Outcome (e.g., Bullish)
    },
    /// 1st Order Day Type Probability.
    FirstOrder {
        #[arg(long, default_value = "assets:US2000:FundedNext")]
        asset: String,
        #[arg(long)]
        ps_name: String,
        #[arg(long)]
        ps_bias: String, // 3-Bias (e.g., Bullish, Bearish, Consolidation)
        #[arg(long)]
        cs_name: String,
        #[arg(long)]
        cs_bias: String, // 3-Bias
        #[arg(long)]
        outcome: String, // 7-State Outcome (e.g., Consolidation-L, Bullish)
    },
    /// 2nd Order Day Type Probability.
    SecondOrder {
        #[arg(long, default_value = "assets:US2000:FundedNext")]
        asset: String,
        #[arg(long)]
        ps2_name: String,
        #[arg(long)]
        ps2_bias: String, // 7-Bias (e.g., Bullish, Consolidation-N, etc.)
        #[arg(long)]
        ps1_name: String,
        #[arg(long)]
        ps1_bias: String, // 7-Bias
        #[arg(long)]
        cs_name: String,
        #[arg(long)]
        cs_bias: String, // 7-Bias
        #[arg(long)]
        outcome: String, // 7-State Outcome
    },
}


// --- Main Execution ---

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    info!("Starting Prediction Engine CLI...");

    // 1. Setup Database Pool (Assumes DATABASE_URL is set in .env)
    let pool = setup_database_pool()
        .await
        .context("Failed to initialize the database connection pool")?;

    let cli = Cli::parse();
    
    // 2. Command Dispatch
    let result = match cli.command {
        Commands::DateLookup { metric } => handle_date_lookup(&pool, metric).await,
        Commands::PatternLookup { metric } => handle_pattern_lookup(&pool, metric).await,
    };
    
    if let Err(e) = result {
        error!("Operation failed: {:?}", e);
        // Return a non-zero exit code on failure
        std::process::exit(1);
    }

    Ok(())
}

// --- Handlers ---

async fn handle_date_lookup(pool: &database_engine::Pool, metric: DateMetric) -> Result<()> {
    match metric {
        DateMetric::Dcs { asset, date } => {
            let target_date = date.map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d")).transpose()
                .context("Invalid date format. Use YYYY-MM-DD.")?;

            let result = metrics_service::get_dcs_by_date(pool, &asset, target_date).await?;
            print_result(result, target_date.is_some())
        }
        DateMetric::Dbs { asset, date } => {
            let target_date = date.map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d")).transpose()
                .context("Invalid date format. Use YYYY-MM-DD.")?;

            let result = metrics_service::get_dbs_by_date(pool, &asset, target_date).await?;
            print_result(result, target_date.is_some())
        }
        DateMetric::PcsLatest { asset } => {
            let results = metrics_service::get_latest_pcs_scores(pool, &asset).await?;
            if results.is_empty() {
                println!("No latest PCS scores found for asset {}.", asset);
            } else {
                println!("\n✅ Latest PCS Scores (Inferred Pattern)");
                println!("--------------------------------------");
                for res in results {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                }
            }
            Ok(())
        }
    }
}

async fn handle_pattern_lookup(pool: &database_engine::Pool, metric: PatternMetric) -> Result<()> {
    match metric {
        PatternMetric::Pcs { asset, ps_name, cs_name, outcome } => {
            let results = metrics_service::get_pcs_by_pattern(pool, &asset, &ps_name, &cs_name, &outcome).await?;
            if results.is_empty() {
                println!("No PCS scores found for pattern: {} -> {} (Outcome: {})", ps_name, cs_name, outcome);
            } else {
                println!("\n✅ PCS Scores for Pattern: {} -> {} (Outcome: {})", ps_name, cs_name, outcome);
                println!("--------------------------------------");
                for res in results {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                }
            }
            Ok(())
        }
        PatternMetric::FirstOrder { asset, ps_name, ps_bias, cs_name, cs_bias, outcome } => {
            let result = metrics_service::get_1st_order_outcome(
                pool, &asset, &ps_name, &ps_bias, &cs_name, &cs_bias, &outcome
            ).await?;
            print_result(result, true)
        }
        PatternMetric::SecondOrder { asset, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, outcome } => {
            let result = metrics_service::get_2nd_order_outcome(
                pool, &asset, &ps2_name, &ps2_bias, &ps1_name, &ps1_bias, &cs_name, &cs_bias, &outcome
            ).await?;
            print_result(result, true)
        }
    }
}

/// Helper function to print a single result
fn print_result<T: serde::Serialize>(result: Option<T>, is_specific_date: bool) -> Result<()> {
    if let Some(res) = result {
        let title = if is_specific_date { "Specific Lookup Result" } else { "Latest Available Result" };
        println!("\n✅ {}", title);
        println!("--------------------------------------");
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!("No data found for the requested parameters.");
    }
    Ok(())
}