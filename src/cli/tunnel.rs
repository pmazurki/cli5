//! Cloudflare Tunnel commands - secure connections to your infrastructure
//!
//! Smart tunnel system:
//! - Admin mode (has CF_API_KEY): Creates tunnel, adds DNS, runs tunnel
//! - User mode (has TUNNEL_TOKEN): Just runs tunnel with token
//!
//! Single command: `cli5 tunnel start <hostname> --port <port>`

use anyhow::Result;
use chrono::Utc;
use clap::{Args, Subcommand, ValueEnum};
use serde_json::json;

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

/// Tunnel method for quick start
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum TunnelMethod {
    /// Quick tunnel - random URL, instant, no config needed
    #[default]
    Quick,
    /// Named tunnel - custom hostname, requires one-time setup
    Named,
    /// Hybrid - tries named first, falls back to quick
    Hybrid,
}

// Quick tunnel subcommands (legacy, kept for compatibility)
#[derive(Subcommand, Debug, Clone)]
pub enum QuickCommand {
    /// Start a tunnel with chosen method
    Start {
        #[arg(short, long, default_value = "8080")]
        port: u16,
        #[arg(long, default_value = "http")]
        protocol: String,
        #[arg(short, long, value_enum, default_value = "quick")]
        method: TunnelMethod,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(long)]
        domain: Option<String>,
        #[arg(short, long)]
        background: bool,
    },
    Stop {
        name: Option<String>,
    },
    Status,
    Setup {
        name: String,
        domain: String,
        #[arg(long, default_value = "support")]
        subdomain: String,
    },
    List,
}

#[derive(Args, Debug)]
pub struct TunnelArgs {
    #[command(subcommand)]
    pub command: TunnelCommand,
}

#[derive(Subcommand, Debug)]
pub enum TunnelCommand {
    /// 🚀 Smart start - auto-detects admin/user mode
    /// Admin (has CF_API_KEY): creates tunnel + DNS + runs
    /// User (has TUNNEL_TOKEN): just runs with token
    Start {
        /// Hostname (e.g., support.example.com)
        hostname: Option<String>,

        /// Local port to expose
        #[arg(short, long, default_value = "22")]
        port: u16,

        /// Protocol: http, https, tcp, ssh
        #[arg(long, default_value = "ssh")]
        protocol: String,

        /// Tunnel token (for user mode, or use TUNNEL_TOKEN env var)
        #[arg(short, long, env = "TUNNEL_TOKEN")]
        token: Option<String>,

        /// Run in background
        #[arg(short, long)]
        background: bool,
    },

    /// List all tunnels
    List,

    /// Create a new tunnel (admin only)
    Create {
        /// Tunnel name
        name: String,

        /// Domain for DNS record (e.g., example.com)
        #[arg(long)]
        domain: Option<String>,
    },

    /// Delete a tunnel
    Delete {
        /// Tunnel ID
        tunnel_id: String,
    },

    /// Get tunnel details
    Info {
        /// Tunnel ID
        tunnel_id: String,
    },

    /// Get tunnel token (for cloudflared)
    Token {
        /// Tunnel ID
        tunnel_id: String,
    },

    /// Install cloudflared client
    InstallClient,

    /// Run tunnel (requires cloudflared)
    Run {
        /// Tunnel ID or name
        tunnel: String,
        /// Run in background
        #[arg(short, long)]
        background: bool,
    },

    /// Stop running tunnel
    Stop {
        /// Tunnel name (for finding PID)
        tunnel: Option<String>,
    },

    /// Show tunnel client status
    Status,

    /// Quick tunnel - no config needed, instant URL
    Quick {
        #[command(subcommand)]
        cmd: QuickCommand,
    },

    /// List tunnel configurations
    Config {
        /// Tunnel ID
        tunnel_id: String,
    },

    /// List private network routes
    Routes,

    /// Add a route to tunnel
    AddRoute {
        /// CIDR range (e.g., 192.168.1.0/24)
        cidr: String,
        /// Tunnel ID
        #[arg(long)]
        tunnel: String,
        /// Comment
        #[arg(long)]
        comment: Option<String>,
    },

    /// Delete a route
    DeleteRoute {
        /// Route ID
        route_id: String,
    },

    /// List virtual networks
    Vnets,

    /// Create virtual network
    CreateVnet {
        /// Virtual network name
        name: String,
        /// Comment
        #[arg(long)]
        comment: Option<String>,
        /// Set as default
        #[arg(long)]
        default: bool,
    },

    /// Delete virtual network
    DeleteVnet {
        /// Virtual network ID
        vnet_id: String,
    },

    /// List WARP connectors
    Connectors,
}

