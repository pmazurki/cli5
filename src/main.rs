//! CLI5 - Modern Cloudflare CLI in Rust
//!
//! Supports both REST API and GraphQL API with dynamic endpoint configuration.

mod api;
mod cli;
mod config;
mod output;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::cli::{Cli, Commands};
use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Load configuration
    let config = Config::load()?;

    // Execute command
    match cli.command {
        Commands::Zones(args) => cli::zones::execute(&config, args).await,
        Commands::Dns(args) => cli::dns::execute(&config, args).await,
        Commands::Settings(args) => cli::settings::execute(&config, args).await,
        Commands::Firewall(args) => cli::firewall::execute(&config, args).await,
        Commands::Cache(args) => cli::cache::execute(&config, args).await,
        Commands::Ssl(args) => cli::ssl::execute(&config, args).await,
        Commands::Analytics(args) => cli::analytics::execute(&config, args).await,
        Commands::Workers(args) => cli::workers::execute(&config, args).await,
        Commands::Pages(args) => cli::pages::execute(&config, args).await,
        Commands::Ai(args) => cli::ai::execute(&config, args).await,
        Commands::Raw(args) => cli::raw::execute(&config, args).await,
        Commands::Config(args) => cli::config_cmd::execute(&config, args).await,
    }
}
