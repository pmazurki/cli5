//! Cloudflare Tunnel commands - secure connections to your infrastructure

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct TunnelArgs {
    #[command(subcommand)]
    pub command: TunnelCommand,
}

#[derive(Subcommand, Debug)]
pub enum TunnelCommand {
    /// List all tunnels
    List,

    /// Create a new tunnel
    Create {
        /// Tunnel name
        name: String,
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
    let client = CloudflareClient::new(config.clone())?;
    let account_id = get_account_id(&client).await?;

    match args.command {
        TunnelCommand::List => {
            let path = format!("/accounts/{}/cfd_tunnel?is_deleted=false", account_id);
            let response = client.get_raw(&path).await?;
            print_tunnels(&response);
        }

        TunnelCommand::Create { name } => {
            let path = format!("/accounts/{}/cfd_tunnel", account_id);
            // Generate a random tunnel secret
            let secret = generate_tunnel_secret();
            let body = json!({
                "name": name,
                "tunnel_secret": secret,
                "config_src": "cloudflare"
            });
            let response = client.post_raw(&path, body).await?;
            if let Some(result) = response.get("result") {
                let id = result.get("id").and_then(|i| i.as_str()).unwrap_or("-");
                output::success(&format!("Tunnel '{}' created!", name));
                println!("\nTunnel ID: {}", id);
                println!("\n📋 Next steps:");
                println!("1. Get token:    cli5 tunnel token {}", id);
                println!("2. Run tunnel:   cloudflared tunnel run --token <TOKEN>");
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

        TunnelCommand::InstallClient => {
            install_cloudflared().await?;
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

        TunnelCommand::Stop { tunnel: _ } => {
            stop_tunnel().await?;
        }

        TunnelCommand::Status => {
            show_client_status().await?;
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

// ============ Helpers ============

fn generate_tunnel_secret() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // Base64 encoded 32-byte secret
    let secret_bytes: Vec<u8> = (0..32)
        .map(|i| ((timestamp >> (i % 16)) & 0xFF) as u8 ^ (i as u8 * 17))
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
