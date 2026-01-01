//! Storage commands - KV, D1, Queues, Vectorize, Hyperdrive, R2

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct StorageArgs {
    #[command(subcommand)]
    pub command: StorageCommand,
}

#[derive(Subcommand, Debug)]
pub enum StorageCommand {
    /// Workers KV - Key-Value storage
    Kv {
        #[command(subcommand)]
        cmd: KvCommand,
    },

    /// D1 - SQLite database at the edge
    D1 {
        #[command(subcommand)]
        cmd: D1Command,
    },

    /// Queues - Message queues
    Queues {
        #[command(subcommand)]
        cmd: QueuesCommand,
    },

    /// Vectorize - Vector database for AI
    Vectorize {
        #[command(subcommand)]
        cmd: VectorizeCommand,
    },

    /// Hyperdrive - Database connection pooling
    Hyperdrive {
        #[command(subcommand)]
        cmd: HyperdriveCommand,
    },

    /// R2 - Object storage (S3-compatible)
    R2 {
        #[command(subcommand)]
        cmd: R2Command,
    },
}

// ============ KV Commands ============

#[derive(Subcommand, Debug)]
pub enum KvCommand {
    /// List KV namespaces
    List,
    /// Create KV namespace
    Create { title: String },
    /// Delete KV namespace
    Delete { namespace_id: String },
    /// List keys in namespace
    Keys { namespace_id: String },
    /// Get value
    Get { namespace_id: String, key: String },
    /// Put value
    Put { namespace_id: String, key: String, value: String },
}

// ============ D1 Commands ============

#[derive(Subcommand, Debug)]
pub enum D1Command {
    /// List D1 databases
    List,
    /// Create D1 database
    Create { name: String },
    /// Delete D1 database
    Delete { database_id: String },
    /// Execute SQL query
    Query { database_id: String, sql: String },
}

// ============ Queues Commands ============

#[derive(Subcommand, Debug)]
pub enum QueuesCommand {
    /// List queues
    List,
    /// Create queue
    Create { name: String },
    /// Delete queue
    Delete { queue_id: String },
}

// ============ Vectorize Commands ============

#[derive(Subcommand, Debug)]
pub enum VectorizeCommand {
    /// List Vectorize indexes
    List,
    /// Create Vectorize index
    Create {
        name: String,
        #[arg(short, long, default_value = "768")]
        dimensions: u32,
        #[arg(short, long, default_value = "cosine")]
        metric: String,
    },
    /// Delete Vectorize index
    Delete { name: String },
}

// ============ Hyperdrive Commands ============

#[derive(Subcommand, Debug)]
pub enum HyperdriveCommand {
    /// List Hyperdrive configs
    List,
    /// Create Hyperdrive config
    Create {
        name: String,
        #[arg(long)]
        connection_string: String,
    },
    /// Delete Hyperdrive config
    Delete { config_id: String },
}

// ============ R2 Commands ============

#[derive(Subcommand, Debug)]
pub enum R2Command {
    /// List R2 buckets
    List,
    /// Create R2 bucket
    Create { name: String },
    /// Delete R2 bucket
    Delete { name: String },
}

pub async fn execute(config: &Config, args: StorageArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;
    let account_id = get_account_id(&client).await?;

    match args.command {
        StorageCommand::Kv { cmd } => execute_kv(&client, &account_id, cmd).await,
        StorageCommand::D1 { cmd } => execute_d1(&client, &account_id, cmd).await,
        StorageCommand::Queues { cmd } => execute_queues(&client, &account_id, cmd).await,
        StorageCommand::Vectorize { cmd } => execute_vectorize(&client, &account_id, cmd).await,
        StorageCommand::Hyperdrive { cmd } => execute_hyperdrive(&client, &account_id, cmd).await,
        StorageCommand::R2 { cmd } => execute_r2(&client, &account_id, cmd).await,
    }
}

// ============ KV Implementation ============

