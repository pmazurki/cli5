//! Zones command

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct ZonesArgs {
    #[command(subcommand)]
    pub command: Option<ZonesCommand>,
}

#[derive(Subcommand, Debug)]
pub enum ZonesCommand {
    /// List all zones
    List,

    /// Get zone details
    Get {
        /// Zone name or ID
        zone: String,
    },

    /// Get zone ID by name
    Id {
        /// Zone name
        name: String,
    },
}

pub async fn execute(config: &Config, args: ZonesArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;

    match args.command.unwrap_or(ZonesCommand::List) {
        ZonesCommand::List => {
            let response = client.get_raw("/zones").await?;

            if let Some(zones) = response.get("result").and_then(|r| r.as_array()) {
                output::table_header(&["NAME", "STATUS", "PLAN", "ID"]);
                for zone in zones {
                    output::print_zone(zone);
                }
                output::info(&format!("Total: {} zones", zones.len()));
            }
        }

        ZonesCommand::Get { zone } => {
            let zone_id = client.resolve_zone_id(&zone).await?;
            let response = client.get_raw(&format!("/zones/{}", zone_id)).await?;

            if let Some(result) = response.get("result") {
                output::print_output(result, &config.output_format)?;
            }
        }

        ZonesCommand::Id { name } => {
            let zone_id = client.get_zone_id(&name).await?;
            println!("{}", zone_id);
        }
    }

    Ok(())
}
