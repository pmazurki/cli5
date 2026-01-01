# cli5

Modern Cloudflare CLI written in Rust. Supports both REST API and GraphQL Analytics API.

## Features

- 🚀 **Fast** - Native Rust binary, async I/O
- 🔐 **Secure** - API Token or Global API Key authentication
- 📊 **Analytics** - Full GraphQL Analytics API support
- 🔒 **SSL/TLS** - Full SSL management with security recommendations
- 👷 **Workers** - Create, deploy and manage Cloudflare Workers
- 📄 **Pages** - Manage Cloudflare Pages projects
- 🤖 **AI** - Chat with Cloudflare Workers AI (Llama, Mistral)
- 💾 **Storage** - KV, D1, Queues, Vectorize, Hyperdrive, R2
- 🚇 **Tunnels** - Create, manage and run Cloudflare Tunnels
- 🎨 **Colored output** - Beautiful terminal formatting
- 📦 **Modular** - Endpoints defined in JSON files
- 🌍 **Cross-platform** - Linux, macOS, Windows (x86_64 & ARM64)

## Installation

### From GitHub Releases

Download the latest release for your platform:

**macOS:**
```bash
# macOS ARM64 (Apple Silicon)
curl -L https://github.com/pmazurki/cli5/releases/latest/download/cli5-macos-arm64 -o cli5
chmod +x cli5
sudo mv cli5 /usr/local/bin/

# macOS x86_64 (Intel)
curl -L https://github.com/pmazurki/cli5/releases/latest/download/cli5-macos-x86_64 -o cli5
chmod +x cli5
sudo mv cli5 /usr/local/bin/
```

**Linux:**
```bash
# Linux x86_64
curl -L https://github.com/pmazurki/cli5/releases/latest/download/cli5-linux-x86_64 -o cli5
chmod +x cli5
sudo mv cli5 /usr/local/bin/

# Linux ARM64
curl -L https://github.com/pmazurki/cli5/releases/latest/download/cli5-linux-arm64 -o cli5
chmod +x cli5
sudo mv cli5 /usr/local/bin/
```

**Windows:**
```powershell
# PowerShell - Automatyczna instalacja
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/pmazurki/cli5/main/install-windows.ps1" -OutFile install-windows.ps1
.\install-windows.ps1

# Lub ręcznie:
# 1. Pobierz cli5-windows-x86_64.exe z Releases
# 2. Zmień nazwę na cli5.exe
# 3. Dodaj do PATH
```

📖 **Szczegółowe instrukcje Windows:** [WINDOWS.md](WINDOWS.md)

### From Source

```bash
git clone https://github.com/pmazurki/cli5.git
cd cli5
cargo build --release
cp target/release/cli5 ~/.local/bin/
```

## Configuration

Create a `.env` file or set environment variables:

```bash
# API Token (recommended)
export CF_API_TOKEN="your_api_token"

# Or Global API Key (legacy)
export CF_API_EMAIL="your@email.com"
export CF_API_KEY="your_global_api_key"

# Optional defaults
export CF_ZONE_ID="your_default_zone_id"
export CF_ZONE_NAME="example.com"
export CF_OUTPUT_FORMAT="table"  # json, table, compact
```

Create an API Token at: https://dash.cloudflare.com/profile/api-tokens

## Usage

### Zones

```bash
cli5 zones list
cli5 zones get example.com
cli5 zones id example.com
```

### DNS Records

```bash
cli5 dns list --zone example.com
cli5 dns list --zone example.com --type A
cli5 dns add www A 1.2.3.4 --zone example.com
cli5 dns add api CNAME api.backend.com --zone example.com --proxied false
cli5 dns update RECORD_ID --content 5.6.7.8 --zone example.com
cli5 dns delete RECORD_ID -y --zone example.com
cli5 dns export --zone example.com > dns_backup.json
```

### Settings

```bash
cli5 settings list --zone example.com
cli5 settings ssl strict --zone example.com
cli5 settings https on --zone example.com
cli5 settings security high --zone example.com
cli5 settings minify --css true --js true --zone example.com
```

### Firewall

```bash
cli5 firewall list --zone example.com
cli5 firewall block-ip 1.2.3.4 --note "Spam" --zone example.com
cli5 firewall block-country RU --note "Block Russia" --zone example.com
cli5 firewall whitelist-ip 5.6.7.8 --note "Office" --zone example.com
cli5 firewall delete RULE_ID --zone example.com
```

### Cache

```bash
cli5 cache purge-all -y --zone example.com
cli5 cache purge-urls https://example.com/style.css,https://example.com/app.js --zone example.com
cli5 cache purge-tags static,images --zone example.com  # Enterprise
```

### SSL/TLS

```bash
# Show status with security recommendations
cli5 ssl status --zone example.com

# Configure SSL
cli5 ssl mode strict --zone example.com          # off, flexible, full, strict
cli5 ssl min-tls 1.2 --zone example.com          # 1.0, 1.1, 1.2, 1.3
cli5 ssl tls13 on --zone example.com             # Enable TLS 1.3
cli5 ssl always-https on --zone example.com      # Force HTTPS

# View certificates
cli5 ssl certs --zone example.com
```

### Workers