pub async fn execute(config: &Config, args: TunnelArgs) -> Result<()> {
    // Commands that don't require API access
    match &args.command {
        TunnelCommand::Start {
            hostname,
            port,
            protocol,
            token,
            background,
        } => {
            return smart_start(
                config,
                hostname.clone(),
                *port,
                protocol,
                token.clone(),
                *background,
            )
            .await;
        }
        TunnelCommand::Stop { .. } => {
            return stop_tunnel().await;
        }
        TunnelCommand::Status => {
            return show_client_status().await;
        }
        TunnelCommand::InstallClient => {
            return install_cloudflared().await;
        }
        _ => {} // Continue to API-based commands
    }

    // All other commands require API access
    let client = CloudflareClient::new(config.clone())?;
    let account_id = get_account_id(&client).await?;

    match args.command {
        TunnelCommand::Start { .. }
        | TunnelCommand::Stop { .. }
        | TunnelCommand::Status
        | TunnelCommand::InstallClient => unreachable!(), // Handled above

        TunnelCommand::List => {
            let path = format!("/accounts/{}/cfd_tunnel?is_deleted=false", account_id);
            let response = client.get_raw(&path).await?;
            print_tunnels(&response);
        }

        TunnelCommand::Create { name, domain } => {
            // Check if tunnel already exists
            let check_path = format!(
                "/accounts/{}/cfd_tunnel?name={}&is_deleted=false",
                account_id, name
            );
            let check_response = client.get_raw(&check_path).await?;

            let tunnel_id =
                if let Some(tunnels) = check_response.get("result").and_then(|r| r.as_array()) {
                    if let Some(existing) = tunnels.first() {
                        let id = existing.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        output::warning(&format!("Tunnel '{}' already exists", name));
                        id.to_string()
                    } else {
                        // Create new tunnel
                        let path = format!("/accounts/{}/cfd_tunnel", account_id);
                        let secret = generate_tunnel_secret();
                        let body = json!({
                            "name": name,
                            "tunnel_secret": secret,
                            "config_src": "cloudflare"
                        });
                        let response = client.post_raw(&path, body).await?;
                        let id = response
                            .get("result")
                            .and_then(|r| r.get("id"))
                            .and_then(|i| i.as_str())
                            .unwrap_or("");
                        output::success(&format!("Tunnel '{}' created!", name));
                        id.to_string()
                    }
                } else {
                    return Err(anyhow::anyhow!("Failed to check existing tunnels"));
                };

            // Add DNS record if domain specified
            if let Some(domain) = domain {
                add_tunnel_dns(&client, &name, &domain, &tunnel_id).await?;
            }

            // Get and show token
            let token_path = format!("/accounts/{}/cfd_tunnel/{}/token", account_id, tunnel_id);
            let token_response = client.get_raw(&token_path).await?;
            if let Some(token) = token_response.get("result").and_then(|r| r.as_str()) {
                println!();
                println!("📋 Tunnel ID: {}", tunnel_id);
                println!();
                println!("🔐 Token for users (without CF API access):");
                println!("   TUNNEL_TOKEN={}", token);
                println!();
                println!("📋 User command:");
                println!("   cli5 tunnel start --token $TUNNEL_TOKEN --port 22");
            }
        }

        TunnelCommand::Delete { tunnel_id } => {
            let path = format!("/accounts/{}/cfd_tunnel/{}", account_id, tunnel_id);
            client.delete_raw(&path).await?;
            output::success("Tunnel deleted!");
        }

        TunnelCommand::Info { tunnel_id } => {
            let path = format!("/accounts/{}/cfd_tunnel/{}", account_id, tunnel_id);
            let response = client.get_raw(&path).await?;
            if let Some(result) = response.get("result") {
                println!("{}", serde_json::to_string_pretty(result)?);
            }
        }

        TunnelCommand::Token { tunnel_id } => {
            let path = format!("/accounts/{}/cfd_tunnel/{}/token", account_id, tunnel_id);
            let response = client.get_raw(&path).await?;
            if let Some(token) = response.get("result").and_then(|r| r.as_str()) {
                output::success("Tunnel token:");
                println!("\n{}", token);
                println!("\n📋 Usage:");
                println!("cli5 tunnel run {} --background", tunnel_id);
                println!("# or manually:");
                println!("cloudflared tunnel run --token {}", token);
            }
        }

        TunnelCommand::Run { tunnel, background } => {
            // Get token for tunnel
            let tunnel_id = resolve_tunnel_id(&client, &account_id, &tunnel).await?;
            let path = format!("/accounts/{}/cfd_tunnel/{}/token", account_id, tunnel_id);
            let response = client.get_raw(&path).await?;

            if let Some(token) = response.get("result").and_then(|r| r.as_str()) {
                run_tunnel(token, background).await?;
            } else {
                return Err(anyhow::anyhow!("Could not get tunnel token"));
            }
        }

        TunnelCommand::Quick { cmd } => {
            execute_quick(&client, &account_id, cmd).await?;
        }

        TunnelCommand::Config { tunnel_id } => {
            let path = format!(
                "/accounts/{}/cfd_tunnel/{}/configurations",
                account_id, tunnel_id
            );
            let response = client.get_raw(&path).await?;
            if let Some(result) = response.get("result") {
                println!("{}", serde_json::to_string_pretty(result)?);
            }
        }

        TunnelCommand::Routes => {
            let path = format!("/accounts/{}/teamnet/routes", account_id);
            let response = client.get_raw(&path).await?;
            print_routes(&response);
        }

        TunnelCommand::AddRoute {
            cidr,
            tunnel,
            comment,
        } => {
            let path = format!("/accounts/{}/teamnet/routes", account_id);
            let mut body = json!({
                "network": cidr,
                "tunnel_id": tunnel
            });
            if let Some(c) = comment {
                body["comment"] = json!(c);
            }
            let response = client.post_raw(&path, body).await?;
            if response
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false)
            {
                output::success(&format!("Route {} added to tunnel!", cidr));
            }
        }

        TunnelCommand::DeleteRoute { route_id } => {
            let path = format!("/accounts/{}/teamnet/routes/{}", account_id, route_id);
            client.delete_raw(&path).await?;
            output::success("Route deleted!");
        }

        TunnelCommand::Vnets => {
            let path = format!("/accounts/{}/teamnet/virtual_networks", account_id);
            let response = client.get_raw(&path).await?;
            print_vnets(&response);
        }

        TunnelCommand::CreateVnet {
            name,
            comment,
            default,
        } => {
            let path = format!("/accounts/{}/teamnet/virtual_networks", account_id);
            let mut body = json!({
                "name": name,
                "is_default_network": default
            });
            if let Some(c) = comment {
                body["comment"] = json!(c);
            }
            let response = client.post_raw(&path, body).await?;
            if response
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false)
            {
                output::success(&format!("Virtual network '{}' created!", name));
            }
        }

        TunnelCommand::DeleteVnet { vnet_id } => {
            let path = format!(
                "/accounts/{}/teamnet/virtual_networks/{}",
                account_id, vnet_id
            );
            client.delete_raw(&path).await?;
            output::success("Virtual network deleted!");
        }

        TunnelCommand::Connectors => {
            let path = format!("/accounts/{}/warp_connector", account_id);
            let response = client.get_raw(&path).await?;
            print_connectors(&response);
        }
    }

    Ok(())
}

// ============ Smart Start Implementation ============

