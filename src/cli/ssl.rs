//! SSL/TLS command

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct SslArgs {
    /// Zone name or ID
    #[arg(short, long)]
    pub zone: Option<String>,

    #[command(subcommand)]
    pub command: SslCommand,
}

#[derive(Subcommand, Debug)]
pub enum SslCommand {
    /// Show current SSL/TLS settings
    Status,

    /// Set SSL mode (off, flexible, full, strict)
    Mode {
        /// SSL mode: off, flexible, full, strict
        mode: String,
    },

    /// Set minimum TLS version
    MinTls {
        /// TLS version: 1.0, 1.1, 1.2, 1.3
        version: String,
    },

    /// Enable/disable TLS 1.3
    Tls13 {
        /// on or off
        state: String,
    },

    /// Enable/disable Always Use HTTPS
    AlwaysHttps {
        /// on or off
        state: String,
    },

    /// Show certificate details
    Certs,

    /// Enable/disable Automatic HTTPS Rewrites
    AutoHttps {
        /// on or off
        state: String,
    },
}

pub async fn execute(config: &Config, args: SslArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;
    let zone = config.resolve_zone(args.zone.as_deref())?;
    let zone_id = client.resolve_zone_id(&zone).await?;

    match args.command {
        SslCommand::Status => {
            output::info("SSL/TLS Settings:");
            println!();

            // Get all SSL settings
            let ssl = get_setting(&client, &zone_id, "ssl").await?;
            let min_tls = get_setting(&client, &zone_id, "min_tls_version").await?;
            let tls13 = get_setting(&client, &zone_id, "tls_1_3").await?;
            let always_https = get_setting(&client, &zone_id, "always_use_https").await?;
            let auto_https = get_setting(&client, &zone_id, "automatic_https_rewrites").await?;

            println!("SSL Mode:          {}", format_ssl_mode(&ssl));
            println!("Min TLS Version:   {}", format_tls_version(&min_tls));
            println!("TLS 1.3:           {}", format_on_off(&tls13));
            println!("Always HTTPS:      {}", format_on_off(&always_https));
            println!("Auto HTTPS Rewrite:{}", format_on_off(&auto_https));

            // Security recommendations
            println!();
            if min_tls == "1.0" || min_tls == "1.1" {
                output::warning(&format!(
                    "⚠️  Min TLS {} is insecure! Recommend: cli5 ssl min-tls 1.2",
                    min_tls
                ));
            }
            if ssl == "off" || ssl == "flexible" {
                output::warning(&format!(
                    "⚠️  SSL mode '{}' is not recommended! Use 'full' or 'strict'",
                    ssl
                ));
            }
        }

        SslCommand::Mode { mode } => {
            let valid = ["off", "flexible", "full", "strict"];
            if !valid.contains(&mode.as_str()) {
                return Err(anyhow::anyhow!(
                    "Invalid SSL mode. Use: off, flexible, full, strict"
                ));
            }

            set_setting(&client, &zone_id, "ssl", &mode).await?;
            output::success(&format!("SSL mode set to: {}", mode));
        }

        SslCommand::MinTls { version } => {
            let valid = ["1.0", "1.1", "1.2", "1.3"];
            if !valid.contains(&version.as_str()) {
                return Err(anyhow::anyhow!("Invalid TLS version. Use: 1.0, 1.1, 1.2, 1.3"));
            }

            set_setting(&client, &zone_id, "min_tls_version", &version).await?;
            output::success(&format!("Minimum TLS version set to: {}", version));
        }

        SslCommand::Tls13 { state } => {
            let state = normalize_on_off(&state)?;
            set_setting(&client, &zone_id, "tls_1_3", &state).await?;
            output::success(&format!("TLS 1.3: {}", state));
        }

        SslCommand::AlwaysHttps { state } => {
            let state = normalize_on_off(&state)?;
            set_setting(&client, &zone_id, "always_use_https", &state).await?;
            output::success(&format!("Always Use HTTPS: {}", state));
        }

        SslCommand::Certs => {
            let path = format!("/zones/{}/ssl/certificate_packs", zone_id);
            let response = client.get_raw(&path).await?;

            if let Some(certs) = response.get("result").and_then(|r| r.as_array()) {
                output::table_header(&["STATUS", "HOSTS", "ISSUER", "EXPIRES"]);

                for pack in certs {
                    if let Some(certificates) = pack.get("certificates").and_then(|c| c.as_array())
                    {
                        for cert in certificates {
                            let status = cert
                                .get("status")
                                .and_then(|s| s.as_str())
                                .unwrap_or("-");
                            let hosts = cert
                                .get("hosts")
                                .and_then(|h| h.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                })
                                .unwrap_or_else(|| "-".to_string());
                            let issuer = cert
                                .get("issuer")
                                .and_then(|i| i.as_str())
                                .unwrap_or("-");
                            let expires = cert
                                .get("expires_on")
                                .and_then(|e| e.as_str())
                                .map(|s| s.split('T').next().unwrap_or(s))
                                .unwrap_or("-");

                            let status_colored = if status == "active" {
                                format!("\x1b[32m{}\x1b[0m", status)
                            } else {
                                status.to_string()
                            };

                            println!("{}\t{}\t{}\t{}", status_colored, hosts, issuer, expires);
                        }
                    }
                }
            } else {
                output::warning("No certificates found");
            }
        }

        SslCommand::AutoHttps { state } => {
            let state = normalize_on_off(&state)?;
            set_setting(&client, &zone_id, "automatic_https_rewrites", &state).await?;
            output::success(&format!("Automatic HTTPS Rewrites: {}", state));
        }
    }

    Ok(())
}

