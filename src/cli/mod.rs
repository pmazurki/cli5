//! CLI commands module

pub mod analytics;
pub mod cache;
pub mod config_cmd;
pub mod dns;
pub mod firewall;
pub mod raw;
pub mod settings;
pub mod zones;

use clap::{Parser, Subcommand};

/// CLI5 - Modern Cloudflare CLI
#[derive(Parser, Debug)]
#[command(name = "cli5")]
#[command(author = "Your Name")]
#[command(version = "0.1.0")]
#[command(about = "Modern Cloudflare CLI - REST & GraphQL API client", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output format: json, table, compact
    #[arg(short, long, global = true)]
    pub format: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List and manage zones (domains)
    Zones(zones::ZonesArgs),

    /// Manage DNS records
    Dns(dns::DnsArgs),

    /// Manage zone settings
    Settings(settings::SettingsArgs),

    /// Manage firewall rules
    Firewall(firewall::FirewallArgs),

    /// Cache management
    Cache(cache::CacheArgs),

    /// Analytics and logs (GraphQL)
    Analytics(analytics::AnalyticsArgs),

    /// Raw API requests
    Raw(raw::RawArgs),

    /// Configuration management
    Config(config_cmd::ConfigArgs),
}
