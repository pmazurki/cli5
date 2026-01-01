//! Workers command

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct WorkersArgs {
    #[command(subcommand)]
    pub command: WorkersCommand,
}

#[derive(Subcommand, Debug)]
pub enum WorkersCommand {
    /// List all Workers scripts
    List,

    /// Show Worker script details
    Info {
        /// Script name
        name: String,
    },

    /// Create a simple hello-world Worker
    Create {
        /// Script name
        name: String,

        /// Custom response message (optional)
        #[arg(short, long, default_value = "Hello from cli5 Worker!")]
        message: String,
    },

    /// Delete a Worker script
    Delete {
        /// Script name
        name: String,
    },

    /// List Workers KV namespaces
    Kv,

    /// List Worker routes for a zone
    Routes {
        /// Zone name or ID
        #[arg(short, long)]
        zone: String,
    },

    /// Add a route for a Worker
    AddRoute {
        /// Zone name or ID
        #[arg(short, long)]
        zone: String,

        /// Route pattern (e.g., "example.com/*")
        #[arg(short, long)]
        pattern: String,

        /// Worker script name
        #[arg(short, long)]
        script: String,
    },
}

pub async fn execute(config: &Config, args: WorkersArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;
    let account_id = get_account_id(&client).await?;

    match args.command {
        WorkersCommand::List => {
            let path = format!("/accounts/{}/workers/scripts", account_id);
            let response = client.get_raw(&path).await?;

            if let Some(scripts) = response.get("result").and_then(|r| r.as_array()) {
                if scripts.is_empty() {
                    output::info("No Workers scripts found");
                    println!();
                    println!("Create your first Worker:");
                    println!("  wrangler init my-worker");
                    println!("  wrangler deploy");
                } else {
                    output::table_header(&["NAME", "CREATED", "MODIFIED"]);

                    for script in scripts {
                        let name = script.get("id").and_then(|n| n.as_str()).unwrap_or("-");
                        let created = script
                            .get("created_on")
                            .and_then(|c| c.as_str())
                            .map(|s| s.split('T').next().unwrap_or(s))
                            .unwrap_or("-");
                        let modified = script
                            .get("modified_on")
                            .and_then(|m| m.as_str())
                            .map(|s| s.split('T').next().unwrap_or(s))
                            .unwrap_or("-");

                        println!("{}\t{}\t{}", name, created, modified);
                    }
                    output::info(&format!("Total: {} scripts", scripts.len()));
                }
            }
        }

        WorkersCommand::Info { name } => {
            let path = format!("/accounts/{}/workers/scripts/{}", account_id, name);
            let response = client.get_raw(&path).await?;

            output::print_output(&response, &config.output_format)?;
        }

        WorkersCommand::Kv => {
            let path = format!("/accounts/{}/storage/kv/namespaces", account_id);
            let response = client.get_raw(&path).await?;

            if let Some(namespaces) = response.get("result").and_then(|r| r.as_array()) {
                if namespaces.is_empty() {
                    output::info("No KV namespaces found");
                } else {
                    output::table_header(&["TITLE", "ID"]);

                    for ns in namespaces {
                        let title = ns.get("title").and_then(|t| t.as_str()).unwrap_or("-");
                        let id = ns.get("id").and_then(|i| i.as_str()).unwrap_or("-");
                        println!("{}\t{}", title, id);
                    }
                }
            }
        }

        WorkersCommand::Create { name, message } => {
            // Create a simple service worker (legacy format - most compatible)
            let script = format!(
                r#"addEventListener("fetch", event => {{ event.respondWith(new Response("{}")) }})"#,
                message
            );

            let path = format!("/accounts/{}/workers/scripts/{}", account_id, name);

            // Use service worker format (simpler)
            let response = client.put_worker_script(&path, &script, false).await?;

            if response
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false)
            {
                output::success(&format!("Worker '{}' created!", name));
                println!();
                println!("To add a route:");
                println!(
                    "  cli5 workers add-route --zone maz.ie --pattern '*.maz.ie/api/*' --script {}",
                    name
                );
            } else {
                let errors = response.get("errors").and_then(|e| e.as_array());
                if let Some(errs) = errors {
                    for err in errs {
                        let msg = err
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown error");
                        output::error(msg);
                    }
                }
            }
        }

        WorkersCommand::Delete { name } => {
            let path = format!("/accounts/{}/workers/scripts/{}", account_id, name);
            let response = client.delete_raw(&path).await?;

            if response
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false)
            {
                output::success(&format!("Worker '{}' deleted!", name));
            } else {
                output::error("Failed to delete worker");
            }
        }

        WorkersCommand::Routes { zone } => {
            let zone_id = client.resolve_zone_id(&zone).await?;
            let path = format!("/zones/{}/workers/routes", zone_id);
            let response = client.get_raw(&path).await?;

            if let Some(routes) = response.get("result").and_then(|r| r.as_array()) {
                if routes.is_empty() {
                    output::info("No routes found");
                } else {
                    output::table_header(&["PATTERN", "SCRIPT", "ID"]);

                    for route in routes {
                        let pattern = route.get("pattern").and_then(|p| p.as_str()).unwrap_or("-");
                        let script = route.get("script").and_then(|s| s.as_str()).unwrap_or("-");
                        let id = route.get("id").and_then(|i| i.as_str()).unwrap_or("-");
                        println!("{}\t{}\t{}", pattern, script, id);
                    }
                }
            }
        }

        WorkersCommand::AddRoute {
            zone,
            pattern,
            script,
        } => {
            let zone_id = client.resolve_zone_id(&zone).await?;
            let path = format!("/zones/{}/workers/routes", zone_id);
            let body = serde_json::json!({
                "pattern": pattern,
                "script": script
            });

            let response = client.post_raw(&path, body).await?;

            if response
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false)
            {
                output::success(&format!("Route '{}' -> '{}' created!", pattern, script));
            } else {
                let errors = response.get("errors").and_then(|e| e.as_array());
                if let Some(errs) = errors {
                    for err in errs {
                        let msg = err
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown error");
                        output::error(msg);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn get_account_id(client: &CloudflareClient) -> Result<String> {
    let response = client.get_raw("/zones?per_page=1").await?;

    if let Some(zones) = response.get("result").and_then(|r| r.as_array()) {
        if let Some(zone) = zones.first() {
            if let Some(account) = zone.get("account") {
                if let Some(id) = account.get("id").and_then(|i| i.as_str()) {
                    return Ok(id.to_string());
                }
            }
        }
    }

    Err(anyhow::anyhow!("Could not determine account ID"))
}