async fn get_setting(client: &CloudflareClient, zone_id: &str, setting: &str) -> Result<String> {
    let path = format!("/zones/{}/settings/{}", zone_id, setting);
    let response = client.get_raw(&path).await?;

    Ok(response
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "-".to_string()))
}

async fn set_setting(
    client: &CloudflareClient,
    zone_id: &str,
    setting: &str,
    value: &str,
) -> Result<()> {
    let path = format!("/zones/{}/settings/{}", zone_id, setting);
    let body = json!({ "value": value });
    client.patch_raw(&path, body).await?;
    Ok(())
}

fn normalize_on_off(state: &str) -> Result<String> {
    match state.to_lowercase().as_str() {
        "on" | "true" | "1" | "yes" | "enable" => Ok("on".to_string()),
        "off" | "false" | "0" | "no" | "disable" => Ok("off".to_string()),
        _ => Err(anyhow::anyhow!("Invalid state. Use: on/off")),
    }
}

fn format_ssl_mode(mode: &str) -> String {
    match mode {
        "off" => "\x1b[31moff\x1b[0m (insecure!)".to_string(),
        "flexible" => "\x1b[33mflexible\x1b[0m (partial)".to_string(),
        "full" => "\x1b[32mfull\x1b[0m".to_string(),
        "strict" => "\x1b[32mstrict\x1b[0m (recommended)".to_string(),
        _ => mode.to_string(),
    }
}

fn format_tls_version(version: &str) -> String {
    match version {
        "1.0" => "\x1b[31m1.0\x1b[0m (insecure!)".to_string(),
        "1.1" => "\x1b[33m1.1\x1b[0m (deprecated)".to_string(),
        "1.2" => "\x1b[32m1.2\x1b[0m".to_string(),
        "1.3" => "\x1b[32m1.3\x1b[0m (best)".to_string(),
        _ => version.to_string(),
    }
}

fn format_on_off(state: &str) -> String {
    match state {
        "on" => "\x1b[32mon\x1b[0m".to_string(),
        "off" => "\x1b[31moff\x1b[0m".to_string(),
        _ => state.to_string(),
    }
}

