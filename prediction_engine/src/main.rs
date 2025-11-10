use anyhow::Result;
use clap::Parser;

mod cli_modules;
mod tui_modules;
use cli_modules::Commands;
use cli_modules::cli_commands;


#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Parse the command-line arguments
    let cli = Cli::parse();

    // 2. Execute the corresponding command logic
    if let Err(e) = cli_commands::execute_command(cli.command).await {
        eprintln!("[FATAL ERROR] Command execution failed: {}", e);
        // Return error code 1 for failure
        std::process::exit(1);
    }

    Ok(())
}