async fn execute_kv(client: &CloudflareClient, account_id: &str, cmd: KvCommand) -> Result<()> {
    match cmd {
        KvCommand::List => {
            let path = format!("/accounts/{}/storage/kv/namespaces", account_id);
            let response = client.get_raw(&path).await?;
            print_list(&response, &["TITLE", "ID"], |item| {
                vec![
                    item.get("title").and_then(|t| t.as_str()).unwrap_or("-").to_string(),
                    item.get("id").and_then(|i| i.as_str()).unwrap_or("-").to_string(),
                ]
            });
        }
        KvCommand::Create { title } => {
            let path = format!("/accounts/{}/storage/kv/namespaces", account_id);
            let body = json!({ "title": title });
            let response = client.post_raw(&path, body).await?;
            if response.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                let id = response.get("result").and_then(|r| r.get("id")).and_then(|i| i.as_str()).unwrap_or("-");
                output::success(&format!("KV namespace '{}' created! ID: {}", title, id));
            }
        }
        KvCommand::Delete { namespace_id } => {
            let path = format!("/accounts/{}/storage/kv/namespaces/{}", account_id, namespace_id);
            client.delete_raw(&path).await?;
            output::success("KV namespace deleted!");
        }
        KvCommand::Keys { namespace_id } => {
            let path = format!("/accounts/{}/storage/kv/namespaces/{}/keys", account_id, namespace_id);
            let response = client.get_raw(&path).await?;
            print_list(&response, &["KEY", "EXPIRATION"], |item| {
                vec![
                    item.get("name").and_then(|n| n.as_str()).unwrap_or("-").to_string(),
                    item.get("expiration").and_then(|e| e.as_u64()).map(|e| e.to_string()).unwrap_or("-".to_string()),
                ]
            });
        }
        KvCommand::Get { namespace_id, key } => {
            let path = format!("/accounts/{}/storage/kv/namespaces/{}/values/{}", account_id, namespace_id, key);
            let response = client.get_raw(&path).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        KvCommand::Put { namespace_id, key, value } => {
            let path = format!("/accounts/{}/storage/kv/namespaces/{}/values/{}", account_id, namespace_id, key);
            let response = client.put_raw(&path, json!(value)).await?;
            if response.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                output::success(&format!("Key '{}' saved!", key));
            }
        }
    }
    Ok(())
}

// ============ D1 Implementation ============

async fn execute_d1(client: &CloudflareClient, account_id: &str, cmd: D1Command) -> Result<()> {
    match cmd {
        D1Command::List => {
            let path = format!("/accounts/{}/d1/database", account_id);
            let response = client.get_raw(&path).await?;
            print_list(&response, &["NAME", "ID", "VERSION"], |item| {
                vec![
                    item.get("name").and_then(|n| n.as_str()).unwrap_or("-").to_string(),
                    item.get("uuid").and_then(|i| i.as_str()).unwrap_or("-").to_string(),
                    item.get("version").and_then(|v| v.as_str()).unwrap_or("-").to_string(),
                ]
            });
        }
        D1Command::Create { name } => {
            let path = format!("/accounts/{}/d1/database", account_id);
            let body = json!({ "name": name });
            let response = client.post_raw(&path, body).await?;
            if response.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                let id = response.get("result").and_then(|r| r.get("uuid")).and_then(|i| i.as_str()).unwrap_or("-");
                output::success(&format!("D1 database '{}' created! ID: {}", name, id));
            }
        }
        D1Command::Delete { database_id } => {
            let path = format!("/accounts/{}/d1/database/{}", account_id, database_id);
            client.delete_raw(&path).await?;
            output::success("D1 database deleted!");
        }
        D1Command::Query { database_id, sql } => {
            let path = format!("/accounts/{}/d1/database/{}/query", account_id, database_id);
            let body = json!({ "sql": sql });
            let response = client.post_raw(&path, body).await?;
            println!("{}", serde_json::to_string_pretty(&response.get("result").unwrap_or(&json!({})))?);
        }
    }
    Ok(())
}

// ============ Queues Implementation ============

async fn execute_queues(client: &CloudflareClient, account_id: &str, cmd: QueuesCommand) -> Result<()> {
    match cmd {
        QueuesCommand::List => {
            let path = format!("/accounts/{}/queues", account_id);
            let response = client.get_raw(&path).await?;
            print_list(&response, &["NAME", "ID", "CREATED"], |item| {
                vec![
                    item.get("queue_name").and_then(|n| n.as_str()).unwrap_or("-").to_string(),
                    item.get("queue_id").and_then(|i| i.as_str()).unwrap_or("-").to_string(),
                    item.get("created_on").and_then(|c| c.as_str()).map(|s| s.split('T').next().unwrap_or(s)).unwrap_or("-").to_string(),
                ]
            });
        }
        QueuesCommand::Create { name } => {
            let path = format!("/accounts/{}/queues", account_id);
            let body = json!({ "queue_name": name });
            let response = client.post_raw(&path, body).await?;
            if response.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                let id = response.get("result").and_then(|r| r.get("queue_id")).and_then(|i| i.as_str()).unwrap_or("-");
                output::success(&format!("Queue '{}' created! ID: {}", name, id));
            }
        }
        QueuesCommand::Delete { queue_id } => {
            let path = format!("/accounts/{}/queues/{}", account_id, queue_id);
            client.delete_raw(&path).await?;
            output::success("Queue deleted!");
        }
    }
    Ok(())
}

// ============ Vectorize Implementation ============