async fn smart_start(
    config: &Config,
    hostname: Option<String>,
    port: u16,
    protocol: &str,
    token: Option<String>,
    background: bool,
) -> Result<()> {
    // Determine mode: Admin (has API key) or User (has token only)
    let has_api_key =
        config.api_token.is_some() || (config.api_key.is_some() && config.api_email.is_some());

    // Check for token first (user mode takes priority if token provided)
    if let Some(token) = token {
        // Check if cloudflared is installed
        let cloudflared = match get_cloudflared_path() {
            Some(p) => p,
            None => {
                output::info("cloudflared not installed. Installing...");
                download_cloudflared().await?
            }
        };
        // USER MODE: Just run with token
        output::info("🔑 User mode: Running tunnel with token");
        return run_tunnel_with_token(&cloudflared, &token, port, protocol, background).await;
    }

    // Check if cloudflared is installed for other modes
    let cloudflared = match get_cloudflared_path() {
        Some(p) => p,
        None => {
            output::info("cloudflared not installed. Installing...");
            download_cloudflared().await?
        }
    };

    if has_api_key {
        // ADMIN MODE: Create/use tunnel, add DNS, run
        let hostname = hostname.ok_or_else(|| {
            anyhow::anyhow!("Hostname required in admin mode.\nUsage: cli5 tunnel start support.example.com --port 22")
        })?;

        output::info(&format!(
            "🔧 Admin mode: Setting up tunnel for {}",
            hostname
        ));

        let client = CloudflareClient::new(config.clone())?;
        let account_id = get_account_id(&client).await?;

        // Parse hostname to get name and domain
        let parts: Vec<&str> = hostname.splitn(2, '.').collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!(
                "Invalid hostname. Use format: name.domain.com"
            ));
        }
        let name = parts[0];
        let domain = parts[1..].join(".");

        // Check/create tunnel
        let tunnel_id = get_or_create_tunnel(&client, &account_id, name).await?;

        // Add DNS record
        add_tunnel_dns(&client, name, &domain, &tunnel_id).await?;

        // Configure tunnel ingress
        configure_tunnel_ingress(&client, &account_id, &tunnel_id, &hostname, port, protocol)
            .await?;

        // Get token
        let token_path = format!("/accounts/{}/cfd_tunnel/{}/token", account_id, tunnel_id);
        let token_response = client.get_raw(&token_path).await?;
        let token = token_response
            .get("result")
            .and_then(|r| r.as_str())
            .ok_or_else(|| anyhow::anyhow!("Could not get tunnel token"))?;

        println!();
        output::success(&format!("Tunnel configured: {}", hostname));
        println!();
        println!("🔐 Token for users (without CF API access):");
        println!("   export TUNNEL_TOKEN='{}'", token);
        println!();
        println!("📋 User can run:");
        println!("   cli5 tunnel start --port {} --token $TUNNEL_TOKEN", port);
        println!();

        // Run the tunnel
        run_tunnel_with_token(&cloudflared, token, port, protocol, background).await
    } else {
        // NO CREDENTIALS
        println!();
        output::error("No credentials found!");
        println!();
        println!("Options:");
        println!("  1. Admin mode - set CF_API_TOKEN or CF_API_KEY + CF_API_EMAIL in .env");
        println!("  2. User mode  - use --token flag or set TUNNEL_TOKEN env var");
        println!();
        println!("Examples:");
        println!("  # Admin (creates tunnel + DNS):");
        println!("  cli5 tunnel start support.example.com --port 22");
        println!();
        println!("  # User (just runs with token):");
        println!("  cli5 tunnel start --token eyJhIjoiM... --port 22");
        println!();
        Err(anyhow::anyhow!("No credentials"))
    }
}

async fn get_or_create_tunnel(
    client: &CloudflareClient,
    account_id: &str,
    name: &str,
) -> Result<String> {
    // Check if tunnel exists
    let check_path = format!(
        "/accounts/{}/cfd_tunnel?name={}&is_deleted=false",
        account_id, name
    );
    let response = client.get_raw(&check_path).await?;

    if let Some(tunnels) = response.get("result").and_then(|r| r.as_array()) {
        if let Some(existing) = tunnels.first() {
            let id = existing.get("id").and_then(|i| i.as_str()).unwrap_or("");
            output::info(&format!("Using existing tunnel: {}", name));
            return Ok(id.to_string());
        }
    }

    // Create new tunnel
    let path = format!("/accounts/{}/cfd_tunnel", account_id);
    let secret = generate_tunnel_secret();
    let body = json!({
        "name": name,
        "tunnel_secret": secret,
        "config_src": "cloudflare"
    });
    let response = client.post_raw(&path, body).await?;
    let id = response
        .get("result")
        .and_then(|r| r.get("id"))
        .and_then(|i| i.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to create tunnel"))?;

    output::success(&format!("Created tunnel: {}", name));
    Ok(id.to_string())
}

async fn add_tunnel_dns(
    client: &CloudflareClient,
    name: &str,
    domain: &str,
    tunnel_id: &str,
) -> Result<()> {
    // Get zone ID for domain
    let zone_path = format!("/zones?name={}", domain);
    let zone_response = client.get_raw(&zone_path).await?;

    let zone_id = zone_response
        .get("result")
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first())
        .and_then(|z| z.get("id"))
        .and_then(|i| i.as_str())
        .ok_or_else(|| anyhow::anyhow!("Zone '{}' not found in your account", domain))?;

    let hostname = format!("{}.{}", name, domain);
    let tunnel_target = format!("{}.cfargotunnel.com", tunnel_id);

    // Check if DNS record already exists
    let dns_check_path = format!("/zones/{}/dns_records?name={}", zone_id, hostname);
    let dns_check = client.get_raw(&dns_check_path).await?;

    if let Some(records) = dns_check.get("result").and_then(|r| r.as_array()) {
        if let Some(existing) = records.first() {
            let record_type = existing.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let content = existing
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("");

            if record_type == "CNAME" && content == tunnel_target {
                output::info(&format!("DNS record {} already configured", hostname));
                return Ok(());
            } else {
                // Update existing record
                let record_id = existing.get("id").and_then(|i| i.as_str()).unwrap_or("");
                let update_path = format!("/zones/{}/dns_records/{}", zone_id, record_id);
                let body = json!({
                    "type": "CNAME",
                    "name": name,
                    "content": tunnel_target,
                    "proxied": true
                });
                client.put_raw(&update_path, body).await?;
                output::success(&format!("DNS record {} updated", hostname));
                return Ok(());
            }
        }
    }

    // Create new DNS record
    let dns_path = format!("/zones/{}/dns_records", zone_id);
    let body = json!({
        "type": "CNAME",
        "name": name,
        "content": tunnel_target,
        "proxied": true
    });

    client.post_raw(&dns_path, body).await?;
    output::success(&format!("DNS record {} created", hostname));

    Ok(())
}

async fn configure_tunnel_ingress(
    client: &CloudflareClient,
    account_id: &str,
    tunnel_id: &str,
    hostname: &str,
    port: u16,
    protocol: &str,
) -> Result<()> {
    let service = match protocol {
        "ssh" | "tcp" => format!("tcp://localhost:{}", port),
        "https" => format!("https://localhost:{}", port),
        _ => format!("http://localhost:{}", port),
    };

    let path = format!(
        "/accounts/{}/cfd_tunnel/{}/configurations",
        account_id, tunnel_id
    );
    let body = json!({
        "config": {
            "ingress": [
                {
                    "hostname": hostname,
                    "service": service
                },
                {
                    "service": "http_status:404"
                }
            ]
        }
    });

    match client.put_raw(&path, body).await {
        Ok(_) => {
            output::info(&format!(
                "Tunnel ingress configured: {} -> localhost:{}",
                hostname, port
            ));
        }
        Err(e) => {
            output::warning(&format!(
                "Could not configure ingress: {}. Configure in dashboard.",
                e
            ));
        }
    }

    Ok(())
}

