//! Raw API command

use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct RawArgs {
    /// API path (e.g., /zones, /zones/:zone_id/dns_records)
    pub path: String,
    
    /// HTTP method
    #[arg(short, long, default_value = "GET")]
    pub method: String,
    
    /// Request body (JSON)
    #[arg(short, long)]
    pub body: Option<String>,
    
    /// Zone placeholder replacement
    #[arg(short, long)]
    pub zone: Option<String>,
}

pub async fn execute(config: &Config, args: RawArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;
    
    // Replace :zone_id placeholder if zone is provided
    let mut path = args.path.clone();
    if path.contains(":zone_id") || path.contains("{zone_id}") {
        let zone = config.resolve_zone(args.zone.as_deref())?;
        let zone_id = client.resolve_zone_id(&zone).await?;
        path = path.replace(":zone_id", &zone_id).replace("{zone_id}", &zone_id);
    }
    
    // Ensure path starts with /
    if !path.starts_with('/') {
        path = format!("/{}", path);
    }
    
    let method = args.method.to_uppercase();
    
    let response = match method.as_str() {
        "GET" => client.get_raw(&path).await?,
        "POST" => {
            let body: Value = args.body
                .as_ref()
                .map(|b| serde_json::from_str(b))
                .transpose()?
                .unwrap_or(Value::Object(Default::default()));
            client.post_raw(&path, body).await?
        }
        "PUT" => {
            let body: Value = args.body
                .as_ref()
                .map(|b| serde_json::from_str(b))
                .transpose()?
                .unwrap_or(Value::Object(Default::default()));
            client.put_raw(&path, body).await?
        }
        "PATCH" => {
            let body: Value = args.body
                .as_ref()
                .map(|b| serde_json::from_str(b))
                .transpose()?
                .unwrap_or(Value::Object(Default::default()));
            client.patch_raw(&path, body).await?
        }
        "DELETE" => client.delete_raw(&path).await?,
        _ => {
            output::error(&format!("Unsupported HTTP method: {}", method));
            return Ok(());
        }
    };
    
    output::print_output(&response, &config.output_format)?;
    
    Ok(())
}

