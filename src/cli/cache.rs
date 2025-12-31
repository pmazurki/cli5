//! Cache command

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct CacheArgs {
    /// Zone name or ID
    #[arg(short, long)]
    pub zone: Option<String>,
    
    #[command(subcommand)]
    pub command: CacheCommand,
}

#[derive(Subcommand, Debug)]
pub enum CacheCommand {
    /// Purge entire cache
    PurgeAll {
        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
    
    /// Purge specific URLs
    PurgeUrls {
        /// URLs to purge (comma-separated or multiple arguments)
        #[arg(required = true)]
        urls: Vec<String>,
    },
    
    /// Purge by cache tags (Enterprise)
    PurgeTags {
        /// Cache tags to purge
        #[arg(required = true)]
        tags: Vec<String>,
    },
    
    /// Purge by prefix (Enterprise)
    PurgePrefixes {
        /// URL prefixes to purge
        #[arg(required = true)]
        prefixes: Vec<String>,
    },
    
    /// Purge by hostname
    PurgeHosts {
        /// Hostnames to purge
        #[arg(required = true)]
        hosts: Vec<String>,
    },
}

pub async fn execute(config: &Config, args: CacheArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;
    let zone = config.resolve_zone(args.zone.as_deref())?;
    let zone_id = client.resolve_zone_id(&zone).await?;
    
    match args.command {
        CacheCommand::PurgeAll { yes } => {
            if !yes {
                output::warning("This will purge the ENTIRE cache for this zone!");
                output::info("Use -y to confirm");
                return Ok(());
            }
            
            let body = json!({"purge_everything": true});
            client.post_raw(&format!("/zones/{}/purge_cache", zone_id), body).await?;
            output::success("Cache purged successfully!");
        }
        
        CacheCommand::PurgeUrls { urls } => {
            // Flatten comma-separated URLs
            let all_urls: Vec<String> = urls
                .iter()
                .flat_map(|u| u.split(',').map(|s| s.trim().to_string()))
                .collect();
            
            let body = json!({"files": all_urls});
            client.post_raw(&format!("/zones/{}/purge_cache", zone_id), body).await?;
            output::success(&format!("Purged {} URLs", all_urls.len()));
        }
        
        CacheCommand::PurgeTags { tags } => {
            let body = json!({"tags": tags});
            client.post_raw(&format!("/zones/{}/purge_cache", zone_id), body).await?;
            output::success(&format!("Purged cache tags: {}", tags.join(", ")));
        }
        
        CacheCommand::PurgePrefixes { prefixes } => {
            let body = json!({"prefixes": prefixes});
            client.post_raw(&format!("/zones/{}/purge_cache", zone_id), body).await?;
            output::success(&format!("Purged prefixes: {}", prefixes.join(", ")));
        }
        
        CacheCommand::PurgeHosts { hosts } => {
            let body = json!({"hosts": hosts});
            client.post_raw(&format!("/zones/{}/purge_cache", zone_id), body).await?;
            output::success(&format!("Purged hosts: {}", hosts.join(", ")));
        }
    }
    
    Ok(())
}