async fn run_tunnel_with_token(
    cloudflared: &std::path::Path,
    token: &str,
    port: u16,
    protocol: &str,
    background: bool,
) -> Result<()> {
    let _ = (port, protocol); // These are configured in the tunnel, not needed here

    let pid_file = get_pid_file();

    // Check if already running
    if pid_file.exists() {
        let pid_str = std::fs::read_to_string(&pid_file)?;
        output::info(&format!("Tunnel already running (PID: {})", pid_str.trim()));
        return Ok(());
    }

    if background {
        let log_file = get_pid_file().with_extension("log");

        let child = std::process::Command::new(cloudflared)
            .args(["tunnel", "run", "--token", token])
            .stdout(std::fs::File::create(&log_file)?)
            .stderr(std::fs::File::create(&log_file)?)
            .spawn()?;

        std::fs::write(&pid_file, child.id().to_string())?;

        output::success(&format!("🟢 Tunnel started (PID: {})", child.id()));
        println!();
        println!("Stop with: cli5 tunnel stop");
    } else {
        output::info("🟢 Running tunnel (Ctrl+C to stop)...");
        println!();

        let status = std::process::Command::new(cloudflared)
            .args(["tunnel", "run", "--token", token])
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Tunnel exited with: {}", status));
        }
    }

    Ok(())
}

// ============ Helpers ============

fn generate_tunnel_secret() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // Base64 encoded 32-byte secret
    let secret_bytes: Vec<u8> = (0..32)
        .map(|i| ((timestamp >> (i % 16)) & 0xFF) as u8 ^ (i as u8).wrapping_mul(17))
        .collect();

    base64_encode(&secret_bytes)
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3F] as char);
        } else {
            result.push('=');
        }
    }

    result
}

fn print_tunnels(response: &serde_json::Value) {
    if let Some(tunnels) = response.get("result").and_then(|r| r.as_array()) {
        if tunnels.is_empty() {
            output::info("No tunnels found");
            println!("\n📋 Create one: cli5 tunnel create my-tunnel");
        } else {
            output::table_header(&["NAME", "ID", "STATUS", "CONNECTIONS"]);
            for t in tunnels {
                let name = t.get("name").and_then(|n| n.as_str()).unwrap_or("-");
                let id = t.get("id").and_then(|i| i.as_str()).unwrap_or("-");
                let status = t.get("status").and_then(|s| s.as_str()).unwrap_or("-");
                let conns = t
                    .get("connections")
                    .and_then(|c| c.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);

                let status_icon = match status {
                    "healthy" => "🟢",
                    "degraded" => "🟡",
                    "inactive" => "⚫",
                    _ => "⚪",
                };

                println!(
                    "{}\t{}\t{} {}\t{}",
                    name,
                    &id[..8],
                    status_icon,
                    status,
                    conns
                );
            }
            output::info(&format!("Total: {} tunnels", tunnels.len()));
        }
    }
}

fn print_routes(response: &serde_json::Value) {
    if let Some(routes) = response.get("result").and_then(|r| r.as_array()) {
        if routes.is_empty() {
            output::info("No routes found");
        } else {
            output::table_header(&["NETWORK", "TUNNEL", "COMMENT"]);
            for r in routes {
                let network = r.get("network").and_then(|n| n.as_str()).unwrap_or("-");
                let tunnel_id = r.get("tunnel_id").and_then(|t| t.as_str()).unwrap_or("-");
                let comment = r.get("comment").and_then(|c| c.as_str()).unwrap_or("-");
                println!("{}\t{}\t{}", network, &tunnel_id[..8], comment);
            }
        }
    }
}

fn print_vnets(response: &serde_json::Value) {
    if let Some(vnets) = response.get("result").and_then(|r| r.as_array()) {
        if vnets.is_empty() {
            output::info("No virtual networks found");
        } else {
            output::table_header(&["NAME", "ID", "DEFAULT", "COMMENT"]);
            for v in vnets {
                let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("-");
                let id = v.get("id").and_then(|i| i.as_str()).unwrap_or("-");
                let is_default = v
                    .get("is_default_network")
                    .and_then(|d| d.as_bool())
                    .unwrap_or(false);
                let comment = v.get("comment").and_then(|c| c.as_str()).unwrap_or("-");
                println!(
                    "{}\t{}\t{}\t{}",
                    name,
                    &id[..8],
                    if is_default { "✅" } else { "" },
                    comment
                );
            }
        }
    }
}

fn print_connectors(response: &serde_json::Value) {
    if let Some(connectors) = response.get("result").and_then(|r| r.as_array()) {
        if connectors.is_empty() {
            output::info("No WARP connectors found");
        } else {
            output::table_header(&["NAME", "ID", "STATUS"]);
            for c in connectors {
                let name = c.get("name").and_then(|n| n.as_str()).unwrap_or("-");
                let id = c.get("id").and_then(|i| i.as_str()).unwrap_or("-");
                let status = c.get("status").and_then(|s| s.as_str()).unwrap_or("-");
                println!("{}\t{}\t{}", name, &id[..8], status);
            }
        }
    }
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

async fn resolve_tunnel_id(
    client: &CloudflareClient,
    account_id: &str,
    tunnel: &str,
) -> Result<String> {
    // If it looks like a UUID, use it directly
    if tunnel.contains('-') && tunnel.len() > 30 {
        return Ok(tunnel.to_string());
    }

    // Otherwise, search by name
    let path = format!(
        "/accounts/{}/cfd_tunnel?name={}&is_deleted=false",
        account_id, tunnel
    );
    let response = client.get_raw(&path).await?;

    if let Some(tunnels) = response.get("result").and_then(|r| r.as_array()) {
        if let Some(t) = tunnels.first() {
            if let Some(id) = t.get("id").and_then(|i| i.as_str()) {
                return Ok(id.to_string());
            }
        }
    }

    Err(anyhow::anyhow!("Tunnel '{}' not found", tunnel))
}

fn get_cloudflared_path() -> Option<std::path::PathBuf> {
    // Check common locations
    let paths = [
        "/usr/local/bin/cloudflared",
        "/usr/bin/cloudflared",
        "/opt/homebrew/bin/cloudflared",
        &format!(
            "{}/.local/bin/cloudflared",
            std::env::var("HOME").unwrap_or_default()
        ),
    ];

    for p in paths {
        let path = std::path::PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }

    // Try PATH
    if let Ok(output) = std::process::Command::new("which")
        .arg("cloudflared")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Some(std::path::PathBuf::from(path_str));
            }
        }
    }

    None
}

