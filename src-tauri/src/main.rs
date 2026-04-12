mod cli;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tauri_app_lib::mcp::MediumMcpServer;

#[derive(Parser)]
#[command(name = "medium")]
#[command(about = "Medium: The spirit vessel for agentic manifestations", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Medium daemon with Tauri UI
    #[command(alias = "serve")]
    Daemon {
        #[arg(short, long, default_value = "default")]
        instance: String,
        #[arg(short, long, default_value = "vita")]
        ghost: String,
    },
    /// Launch an MCP bridge to inhabit a Shell
    Mcp {
        #[arg(short, long, default_value = "vita")]
        ghost: String,
    },
    /// Initialize Medium and configure agent CLIs
    Init,
    /// View the latest logs from the background daemon
    Logs {
        #[arg(short, long, default_value = "100")]
        lines: usize,
        #[arg(long)]
        filter: Option<String>,
        #[arg(short = 'f', long)]
        follow: bool,
    },
    /// Check if the daemon is currently running
    Status,
    /// Run diagnostic checks on the Medium setup
    Doctor,
    /// Manage ghosts (scaffold, validate, preview, etc.)
    #[command(subcommand)]
    Ghosts(GhostsCommands),
}

#[derive(Subcommand)]
enum GhostsCommands {
    /// Create a new ghost scaffold
    Scaffold {
        /// Name of the ghost
        name: String,
        /// Path where the ghost will be created (defaults to ~/.medium/ghosts)
        path: Option<String>,
    },
    /// Validate a ghost manifest and assets
    Validate {
        /// Path to the ghost folder containing ghost.toml
        path: String,
    },
    /// Preview a ghost locally
    Preview {
        /// Path to the ghost folder containing ghost.toml
        path: String,
    },
    /// Import a ghost from external art
    Import {
        #[command(subcommand)]
        importer: cli::import::ImportCommand,
    },
    /// List available ghosts (built-in and custom)
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let app_cli = Cli::parse();

    match app_cli.command {
        Commands::Daemon { instance, ghost } => {
            tauri_app_lib::run(ghost, instance);
            Ok(())
        }
        Commands::Mcp { ghost } => {
            // When launched by an agent, the daemon should already be running.
            // This logic is now primarily for manual testing.
            cli::daemon::ensure_running(true)?;
            let server = MediumMcpServer::new(ghost);
            server.run().await?;
            Ok(())
        }
        Commands::Init => {
            println!("Initializing Medium...");
            cli::init::run()?;
            Ok(())
        }
        Commands::Logs {
            lines,
            filter,
            follow,
        } => cli::logs::run(lines, filter.as_deref(), follow),
        Commands::Status => cli::status::run().await,
        Commands::Doctor => cli::doctor::run().await,
        Commands::Ghosts(cmd) => match cmd {
            GhostsCommands::Scaffold { name, path } => cli::scaffold::run(&name, path.as_deref()),
            GhostsCommands::Validate { path } => cli::validate::run(&path).await,
            GhostsCommands::Preview { path } => {
                cli::preview::run(&path)
            }
            GhostsCommands::Import { importer } => cli::import::run(importer),
            GhostsCommands::List => cli::list::run(),
        },
    }
}
