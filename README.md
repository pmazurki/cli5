# cli5

Modern Cloudflare CLI written in Rust. Supports both REST API and GraphQL Analytics API.

## Features

- 🚀 **Fast** - Native Rust binary, async I/O
- 🔐 **Secure** - API Token or Global API Key authentication
- 📊 **Analytics** - Full GraphQL Analytics API support (Pro+)
- 🎨 **Colored output** - Beautiful terminal formatting
- 📦 **Modular** - Endpoints defined in JSON files
- 🌍 **Cross-platform** - Linux, macOS (x86_64 & ARM64)

## Installation

### From GitHub Releases

Download the latest release for your platform:

```bash
# macOS ARM64 (Apple Silicon)
curl -L https://github.com/YOUR_USERNAME/cli5/releases/latest/download/cli5-macos-arm64 -o cli5
chmod +x cli5
sudo mv cli5 /usr/local/bin/

# macOS x86_64 (Intel)
curl -L https://github.com/YOUR_USERNAME/cli5/releases/latest/download/cli5-macos-x86_64 -o cli5

# Linux x86_64
curl -L https://github.com/YOUR_USERNAME/cli5/releases/latest/download/cli5-linux-x86_64 -o cli5

# Linux ARM64
curl -L https://github.com/YOUR_USERNAME/cli5/releases/latest/download/cli5-linux-arm64 -o cli5
```

### From Source

```bash
git clone https://github.com/YOUR_USERNAME/cli5.git
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

### Analytics (GraphQL - Pro+)

```bash
# Top statistics
cli5 analytics top-urls --zone example.com --since 24h --limit 20
cli5 analytics top-ips --zone example.com --since 1h
cli5 analytics top-countries --zone example.com --since 7d

# Errors and security
cli5 analytics errors --zone example.com --since 24h
cli5 analytics firewall --zone example.com --since 1h

# Performance
cli5 analytics cache --zone example.com --since 24h
cli5 analytics bandwidth --zone example.com --since 7d
cli5 analytics hourly --zone example.com --since 7d

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