async fn install_cloudflared() -> Result<()> {
    if get_cloudflared_path().is_some() {
        output::success("cloudflared is already installed!");
        show_client_status().await?;
        return Ok(());
    }

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    output::info(&format!("Installing cloudflared for {}/{}", os, arch));

    let (url, install_cmd) = match (os, arch) {
        ("macos", _) => {
            println!("Run: brew install cloudflared");
            println!("Or download from: https://github.com/cloudflare/cloudflared/releases");
            return Ok(());
        }
        ("linux", "x86_64") => (
            "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64",
            vec!["chmod", "+x", "cloudflared", "&&", "sudo", "mv", "cloudflared", "/usr/local/bin/"]
        ),
        ("linux", "aarch64") => (
            "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-arm64",
            vec!["chmod", "+x", "cloudflared", "&&", "sudo", "mv", "cloudflared", "/usr/local/bin/"]
        ),
        _ => {
            println!("Download from: https://github.com/cloudflare/cloudflared/releases");
            return Ok(());
        }
    };

    println!("\n📥 Download:");
    println!("curl -L {} -o cloudflared", url);
    println!("\n📦 Install:");
    println!("{}", install_cmd.join(" "));

    Ok(())
}

async fn run_tunnel(token: &str, background: bool) -> Result<()> {
    let cloudflared = get_cloudflared_path()
        .ok_or_else(|| anyhow::anyhow!("cloudflared not found. Run: cli5 tunnel install-client"))?;

    output::info(&format!("Starting tunnel with {}", cloudflared.display()));

    let mut cmd = std::process::Command::new(&cloudflared);
    cmd.args(["tunnel", "run", "--token", token]);

    if background {
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        let child = cmd.spawn()?;

        // Save PID for later
        let pid_file = get_pid_file();
        std::fs::write(&pid_file, child.id().to_string())?;

        output::success(&format!(
            "Tunnel started in background (PID: {})",
            child.id()
        ));
        println!("\nStop with: cli5 tunnel stop");
    } else {
        output::info("Running tunnel (Ctrl+C to stop)...");
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Tunnel exited with: {}", status));
        }
    }

    Ok(())
}

async fn stop_tunnel() -> Result<()> {
    let pid_file = get_pid_file();

    if pid_file.exists() {
        let pid_str = std::fs::read_to_string(&pid_file)?;
        let pid = pid_str.trim();

        // Kill process using kill command
        let _ = std::process::Command::new("kill").arg(pid).output();

        std::fs::remove_file(&pid_file)?;
        output::success(&format!("Tunnel stopped (PID: {})", pid));
    } else {
        // Try to find cloudflared processes
        let output = std::process::Command::new("pkill")
            .args(["-f", "cloudflared tunnel run"])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                output::success("Tunnel processes stopped");
            }
            _ => {
                output::info("No running tunnel found");
            }
        }
    }

    Ok(())
}

async fn show_client_status() -> Result<()> {
    println!("\n🔧 Cloudflared Client Status:\n");

    // Check if installed
    if let Some(path) = get_cloudflared_path() {
        println!("✅ Installed: {}", path.display());

        // Get version
        if let Ok(output) = std::process::Command::new(&path).arg("--version").output() {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("📦 Version: {}", version.trim());
        }
    } else {
        println!("❌ Not installed");
        println!("   Run: cli5 tunnel install-client");
        return Ok(());
    }

    // Check if running
    let pid_file = get_pid_file();
    if pid_file.exists() {
        let pid_str = std::fs::read_to_string(&pid_file)?;
        println!("🟢 Running (PID: {})", pid_str.trim());
    } else {
        // Check for any cloudflared processes
        if let Ok(output) = std::process::Command::new("pgrep")
            .args(["-f", "cloudflared tunnel"])
            .output()
        {
            if output.status.success() {
                let pids = String::from_utf8_lossy(&output.stdout);
                println!("🟢 Running (PIDs: {})", pids.trim().replace('\n', ", "));
            } else {
                println!("⚫ Not running");
            }
        }
    }

    Ok(())
}

fn get_pid_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home).join(".cloudflared.pid")
}

fn get_quick_pid_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home).join(".cloudflared-quick.pid")
}

fn get_quick_url_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home).join(".cloudflared-quick.url")
}

fn get_named_pid_file(name: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home).join(format!(".cloudflared-{}.pid", name))
}

fn get_named_url_file(name: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home).join(format!(".cloudflared-{}.url", name))
}

fn get_tunnel_config_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".cli5").join("tunnels")
}

fn get_tunnel_config_file(name: &str) -> std::path::PathBuf {
    get_tunnel_config_dir().join(format!("{}.json", name))
}

// ============ Quick Tunnel Implementation ============

async fn execute_quick(
    client: &CloudflareClient,
    account_id: &str,
    cmd: QuickCommand,
) -> Result<()> {
    match cmd {
        QuickCommand::Start {
            port,
            protocol,
            method,
            name,
            domain,
            background,
        } => {
            match method {
                TunnelMethod::Quick => quick_start_random(port, &protocol, background).await,
                TunnelMethod::Named => {
                    let name =
                        name.ok_or_else(|| anyhow::anyhow!("--name is required for named method"))?;
                    quick_start_named(
                        client,
                        account_id,
                        port,
                        &protocol,
                        &name,
                        domain.as_deref(),
                        background,
                    )
                    .await
                }
                TunnelMethod::Hybrid => {
                    let name = name
                        .ok_or_else(|| anyhow::anyhow!("--name is required for hybrid method"))?;
                    // Try named first, fall back to quick
                    match quick_start_named(
                        client,
                        account_id,
                        port,
                        &protocol,
                        &name,
                        domain.as_deref(),
                        background,
                    )
                    .await
                    {
                        Ok(()) => Ok(()),
                        Err(e) => {
                            output::warning(&format!(
                                "Named tunnel failed: {}. Falling back to quick tunnel...",
                                e
                            ));
                            quick_start_random(port, &protocol, background).await
                        }
                    }
                }
            }
        }
        QuickCommand::Stop { name } => quick_stop(name.as_deref()).await,
        QuickCommand::Status => quick_status().await,
        QuickCommand::Setup {
            name,
            domain,
            subdomain,
        } => quick_setup(client, account_id, &name, &domain, &subdomain).await,
        QuickCommand::List => quick_list(client, account_id).await,
    }
}

