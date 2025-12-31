//! Analytics command (GraphQL)

use anyhow::Result;
use chrono::{Duration, Utc};
use clap::{Args, Subcommand};

use crate::api::graphql;
use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct AnalyticsArgs {
    /// Zone name or ID
    #[arg(short, long)]
    pub zone: Option<String>,

    /// Time range: 1h, 6h, 24h, 7d, 30d
    #[arg(short, long, default_value = "24h")]
    pub since: String,

    /// Number of results
    #[arg(short, long, default_value = "20")]
    pub limit: u32,

    #[command(subcommand)]
    pub command: AnalyticsCommand,
}

#[derive(Subcommand, Debug)]
pub enum AnalyticsCommand {
    /// Top requested URLs
    TopUrls,

    /// Top visitor IPs
    TopIps,

    /// Top countries
    TopCountries,

    /// Error responses (4xx, 5xx)
    Errors,

    /// Cache hit/miss statistics
    Cache,

    /// Bandwidth by status code
    Bandwidth,

    /// Device types and user agents
    Bots,

    /// Firewall events (Pro+ only)
    Firewall,

    /// Hourly traffic summary
    Hourly,

    /// Run a custom GraphQL query
    Query {
        /// GraphQL query string
        query: String,
    },
}

fn parse_since(since: &str) -> String {
    let duration = match since {
        "1h" => Duration::hours(1),
        "6h" => Duration::hours(6),
        "24h" => Duration::hours(24),
        "7d" => Duration::days(7),
        "30d" => Duration::days(30),
        _ => Duration::hours(24),
    };

    (Utc::now() - duration)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

pub async fn execute(config: &Config, args: AnalyticsArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;
    let zone = config.resolve_zone(args.zone.as_deref())?;
    let zone_id = client.resolve_zone_id(&zone).await?;
    let since = parse_since(&args.since);

    match args.command {
        AnalyticsCommand::TopUrls => {
            let query = graphql::top_urls_query(&zone_id, &since, args.limit);
            let response = client.graphql(&query, None).await?;
            print_analytics_response(&response, "clientRequestPath")?;
        }

        AnalyticsCommand::TopIps => {
            let query = graphql::top_ips_query(&zone_id, &since, args.limit);
            let response = client.graphql(&query, None).await?;
            print_analytics_response(&response, "clientIP")?;
        }

        AnalyticsCommand::TopCountries => {
            let query = graphql::top_countries_query(&zone_id, &since, args.limit);
            let response = client.graphql(&query, None).await?;
            print_analytics_response(&response, "clientCountryName")?;
        }

        AnalyticsCommand::Errors => {
            let query = graphql::errors_query(&zone_id, &since, args.limit);
            let response = client.graphql(&query, None).await?;
            print_analytics_response(&response, "edgeResponseStatus")?;
        }

        AnalyticsCommand::Cache => {
            let query = graphql::cache_status_query(&zone_id, &since, args.limit);
            let response = client.graphql(&query, None).await?;
            print_analytics_response(&response, "cacheStatus")?;
        }

        AnalyticsCommand::Bandwidth => {
            let query = graphql::bandwidth_query(&zone_id, &since, args.limit);
            let response = client.graphql(&query, None).await?;
            print_bandwidth_response(&response)?;
        }

        AnalyticsCommand::Bots => {
            let query = graphql::bots_query(&zone_id, &since, args.limit);
            let response = client.graphql(&query, None).await?;
            print_analytics_response(&response, "clientDeviceType")?;
        }

        AnalyticsCommand::Firewall => {
            let query = graphql::firewall_events_query(&zone_id, &since, args.limit);
            let response = client.graphql(&query, None).await?;
            print_firewall_response(&response)?;
        }

        AnalyticsCommand::Hourly => {
            let query = graphql::hourly_traffic_query(&zone_id, &since);
            let response = client.graphql(&query, None).await?;
            print_hourly_response(&response)?;
        }

        AnalyticsCommand::Query { query } => {
            let response = client.graphql(&query, None).await?;
            output::print_output(&response, &config.output_format)?;
        }
    }

    Ok(())
}

fn get_analytics_data(response: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    response
        .get("data")
        .and_then(|d| d.get("viewer"))
        .and_then(|v| v.get("zones"))
        .and_then(|z| z.as_array())
        .and_then(|a| a.first())
        .and_then(|z| {
            z.get("httpRequestsAdaptiveGroups")
                .or_else(|| z.get("firewallEventsAdaptiveGroups"))
        })
        .and_then(|g| g.as_array())
}

fn print_analytics_response(response: &serde_json::Value, main_dim: &str) -> Result<()> {
    if let Some(groups) = get_analytics_data(response) {
        output::table_header(&["COUNT", main_dim.to_uppercase().as_str()]);

        for group in groups {
            let count = group.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
            let dims = group.get("dimensions").cloned().unwrap_or_default();
            output::print_analytics_row(count, &dims);
        }

        output::info(&format!("Total groups: {}", groups.len()));
    } else {
        output::warning("No data found");
    }

    Ok(())
}

fn print_bandwidth_response(response: &serde_json::Value) -> Result<()> {
    if let Some(groups) = response
        .get("data")
        .and_then(|d| d.get("viewer"))
        .and_then(|v| v.get("zones"))
        .and_then(|z| z.as_array())
        .and_then(|a| a.first())
        .and_then(|z| z.get("httpRequestsAdaptiveGroups"))
        .and_then(|g| g.as_array())
    {
        output::table_header(&["BYTES", "STATUS"]);

        for group in groups {
            let bytes = group
                .get("sum")
                .and_then(|s| s.get("edgeResponseBytes"))
                .and_then(|b| b.as_u64())
                .unwrap_or(0);

            let status = group
                .get("dimensions")
                .and_then(|d| d.get("edgeResponseStatus"))
                .and_then(|c| c.as_u64())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string());

            let formatted = format_bytes(bytes);
            println!("{}\t{}", formatted, status);
        }
    }

    Ok(())
}

