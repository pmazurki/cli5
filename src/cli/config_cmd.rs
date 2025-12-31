//! Config command

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::api::endpoints;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Show current configuration
    Show,

    /// Test API connection
    Test,

    /// List available endpoints from JSON files
    Endpoints {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },

    /// Show config paths
    Paths,
}

pub async fn execute(config: &Config, args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommand::Show => {
            output::info("Current configuration:");

            if config.api_token.is_some() {
                println!("  Auth: API Token (set)");
            } else if config.api_key.is_some() && config.api_email.is_some() {
                println!("  Auth: Global API Key (set)");
            } else {
                output::warning("  Auth: Not configured!");
            }

            if let Some(ref zone_id) = config.zone_id {
                println!("  Default Zone ID: {}", zone_id);
            }

            if let Some(ref zone_name) = config.zone_name {
                println!("  Default Zone Name: {}", zone_name);
            }

            println!("  Output Format: {:?}", config.output_format);
        }

        ConfigCommand::Test => {
            use crate::api::CloudflareClient;

            output::info("Testing API connection...");

            let client = CloudflareClient::new(config.clone())?;

            match client.get_raw("/user/tokens/verify").await {
                Ok(response) => {
                    if let Some(result) = response.get("result") {
                        let status = result
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        if status == "active" {
                            output::success("API token is valid and active!");
                        } else {
                            output::warning(&format!("Token status: {}", status));
                        }
                    }
                }
                Err(e) => {
                    output::error(&format!("API connection failed: {}", e));
                }
            }
        }

        ConfigCommand::Endpoints { category } => {
            let registry = endpoints::load_registry()?;

            if let Some(cat) = category {
                output::info(&format!("Endpoints in category '{}':", cat));
                for endpoint in registry.list_by_category(&cat) {
                    println!("  {} - {}", endpoint.name, endpoint.description);
                }
            } else {
                output::info("Available endpoint categories:");
                for cat in registry.categories() {
                    println!("  {}", cat);
                }

                output::info(&format!("\nTotal endpoints: {}", registry.endpoints.len()));
                output::info("Use --category <name> to list endpoints in a category");
            }
        }

        ConfigCommand::Paths => {
            output::info("Configuration paths:");

            if let Ok(config_dir) = Config::config_dir() {
                println!("  Config directory: {}", config_dir.display());
            }

            if let Ok(endpoints_dir) = Config::endpoints_dir() {
                println!("  Endpoints directory: {}", endpoints_dir.display());
            }

            println!("  Environment file: .env (current directory)");
        }
    }

    Ok(())
}
