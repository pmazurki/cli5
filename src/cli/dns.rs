//! DNS command

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct DnsArgs {
    /// Zone name or ID
    #[arg(short, long)]
    pub zone: Option<String>,

    #[command(subcommand)]
    pub command: DnsCommand,
}

#[derive(Subcommand, Debug)]
pub enum DnsCommand {
    /// List all DNS records
    List {
        /// Filter by record type (A, AAAA, CNAME, TXT, MX, etc.)
        #[arg(short = 't', long)]
        record_type: Option<String>,

        /// Filter by name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Get a specific DNS record
    Get {
        /// Record ID
        id: String,
    },

    /// Add a new DNS record
    Add {
        /// Record name (e.g., "www" or "api.example.com")
        name: String,

        /// Record type (A, AAAA, CNAME, TXT, MX, etc.)
        #[arg(short = 't', long, default_value = "A")]
        record_type: String,

        /// Record content (IP address, hostname, etc.)
        content: String,

        /// Enable Cloudflare proxy
        #[arg(short, long, default_value = "true")]
        proxied: bool,

        /// TTL in seconds (1 = auto)
        #[arg(long, default_value = "1")]
        ttl: u32,

        /// Priority (for MX/SRV records)
        #[arg(long)]
        priority: Option<u16>,
    },

    /// Update a DNS record
    Update {
        /// Record ID
        id: String,

        /// Record name
        #[arg(short, long)]
        name: Option<String>,

        /// Record content
        #[arg(short, long)]
        content: Option<String>,

        /// Enable/disable proxy
        #[arg(short, long)]
        proxied: Option<bool>,

        /// TTL in seconds
        #[arg(long)]
        ttl: Option<u32>,
    },

    /// Delete a DNS record
    Delete {
        /// Record ID
        id: String,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Export all DNS records as JSON
    Export,
}

pub async fn execute(config: &Config, args: DnsArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;
    let zone = config.resolve_zone(args.zone.as_deref())?;
    let zone_id = client.resolve_zone_id(&zone).await?;

    match args.command {
        DnsCommand::List { record_type, name } => {
            let mut path = format!("/zones/{}/dns_records", zone_id);
            let mut params = vec![];

            if let Some(ref t) = record_type {
                params.push(format!("type={}", t));
            }
            if let Some(ref n) = name {
                params.push(format!("name={}", n));
            }

            if !params.is_empty() {
                path = format!("{}?{}", path, params.join("&"));
            }

            let response = client.get_raw(&path).await?;

            if let Some(records) = response.get("result").and_then(|r| r.as_array()) {
                output::table_header(&["TYPE", "NAME", "CONTENT", "PROXY", "TTL", "ID"]);
                for record in records {
                    output::print_dns_record(record);
                }
                output::info(&format!("Total: {} records", records.len()));
            }
        }

        DnsCommand::Get { id } => {
            let response = client
                .get_raw(&format!("/zones/{}/dns_records/{}", zone_id, id))
                .await?;

            if let Some(result) = response.get("result") {
                output::print_output(result, &config.output_format)?;
            }
        }

        DnsCommand::Add {
            name,
            record_type,
            content,
            proxied,
            ttl,
            priority,
        } => {
            let mut body = json!({
                "type": record_type.to_uppercase(),
                "name": name,
                "content": content,
                "proxied": proxied,
                "ttl": ttl
            });

            if let Some(p) = priority {
                body["priority"] = json!(p);
            }

            let response = client
                .post_raw(&format!("/zones/{}/dns_records", zone_id), body)
                .await?;

            if let Some(result) = response.get("result") {
                output::success(&format!(
                    "Created {} record: {}",
                    record_type.to_uppercase(),
                    result.get("name").and_then(|v| v.as_str()).unwrap_or(&name)
                ));
                output::print_dns_record(result);
            }
        }

        DnsCommand::Update {
            id,
            name,
            content,
            proxied,
            ttl,
        } => {
            // First get current record
            let current = client
                .get_raw(&format!("/zones/{}/dns_records/{}", zone_id, id))
                .await?;
            let current = current
                .get("result")
                .ok_or_else(|| anyhow::anyhow!("Record not found"))?;

            let body = json!({
                "type": current.get("type").and_then(|v| v.as_str()).unwrap_or("A"),
                "name": name.as_deref().or_else(|| current.get("name").and_then(|v| v.as_str())).unwrap_or(""),
                "content": content.as_deref().or_else(|| current.get("content").and_then(|v| v.as_str())).unwrap_or(""),
                "proxied": proxied.or_else(|| current.get("proxied").and_then(|v| v.as_bool())).unwrap_or(false),
                "ttl": ttl.or_else(|| current.get("ttl").and_then(|v| v.as_u64()).map(|v| v as u32)).unwrap_or(1)
            });

            let response = client
                .put_raw(&format!("/zones/{}/dns_records/{}", zone_id, id), body)
                .await?;

            if let Some(result) = response.get("result") {
                output::success("Updated DNS record:");
                output::print_dns_record(result);
            }
        }

        DnsCommand::Delete { id, yes } => {
            if !yes {
                output::warning(&format!("Are you sure you want to delete record {}?", id));
                output::info("Use -y to skip this confirmation");
                return Ok(());
            }

            client
                .delete_raw(&format!("/zones/{}/dns_records/{}", zone_id, id))
                .await?;
            output::success(&format!("Deleted DNS record: {}", id));
        }

        DnsCommand::Export => {
            let response = client
                .get_raw(&format!("/zones/{}/dns_records", zone_id))
                .await?;

            if let Some(result) = response.get("result") {
                println!("{}", serde_json::to_string_pretty(result)?);
            }
        }
    }

    Ok(())
}