fn print_firewall_response(response: &serde_json::Value) -> Result<()> {
    if let Some(groups) = response
        .get("data")
        .and_then(|d| d.get("viewer"))
        .and_then(|v| v.get("zones"))
        .and_then(|z| z.as_array())
        .and_then(|a| a.first())
        .and_then(|z| z.get("firewallEventsAdaptiveGroups"))
        .and_then(|g| g.as_array())
    {
        output::table_header(&["COUNT", "ACTION", "IP", "COUNTRY", "PATH"]);

        for group in groups {
            let count = group.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
            let dims = group.get("dimensions");

            let action = dims
                .and_then(|d| d.get("action"))
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let ip = dims
                .and_then(|d| d.get("clientIP"))
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let country = dims
                .and_then(|d| d.get("clientCountryName"))
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let path = dims
                .and_then(|d| d.get("clientRequestPath"))
                .and_then(|v| v.as_str())
                .unwrap_or("-");

            println!("{}\t{}\t{}\t{}\t{}", count, action, ip, country, path);
        }
    }

    Ok(())
}

fn print_hourly_response(response: &serde_json::Value) -> Result<()> {
    if let Some(groups) = response
        .get("data")
        .and_then(|d| d.get("viewer"))
        .and_then(|v| v.get("zones"))
        .and_then(|z| z.as_array())
        .and_then(|a| a.first())
        .and_then(|z| z.get("httpRequests1hGroups"))
        .and_then(|g| g.as_array())
    {
        output::table_header(&["DATETIME", "REQUESTS", "BYTES", "CACHED", "THREATS"]);

        for group in groups {
            let datetime = group
                .get("dimensions")
                .and_then(|d| d.get("datetime"))
                .and_then(|v| v.as_str())
                .unwrap_or("-");

            let sum = group.get("sum");
            let requests = sum
                .and_then(|s| s.get("requests"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let bytes = sum
                .and_then(|s| s.get("bytes"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let cached = sum
                .and_then(|s| s.get("cachedBytes"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let threats = sum
                .and_then(|s| s.get("threats"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            println!(
                "{}\t{}\t{}\t{}\t{}",
                datetime,
                requests,
                format_bytes(bytes),
                format_bytes(cached),
                threats
            );
        }
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