async fn quick_start_random(port: u16, protocol: &str, background: bool) -> Result<()> {
    // Check if cloudflared is installed
    let cloudflared = match get_cloudflared_path() {
        Some(p) => p,
        None => {
            output::info("cloudflared not found, downloading...");
            download_cloudflared().await?
        }
    };

    // Check if already running
    let pid_file = get_quick_pid_file();
    if pid_file.exists() {
        let pid_str = std::fs::read_to_string(&pid_file)?;
        output::info(&format!(
            "Quick tunnel already running (PID: {})",
            pid_str.trim()
        ));
        quick_status().await?;
        return Ok(());
    }

    // Build URL based on protocol
    let url = match protocol {
        "ssh" | "tcp" => format!("tcp://localhost:{}", port),
        "https" => format!("https://localhost:{}", port),
        _ => format!("http://localhost:{}", port),
    };

    output::info(&format!(
        "Starting quick tunnel: {} -> {}",
        url, "*.trycloudflare.com"
    ));

    if background {
        // Run in background and capture URL
        let log_file = get_quick_url_file().with_extension("log");

        let child = std::process::Command::new(&cloudflared)
            .args(["tunnel", "--url", &url])
            .stdout(std::fs::File::create(&log_file)?)
            .stderr(std::fs::File::create(&log_file)?)
            .spawn()?;

        // Save PID
        std::fs::write(&pid_file, child.id().to_string())?;

        output::success(&format!(
            "Quick tunnel started in background (PID: {})",
            child.id()
        ));
        println!();
        println!("⏳ Waiting for URL (checking log)...");

        // Wait a bit and try to get URL from log
        std::thread::sleep(std::time::Duration::from_secs(3));

        if let Ok(log_content) = std::fs::read_to_string(&log_file) {
            if let Some(url) = extract_tunnel_url(&log_content) {
                std::fs::write(get_quick_url_file(), &url)?;
                println!();
                println!("🔗 Tunnel URL: {}", url);
                println!();
                if protocol == "ssh" || protocol == "tcp" {
                    println!("📋 Connect with:");
                    println!("   ssh -o ProxyCommand=\"cloudflared access tcp --hostname {}\" user@localhost", 
                             url.replace("https://", ""));
                }
            } else {
                println!("📋 Check URL with: cli5 tunnel quick status");
            }
        }

        println!();
        println!("Stop with: cli5 tunnel quick stop");
    } else {
        // Run in foreground
        output::info("Running quick tunnel (Ctrl+C to stop)...");
        println!();
        println!("🔗 URL will appear below:");
        println!();

        let status = std::process::Command::new(&cloudflared)
            .args(["tunnel", "--url", &url])
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Tunnel exited with: {}", status));
        }
    }

    Ok(())
}

async fn quick_stop(name: Option<&str>) -> Result<()> {
    let pid_file = match name {
        Some(n) => get_named_pid_file(n),
        None => get_quick_pid_file(),
    };
    let url_file = match name {
        Some(n) => get_named_url_file(n),
        None => get_quick_url_file(),
    };

    if pid_file.exists() {
        let pid_str = std::fs::read_to_string(&pid_file)?;
        let pid = pid_str.trim();

        // Kill process
        #[cfg(unix)]
        {
            let _ = std::process::Command::new("kill").arg(pid).output();
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", pid, "/F"])
                .output();
        }

        std::fs::remove_file(&pid_file)?;
        let label = name.unwrap_or("Quick tunnel");
        output::success(&format!("{} stopped (PID: {})", label, pid));
    } else {
        // Try to find and kill any cloudflared quick tunnel
        #[cfg(unix)]
        {
            let _ = std::process::Command::new("pkill")
                .args(["-f", "cloudflared tunnel"])
                .output();
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/IM", "cloudflared.exe", "/F"])
                .output();
        }
        output::info("Stopped cloudflared processes");
    }

    // Cleanup URL file
    if url_file.exists() {
        std::fs::remove_file(&url_file)?;
    }

    // Cleanup log file
    let log_file = url_file.with_extension("log");
    if log_file.exists() {
        std::fs::remove_file(&log_file)?;
    }

    Ok(())
}

async fn quick_status() -> Result<()> {
    println!();
    println!("🚇 Quick Tunnel Status:");
    println!();

    let pid_file = get_quick_pid_file();
    let url_file = get_quick_url_file();

    if pid_file.exists() {
        let pid_str = std::fs::read_to_string(&pid_file)?;
        let pid = pid_str.trim();

        // Check if process is actually running
        #[cfg(unix)]
        let is_running = std::process::Command::new("kill")
            .args(["-0", pid])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        #[cfg(windows)]
        let is_running = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(pid))
            .unwrap_or(false);

        if is_running {
            println!("🟢 Status: Running (PID: {})", pid);

            // Try to get URL
            if url_file.exists() {
                if let Ok(url) = std::fs::read_to_string(&url_file) {
                    println!("🔗 URL: {}", url.trim());
                }
            } else {
                // Try from log file
                let log_file = url_file.with_extension("log");
                if log_file.exists() {
                    if let Ok(log) = std::fs::read_to_string(&log_file) {
                        if let Some(url) = extract_tunnel_url(&log) {
                            std::fs::write(&url_file, &url)?;
                            println!("🔗 URL: {}", url);
                        }
                    }
                }
            }
        } else {
            println!("⚫ Status: Not running (stale PID file)");
            std::fs::remove_file(&pid_file)?;
        }
    } else {
        println!("⚫ Status: Not running");
        println!();
        println!("Start with: cli5 tunnel quick start");
        println!("            cli5 tunnel quick start --port 22 --protocol ssh");
    }

    println!();
    Ok(())
}

fn extract_tunnel_url(log: &str) -> Option<String> {
    for line in log.lines() {
        if let Some(start) = line.find("https://") {
            let rest = &line[start..];
            if rest.contains(".trycloudflare.com") {
                if let Some(end) = rest.find(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
                    return Some(rest[..end].to_string());
                } else {
                    return Some(rest.trim().to_string());
                }
            }
        }
    }
    None
}

// ============ Named Tunnel Implementation ============

