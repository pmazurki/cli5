//! Pages command

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct PagesArgs {
    #[command(subcommand)]
    pub command: PagesCommand,
}

#[derive(Subcommand, Debug)]
pub enum PagesCommand {
    /// List all Pages projects
    List,

    /// Show project details
    Info {
        /// Project name
        name: String,
    },

    /// Create a new Pages project
    Create {
        /// Project name
        name: String,

        /// Production branch
        #[arg(short, long, default_value = "main")]
        branch: String,
    },

    /// Delete a Pages project
    Delete {
        /// Project name
        name: String,
    },

    /// List deployments for a project
    Deployments {
        /// Project name
        name: String,
    },
}

pub async fn execute(config: &Config, args: PagesArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;
    let account_id = get_account_id(&client).await?;

    match args.command {
        PagesCommand::List => {
            let path = format!("/accounts/{}/pages/projects", account_id);
            let response = client.get_raw(&path).await?;

            if let Some(projects) = response.get("result").and_then(|r| r.as_array()) {
                if projects.is_empty() {
                    output::info("No Pages projects found");
                    println!();
                    println!("Create your first Pages project:");
                    println!("  wrangler pages project create my-site");
                    println!("  wrangler pages deploy ./dist");
                } else {
                    output::table_header(&["NAME", "SUBDOMAIN", "CREATED"]);

                    for project in projects {
                        let name = project.get("name").and_then(|n| n.as_str()).unwrap_or("-");
                        let subdomain = project
                            .get("subdomain")
                            .and_then(|s| s.as_str())
                            .unwrap_or("-");
                        let created = project
                            .get("created_on")
                            .and_then(|c| c.as_str())
                            .map(|s| s.split('T').next().unwrap_or(s))
                            .unwrap_or("-");

                        println!("{}\t{}\t{}", name, subdomain, created);
                    }
                    output::info(&format!("Total: {} projects", projects.len()));
                }
            }
        }

        PagesCommand::Info { name } => {
            let path = format!("/accounts/{}/pages/projects/{}", account_id, name);
            let response = client.get_raw(&path).await?;

            if let Some(result) = response.get("result") {
                let subdomain = result
                    .get("subdomain")
                    .and_then(|s| s.as_str())
                    .unwrap_or("-");
                let created = result
                    .get("created_on")
                    .and_then(|c| c.as_str())
                    .unwrap_or("-");

                println!("Name:       {}", name);
                println!("URL:        https://{}.pages.dev", subdomain);
                println!("Created:    {}", created);

                if let Some(prod) = result.get("production_branch").and_then(|p| p.as_str()) {
                    println!("Prod Branch:{}", prod);
                }
            }
        }

        PagesCommand::Create { name, branch } => {
            let path = format!("/accounts/{}/pages/projects", account_id);
            let body = serde_json::json!({
                "name": name,
                "production_branch": branch
            });

            let response = client.post_raw(&path, body).await?;

            if response.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                let subdomain = response
                    .get("result")
                    .and_then(|r| r.get("subdomain"))
                    .and_then(|s| s.as_str())
                    .unwrap_or(&name);

                output::success(&format!("Pages project '{}' created!", name));
                println!();
                println!("URL: https://{}", subdomain);
                println!();
                println!("Deploy with:");
                println!("  wrangler pages deploy ./dist --project-name {}", name);
            } else {
                let errors = response.get("errors").and_then(|e| e.as_array());
                if let Some(errs) = errors {
                    for err in errs {
                        let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
                        output::error(msg);
                    }
                }
            }
        }

        PagesCommand::Delete { name } => {
            let path = format!("/accounts/{}/pages/projects/{}", account_id, name);
            let response = client.delete_raw(&path).await?;

            if response.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                output::success(&format!("Pages project '{}' deleted!", name));
            } else {
                output::error("Failed to delete project");
            }
        }

        PagesCommand::Deployments { name } => {
            let path = format!("/accounts/{}/pages/projects/{}/deployments", account_id, name);
            let response = client.get_raw(&path).await?;

            if let Some(deployments) = response.get("result").and_then(|r| r.as_array()) {
                if deployments.is_empty() {
                    output::info("No deployments found");
                } else {
                    output::table_header(&["ID", "ENV", "STATUS", "CREATED"]);

                    for deploy in deployments.iter().take(10) {
                        let id = deploy
                            .get("id")
                            .and_then(|i| i.as_str())
                            .map(|s| &s[..8])
                            .unwrap_or("-");
                        let env = deploy
                            .get("environment")
                            .and_then(|e| e.as_str())
                            .unwrap_or("-");
                        let status = deploy
                            .get("latest_stage")
                            .and_then(|l| l.get("status"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("-");
                        let created = deploy
                            .get("created_on")
                            .and_then(|c| c.as_str())
                            .map(|s| s.split('T').next().unwrap_or(s))
                            .unwrap_or("-");

                        let status_colored = match status {
                            "success" => format!("\x1b[32m{}\x1b[0m", status),
                            "failure" => format!("\x1b[31m{}\x1b[0m", status),
                            _ => status.to_string(),
                        };

                        println!("{}\t{}\t{}\t{}", id, env, status_colored, created);
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

