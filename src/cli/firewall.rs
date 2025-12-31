//! Firewall command

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct FirewallArgs {
    /// Zone name or ID
    #[arg(short, long)]
    pub zone: Option<String>,

    #[command(subcommand)]
    pub command: FirewallCommand,
}

#[derive(Subcommand, Debug)]
pub enum FirewallCommand {
    /// List access rules
    List,

    /// Block an IP address
    BlockIp {
        /// IP address to block
        ip: String,

        /// Note/reason
        #[arg(short, long)]
        note: Option<String>,
    },

    /// Block a country
    BlockCountry {
        /// Country code (e.g., RU, CN)
        code: String,

        /// Note/reason
        #[arg(short, long)]
        note: Option<String>,
    },

    /// Whitelist an IP address
    WhitelistIp {
        /// IP address to whitelist
        ip: String,

        /// Note/reason
        #[arg(short, long)]
        note: Option<String>,
    },

    /// Challenge an IP (CAPTCHA)
    ChallengeIp {
        /// IP address to challenge
        ip: String,

        /// Note/reason
        #[arg(short, long)]
        note: Option<String>,
    },

    /// Delete an access rule
    Delete {
        /// Rule ID
        id: String,
    },

    /// List firewall rules
    Rules,

    /// List WAF packages (Pro+)
    Waf,
}

pub async fn execute(config: &Config, args: FirewallArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;
    let zone = config.resolve_zone(args.zone.as_deref())?;
    let zone_id = client.resolve_zone_id(&zone).await?;

    match args.command {
        FirewallCommand::List => {
            let response = client
                .get_raw(&format!("/zones/{}/firewall/access_rules/rules", zone_id))
                .await?;

            if let Some(rules) = response.get("result").and_then(|r| r.as_array()) {
                output::table_header(&["MODE", "TARGET", "VALUE", "NOTES", "ID"]);
                for rule in rules {
                    output::print_firewall_rule(rule);
                }
                output::info(&format!("Total: {} rules", rules.len()));
            }
        }

        FirewallCommand::BlockIp { ip, note } => {
            let body = json!({
                "mode": "block",
                "configuration": {
                    "target": "ip",
                    "value": ip
                },
                "notes": note.unwrap_or_default()
            });

            let response = client
                .post_raw(
                    &format!("/zones/{}/firewall/access_rules/rules", zone_id),
                    body,
                )
                .await?;
            output::success(&format!("Blocked IP: {}", ip));

            if let Some(result) = response.get("result") {
                output::print_firewall_rule(result);
            }
        }

        FirewallCommand::BlockCountry { code, note } => {
            let body = json!({
                "mode": "block",
                "configuration": {
                    "target": "country",
                    "value": code.to_uppercase()
                },
                "notes": note.unwrap_or_default()
            });

            let response = client
                .post_raw(
                    &format!("/zones/{}/firewall/access_rules/rules", zone_id),
                    body,
                )
                .await?;
            output::success(&format!("Blocked country: {}", code.to_uppercase()));

            if let Some(result) = response.get("result") {
                output::print_firewall_rule(result);
            }
        }

        FirewallCommand::WhitelistIp { ip, note } => {
            let body = json!({
                "mode": "whitelist",
                "configuration": {
                    "target": "ip",
                    "value": ip
                },
                "notes": note.unwrap_or_default()
            });

            let response = client
                .post_raw(
                    &format!("/zones/{}/firewall/access_rules/rules", zone_id),
                    body,
                )
                .await?;
            output::success(&format!("Whitelisted IP: {}", ip));

            if let Some(result) = response.get("result") {
                output::print_firewall_rule(result);
            }
        }

        FirewallCommand::ChallengeIp { ip, note } => {
            let body = json!({
                "mode": "challenge",
                "configuration": {
                    "target": "ip",
                    "value": ip
                },
                "notes": note.unwrap_or_default()
            });

            let response = client
                .post_raw(
                    &format!("/zones/{}/firewall/access_rules/rules", zone_id),
                    body,
                )
                .await?;
            output::success(&format!("Challenge enabled for IP: {}", ip));

            if let Some(result) = response.get("result") {
                output::print_firewall_rule(result);
            }
        }

        FirewallCommand::Delete { id } => {
            client
                .delete_raw(&format!(
                    "/zones/{}/firewall/access_rules/rules/{}",
                    zone_id, id
                ))
                .await?;
            output::success(&format!("Deleted firewall rule: {}", id));
        }

        FirewallCommand::Rules => {
            let response = client
                .get_raw(&format!("/zones/{}/firewall/rules", zone_id))
                .await?;
            output::print_output(&response.get("result"), &config.output_format)?;
        }

        FirewallCommand::Waf => {
            let response = client
                .get_raw(&format!("/zones/{}/firewall/waf/packages", zone_id))
                .await?;
            output::print_output(&response.get("result"), &config.output_format)?;
        }
    }

    Ok(())
}