async fn quick_start_named(
    client: &CloudflareClient,
    account_id: &str,
    port: u16,
    protocol: &str,
    name: &str,
    domain: Option<&str>,
    background: bool,
) -> Result<()> {
    // Check if cloudflared is installed
    let cloudflared = match get_cloudflared_path() {
        Some(p) => p,
        None => {
            output::info("cloudflared not found, downloading...");
            download_cloudflared().await?
        }
    };

    // Check for saved tunnel config
    let config_file = get_tunnel_config_file(name);

    let (tunnel_id, tunnel_token, hostname) = if config_file.exists() {
        // Load from saved config
        let config_str = std::fs::read_to_string(&config_file)?;
        let config: serde_json::Value = serde_json::from_str(&config_str)?;

        let tunnel_id = config
            .get("tunnel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid config: missing tunnel_id"))?
            .to_string();

        let token = config
            .get("token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let hostname = config
            .get("hostname")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid config: missing hostname"))?
            .to_string();

        output::info(&format!("Using saved tunnel config: {}", name));
        (tunnel_id, token, hostname)
    } else {
        // Create new tunnel or use existing
        let domain = domain.ok_or_else(|| {
            anyhow::anyhow!(
                "--domain is required for first-time named tunnel.\n\
             Run: cli5 tunnel quick setup {} yourdomain.com",
                name
            )
        })?;

        // Check if tunnel exists
        let path = format!(
            "/accounts/{}/cfd_tunnel?name={}&is_deleted=false",
            account_id, name
        );
        let response = client.get_raw(&path).await?;

        let tunnel_id = if let Some(tunnels) = response.get("result").and_then(|r| r.as_array()) {
            if let Some(t) = tunnels.first() {
                t.get("id")
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                // Create tunnel
                let create_path = format!("/accounts/{}/cfd_tunnel", account_id);
                let secret = generate_tunnel_secret();
                let body = json!({
                    "name": name,
                    "tunnel_secret": secret,
                    "config_src": "cloudflare"
                });
                let create_response = client.post_raw(&create_path, body).await?;
                create_response
                    .get("result")
                    .and_then(|r| r.get("id"))
                    .and_then(|i| i.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Failed to create tunnel"))?
                    .to_string()
            }
        } else {
            return Err(anyhow::anyhow!("Failed to check existing tunnels"));
        };

        // Get tunnel token
        let token_path = format!("/accounts/{}/cfd_tunnel/{}/token", account_id, tunnel_id);
        let token_response = client.get_raw(&token_path).await?;
        let token = token_response
            .get("result")
            .and_then(|r| r.as_str())
            .unwrap_or("")
            .to_string();

        let hostname = format!("{}.{}", name, domain);

        // Save config for future use
        std::fs::create_dir_all(get_tunnel_config_dir())?;
        let config = json!({
            "name": name,
            "tunnel_id": tunnel_id,
            "token": token,
            "hostname": hostname,
            "domain": domain,
            "created": Utc::now().to_rfc3339()
        });
        std::fs::write(&config_file, serde_json::to_string_pretty(&config)?)?;

        output::success(&format!("Created named tunnel: {}", name));
        (tunnel_id, token, hostname)
    };

    // Build URL based on protocol
    let url = match protocol {
        "ssh" | "tcp" => format!("tcp://localhost:{}", port),
        "https" => format!("https://localhost:{}", port),
        _ => format!("http://localhost:{}", port),
    };

    // Check if already running
    let pid_file = get_named_pid_file(name);
    if pid_file.exists() {
        let pid_str = std::fs::read_to_string(&pid_file)?;
        output::info(&format!(
            "Tunnel '{}' already running (PID: {})",
            name,
            pid_str.trim()
        ));
        println!("🔗 URL: https://{}", hostname);
        return Ok(());
    }

    output::info(&format!(
        "Starting named tunnel: {} -> https://{}",
        url, hostname
    ));

    if background {
        let log_file = get_named_url_file(name).with_extension("log");

        // Run cloudflared with the tunnel token
        let child = if !tunnel_token.is_empty() {
            std::process::Command::new(&cloudflared)
                .args(["tunnel", "run", "--token", &tunnel_token])
                .stdout(std::fs::File::create(&log_file)?)
                .stderr(std::fs::File::create(&log_file)?)
                .spawn()?
        } else {
            // Fall back to quick tunnel mode with hostname
            std::process::Command::new(&cloudflared)
                .args(["tunnel", "--url", &url, "--hostname", &hostname])
                .stdout(std::fs::File::create(&log_file)?)
                .stderr(std::fs::File::create(&log_file)?)
                .spawn()?
        };

        std::fs::write(&pid_file, child.id().to_string())?;
        std::fs::write(get_named_url_file(name), format!("https://{}", hostname))?;

        output::success(&format!(
            "Named tunnel '{}' started (PID: {})",
            name,
            child.id()
        ));
        println!();
        println!("🔗 URL: https://{}", hostname);
        println!();

        if protocol == "ssh" || protocol == "tcp" {
            println!("📋 Connect with:");
            println!(
                "   ssh -o ProxyCommand=\"cloudflared access tcp --hostname {}\" user@localhost",
                hostname
            );
        }

        println!();
        println!("Stop with: cli5 tunnel quick stop --name {}", name);
    } else {
        output::info("Running named tunnel (Ctrl+C to stop)...");
        println!();
        println!("🔗 URL: https://{}", hostname);
        println!();

        let status = if !tunnel_token.is_empty() {
            std::process::Command::new(&cloudflared)
                .args(["tunnel", "run", "--token", &tunnel_token])
                .status()?
        } else {
            std::process::Command::new(&cloudflared)
                .args(["tunnel", "--url", &url])
                .status()?
        };

        if !status.success() {
            return Err(anyhow::anyhow!("Tunnel exited with: {}", status));
        }
    }

    // If first time, configure the hostname routing
    if !config_file.exists() {
        configure_tunnel_hostname(client, account_id, &tunnel_id, &hostname, port, protocol)
            .await?;
    }

    Ok(())
}

async fn configure_tunnel_hostname(
    client: &CloudflareClient,
    account_id: &str,
    tunnel_id: &str,
    hostname: &str,
    port: u16,
    protocol: &str,
) -> Result<()> {
    let service = match protocol {
        "ssh" | "tcp" => format!("tcp://localhost:{}", port),
        "https" => format!("https://localhost:{}", port),
        _ => format!("http://localhost:{}", port),
    };

    let path = format!(
        "/accounts/{}/cfd_tunnel/{}/configurations",
        account_id, tunnel_id
    );
    let body = json!({
        "config": {
            "ingress": [
                {
                    "hostname": hostname,
                    "service": service
                },
                {
                    "service": "http_status:404"
                }
            ]
        }
    });

    match client.put_raw(&path, body).await {
        Ok(_) => {
            output::success(&format!("Hostname {} configured for tunnel", hostname));
        }
        Err(e) => {
            output::warning(&format!(
                "Could not configure hostname: {}. You may need to configure it in the dashboard.",
                e
            ));
        }
    }

    Ok(())
}

async fn quick_setup(
    client: &CloudflareClient,
    account_id: &str,
    name: &str,
    domain: &str,
    subdomain: &str,
) -> Result<()> {
    output::info(&format!("Setting up named tunnel: {}", name));

    // 1. Create or get tunnel
    let path = format!(
        "/accounts/{}/cfd_tunnel?name={}&is_deleted=false",
        account_id, name
    );
    let response = client.get_raw(&path).await?;

    let tunnel_id = if let Some(tunnels) = response.get("result").and_then(|r| r.as_array()) {
        if let Some(t) = tunnels.first() {
            let id = t
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string();
            output::info(&format!("Using existing tunnel: {}", id));
            id
        } else {
            // Create tunnel
            let create_path = format!("/accounts/{}/cfd_tunnel", account_id);
            let secret = generate_tunnel_secret();
            let body = json!({
                "name": name,
                "tunnel_secret": secret,
                "config_src": "cloudflare"
            });
            let create_response = client.post_raw(&create_path, body).await?;
            let id = create_response
                .get("result")
                .and_then(|r| r.get("id"))
                .and_then(|i| i.as_str())
                .ok_or_else(|| anyhow::anyhow!("Failed to create tunnel"))?
                .to_string();
            output::success(&format!("Created tunnel: {}", id));
            id
        }
    } else {
        return Err(anyhow::anyhow!("Failed to query tunnels"));
    };

    // 2. Get tunnel token
    let token_path = format!("/accounts/{}/cfd_tunnel/{}/token", account_id, tunnel_id);
    let token_response = client.get_raw(&token_path).await?;
    let token = token_response
        .get("result")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .to_string();

    // 3. Build hostname pattern
    let hostname = format!("*.{}.{}", subdomain, domain);

    // 4. Save config
    std::fs::create_dir_all(get_tunnel_config_dir())?;
    let config = json!({
        "name": name,
        "tunnel_id": tunnel_id,
        "token": token,
        "hostname": hostname,
        "hostname_pattern": format!("<name>.{}.{}", subdomain, domain),
        "domain": domain,
        "subdomain": subdomain,
        "created": Utc::now().to_rfc3339()
    });

    let config_file = get_tunnel_config_file(name);
    std::fs::write(&config_file, serde_json::to_string_pretty(&config)?)?;

    println!();
    output::success("Named tunnel setup complete!");
    println!();
    println!("📋 Configuration:");
    println!("   Tunnel:    {}", name);
    println!("   Tunnel ID: {}", tunnel_id);
    println!("   Pattern:   <name>.{}.{}", subdomain, domain);
    println!();
    println!("📋 Usage:");
    println!("   cli5 tunnel quick start --method named --name my-pc --port 22 --protocol ssh");
    println!();
    println!("📋 Result URL:");
    println!("   https://my-pc.{}.{}", subdomain, domain);
    println!();
    println!(
        "⚠️  Note: You need to add a DNS record for *.{}.{} pointing to your tunnel",
        subdomain, domain
    );
    println!(
        "   Or configure it in Cloudflare Dashboard → Tunnels → {} → Public Hostnames",
        name
    );

    Ok(())
}

async fn quick_list(client: &CloudflareClient, account_id: &str) -> Result<()> {
    println!();
    println!("📋 Configured Named Tunnels:");
    println!();

    let config_dir = get_tunnel_config_dir();

    if !config_dir.exists() {
        println!("   No named tunnels configured.");
        println!();
        println!("   Setup with: cli5 tunnel quick setup <name> <domain>");
        return Ok(());
    }

    let entries = std::fs::read_dir(&config_dir)?;
    let mut found = false;

    output::table_header(&["NAME", "HOSTNAME", "TUNNEL ID", "RUNNING"]);

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                    let name = config.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                    let hostname = config
                        .get("hostname")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");
                    let tunnel_id = config
                        .get("tunnel_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");

                    // Check if running
                    let pid_file = get_named_pid_file(name);
                    let running = if pid_file.exists() { "🟢" } else { "⚫" };

                    println!(
                        "{}\t{}\t{}\t{}",
                        name,
                        hostname,
                        &tunnel_id[..8.min(tunnel_id.len())],
                        running
                    );
                    found = true;
                }
            }
        }
    }

    if !found {
        println!("   No named tunnels configured.");
    }

    println!();

    // Also show API tunnels
    println!("📋 Cloudflare Tunnels (API):");
    println!();

    let path = format!("/accounts/{}/cfd_tunnel?is_deleted=false", account_id);
    let response = client.get_raw(&path).await?;
    print_tunnels(&response);

    Ok(())
}