async fn execute_vectorize(client: &CloudflareClient, account_id: &str, cmd: VectorizeCommand) -> Result<()> {
    match cmd {
        VectorizeCommand::List => {
            let path = format!("/accounts/{}/vectorize/indexes", account_id);
            let response = client.get_raw(&path).await?;
            print_list(&response, &["NAME", "DIMENSIONS", "METRIC"], |item| {
                vec![
                    item.get("name").and_then(|n| n.as_str()).unwrap_or("-").to_string(),
                    item.get("config").and_then(|c| c.get("dimensions")).and_then(|d| d.as_u64()).map(|d| d.to_string()).unwrap_or("-".to_string()),
                    item.get("config").and_then(|c| c.get("metric")).and_then(|m| m.as_str()).unwrap_or("-").to_string(),
                ]
            });
        }
        VectorizeCommand::Create { name, dimensions, metric } => {
            let path = format!("/accounts/{}/vectorize/indexes", account_id);
            let body = json!({
                "name": name,
                "config": {
                    "dimensions": dimensions,
                    "metric": metric
                }
            });
            let response = client.post_raw(&path, body).await?;
            if response.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                output::success(&format!("Vectorize index '{}' created!", name));
            }
        }
        VectorizeCommand::Delete { name } => {
            let path = format!("/accounts/{}/vectorize/indexes/{}", account_id, name);
            client.delete_raw(&path).await?;
            output::success("Vectorize index deleted!");
        }
    }
    Ok(())
}

// ============ Hyperdrive Implementation ============

async fn execute_hyperdrive(client: &CloudflareClient, account_id: &str, cmd: HyperdriveCommand) -> Result<()> {
    match cmd {
        HyperdriveCommand::List => {
            let path = format!("/accounts/{}/hyperdrive/configs", account_id);
            let response = client.get_raw(&path).await?;
            print_list(&response, &["NAME", "ID"], |item| {
                vec![
                    item.get("name").and_then(|n| n.as_str()).unwrap_or("-").to_string(),
                    item.get("id").and_then(|i| i.as_str()).unwrap_or("-").to_string(),
                ]
            });
        }
        HyperdriveCommand::Create { name, connection_string } => {
            // Parse connection string: postgres://user:pass@host:port/database
            let path = format!("/accounts/{}/hyperdrive/configs", account_id);
            let body = json!({
                "name": name,
                "origin": {
                    "connection_string": connection_string
                }
            });
            let response = client.post_raw(&path, body).await?;
            if response.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                let id = response.get("result").and_then(|r| r.get("id")).and_then(|i| i.as_str()).unwrap_or("-");
                output::success(&format!("Hyperdrive config '{}' created! ID: {}", name, id));
            }
        }
        HyperdriveCommand::Delete { config_id } => {
            let path = format!("/accounts/{}/hyperdrive/configs/{}", account_id, config_id);
            client.delete_raw(&path).await?;
            output::success("Hyperdrive config deleted!");
        }
    }
    Ok(())
}

// ============ R2 Implementation ============

async fn execute_r2(client: &CloudflareClient, account_id: &str, cmd: R2Command) -> Result<()> {
    match cmd {
        R2Command::List => {
            let path = format!("/accounts/{}/r2/buckets", account_id);
            let response = client.get_raw(&path).await?;
            print_list(&response, &["NAME", "CREATED"], |item| {
                vec![
                    item.get("name").and_then(|n| n.as_str()).unwrap_or("-").to_string(),
                    item.get("creation_date").and_then(|c| c.as_str()).map(|s| s.split('T').next().unwrap_or(s)).unwrap_or("-").to_string(),
                ]
            });
        }
        R2Command::Create { name } => {
            let path = format!("/accounts/{}/r2/buckets", account_id);
            let body = json!({ "name": name });
            let response = client.post_raw(&path, body).await?;
            if response.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                output::success(&format!("R2 bucket '{}' created!", name));
            }
        }
        R2Command::Delete { name } => {
            let path = format!("/accounts/{}/r2/buckets/{}", account_id, name);
            client.delete_raw(&path).await?;
            output::success("R2 bucket deleted!");
        }
    }
    Ok(())
}

// ============ Helpers ============

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

fn print_list<F>(response: &serde_json::Value, headers: &[&str], row_fn: F)
where
    F: Fn(&serde_json::Value) -> Vec<String>,
{
    if let Some(items) = response.get("result").and_then(|r| r.as_array()) {
        if items.is_empty() {
            output::info("No items found");
        } else {
            output::table_header(headers);
            for item in items {
                let row = row_fn(item);
                println!("{}", row.join("\t"));
            }
            output::info(&format!("Total: {} items", items.len()));
        }
    }
}