```bash
# List workers
cli5 workers list

# Create a simple worker
cli5 workers create hello-api --message "Hello from my API!"

# Manage routes
cli5 workers routes --zone example.com
cli5 workers add-route --zone example.com --pattern "api.example.com/*" --script hello-api

# Delete worker
cli5 workers delete hello-api

# KV namespaces
cli5 workers kv
```

### Pages

```bash
# List Pages projects
cli5 pages list

# Create project
cli5 pages create my-site --branch main

# View project info
cli5 pages info my-site

# List deployments
cli5 pages deployments my-site

# Delete project
cli5 pages delete my-site
```

### Workers AI

```bash
# Chat with AI
cli5 ai chat "What is Cloudflare?"
cli5 ai chat "Explain DNS" --model @cf/meta/llama-3.2-3b-instruct

# Translate text
cli5 ai translate "Hello world" --to Polish

# Summarize text
cli5 ai summarize "Long text to summarize..."

# List available models
cli5 ai models
```

### Storage & Databases

```bash
# Workers KV - Key-Value storage
cli5 storage kv list
cli5 storage kv create my-kv
cli5 storage kv keys NAMESPACE_ID
cli5 storage kv put NAMESPACE_ID mykey "myvalue"
cli5 storage kv get NAMESPACE_ID mykey
cli5 storage kv delete NAMESPACE_ID

# D1 - SQLite at the edge
cli5 storage d1 list
cli5 storage d1 create my-database
cli5 storage d1 query DATABASE_ID "SELECT * FROM users"
cli5 storage d1 query DATABASE_ID "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)"
cli5 storage d1 delete DATABASE_ID

# Queues (requires paid plan)
cli5 storage queues list
cli5 storage queues create my-queue
cli5 storage queues delete QUEUE_ID

# Vectorize - Vector database for AI
cli5 storage vectorize list
cli5 storage vectorize create my-index --dimensions 768 --metric cosine
cli5 storage vectorize delete my-index

# Hyperdrive - Database connection pooling
cli5 storage hyperdrive list
cli5 storage hyperdrive create my-config --connection-string "postgres://user:pass@host/db"
cli5 storage hyperdrive delete CONFIG_ID

# R2 - Object storage (requires dashboard activation)
cli5 storage r2 list
cli5 storage r2 create my-bucket
cli5 storage r2 delete my-bucket
```

### Tunnels

```bash
# Check cloudflared status
cli5 tunnel status

# Install cloudflared client
cli5 tunnel install-client

# List tunnels
cli5 tunnel list

# Create tunnel
cli5 tunnel create my-tunnel

# Get token
cli5 tunnel token TUNNEL_ID

# Run tunnel (foreground)
cli5 tunnel run TUNNEL_ID

# Run tunnel (background)
cli5 tunnel run TUNNEL_ID --background

# Stop tunnel
cli5 tunnel stop

# Routes management
cli5 tunnel routes
cli5 tunnel add-route 192.168.1.0/24 --tunnel TUNNEL_ID --comment "Home LAN"

# Virtual networks
cli5 tunnel vnets
cli5 tunnel create-vnet my-vnet --default

# Delete tunnel
cli5 tunnel delete TUNNEL_ID
```

### Analytics (GraphQL)

```bash
# Top statistics (Free: 6h max, Pro+: 7d+)
cli5 analytics top-urls --zone example.com --since 6h --limit 20
cli5 analytics top-ips --zone example.com --since 6h
cli5 analytics top-countries --zone example.com --since 6h

# Errors and performance
cli5 analytics errors --zone example.com --since 6h
cli5 analytics cache --zone example.com --since 6h
cli5 analytics bandwidth --zone example.com --since 6h
cli5 analytics hourly --zone example.com --since 6h
cli5 analytics bots --zone example.com --since 6h

# Firewall events (Pro+ only)
cli5 analytics firewall --zone example.com --since 1h

# Custom GraphQL query
cli5 analytics query "{ viewer { zones(filter: {zoneTag: \"ZONE_ID\"}) { ... } } }"
```

### Raw API

```bash
cli5 raw /user
cli5 raw /zones
cli5 raw /zones/:zone_id/dns_records --zone example.com
cli5 raw /zones/:zone_id/settings/ssl --zone example.com -m PATCH -b '{"value":"strict"}'
```

### Configuration

```bash
cli5 config show
cli5 config test
cli5 config paths
cli5 config endpoints
cli5 config endpoints --category dns
```

## Output Formats

```bash
cli5 zones list                    # Table format (default)
cli5 zones list --format json      # JSON format
cli5 zones list --format compact   # Compact colored format
```

## Adding Custom Endpoints

Create JSON files in `~/.config/cli5/endpoints/` or `./endpoints/`:

```json
{
  "name": "custom",
  "description": "My custom endpoints",
  "version": "v4",
  "endpoints": [
    {
      "name": "my_endpoint",
      "method": "GET",
      "path": "/zones/{zone_id}/custom",
      "description": "My custom endpoint",
      "category": "custom",
      "params": [
        {
          "name": "zone_id",
          "type": "string",
          "required": true,
          "location": "path"
        }
      ]
    }
  ]
}
```

## License

MIT

## Author

Pawel Mazz <pmazurki@me.com>