async fn download_cloudflared() -> Result<std::path::PathBuf> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let url = match (os, arch) {
        ("macos", "aarch64") => "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-darwin-arm64.tgz",
        ("macos", "x86_64") => "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-darwin-amd64.tgz",
        ("linux", "x86_64") => "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64",
        ("linux", "aarch64") => "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-arm64",
        ("windows", _) => "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe",
        _ => return Err(anyhow::anyhow!("Unsupported platform: {}/{}", os, arch)),
    };

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let bin_dir = std::path::PathBuf::from(&home).join(".local").join("bin");
    std::fs::create_dir_all(&bin_dir)?;

    let dest = if os == "windows" {
        bin_dir.join("cloudflared.exe")
    } else {
        bin_dir.join("cloudflared")
    };

    output::info(&format!("Downloading cloudflared to {}", dest.display()));

    // Use curl for download (available on all platforms)
    let status = std::process::Command::new("curl")
        .args(["-L", "-o", dest.to_str().unwrap(), url])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to download cloudflared"));
    }

    // Handle macOS tgz
    if os == "macos" && url.ends_with(".tgz") {
        let tgz_path = dest.with_extension("tgz");
        std::fs::rename(&dest, &tgz_path)?;

        std::process::Command::new("tar")
            .args([
                "-xzf",
                tgz_path.to_str().unwrap(),
                "-C",
                bin_dir.to_str().unwrap(),
            ])
            .status()?;

        std::fs::remove_file(&tgz_path)?;
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)?;
    }

    output::success("cloudflared installed!");

    Ok(dest)
}
