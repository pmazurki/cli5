//! Configuration management

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// API Token (preferred authentication method)
    pub api_token: Option<String>,

    /// Global API Key (legacy)
    pub api_key: Option<String>,

    /// Email for Global API Key auth
    pub api_email: Option<String>,

    /// Default zone ID
    pub zone_id: Option<String>,

    /// Default zone name
    pub zone_name: Option<String>,

    /// Output format
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Compact,
}

impl Config {
    /// Load configuration from environment variables
    pub fn load() -> Result<Self> {
        let config = Self::load_optional();

        // Require at least one auth method
        if config.api_token.is_none() && (config.api_key.is_none() || config.api_email.is_none()) {
            return Err(anyhow!(
                "Authentication required. Set CF_API_TOKEN or both CF_API_KEY and CF_API_EMAIL"
            ));
        }

        Ok(config)
    }

    /// Load configuration without requiring authentication
    /// Useful for commands that can work with just a tunnel token
    pub fn load_optional() -> Self {
        let api_token = env::var("CF_API_TOKEN").ok();
        let api_key = env::var("CF_API_KEY").ok();
        let api_email = env::var("CF_API_EMAIL").ok();

        let output_format = match env::var("CF_OUTPUT_FORMAT")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "json" => OutputFormat::Json,
            "compact" => OutputFormat::Compact,
            _ => OutputFormat::Table,
        };

        Self {
            api_token,
            api_key,
            api_email,
            zone_id: env::var("CF_ZONE_ID").ok(),
            zone_name: env::var("CF_ZONE_NAME").ok(),
            output_format,
        }
    }

    /// Get the authentication headers for API requests
    pub fn auth_headers(&self) -> Vec<(&'static str, String)> {
        if let Some(ref token) = self.api_token {
            vec![("Authorization", format!("Bearer {}", token))]
        } else if let (Some(ref key), Some(ref email)) = (&self.api_key, &self.api_email) {
            vec![("X-Auth-Key", key.clone()), ("X-Auth-Email", email.clone())]
        } else {
            vec![]
        }
    }

    /// Get config directory path
    pub fn config_dir() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("Cannot determine config directory"))?
            .join("cli5");

        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }

        Ok(dir)
    }

    /// Get endpoints directory path
    pub fn endpoints_dir() -> Result<PathBuf> {
        // First check local directory
        let local = PathBuf::from("endpoints");
        if local.exists() {
            return Ok(local);
        }

        // Then check config directory
        let config_dir = Self::config_dir()?;
        let endpoints_dir = config_dir.join("endpoints");

        if !endpoints_dir.exists() {
            std::fs::create_dir_all(&endpoints_dir)?;
        }

        Ok(endpoints_dir)
    }

    /// Resolve zone ID from name or use provided ID
    pub fn resolve_zone(&self, zone: Option<&str>) -> Result<String> {
        if let Some(z) = zone {
            // If it looks like a zone ID (32 hex chars), use it directly
            if z.len() == 32 && z.chars().all(|c| c.is_ascii_hexdigit()) {
                return Ok(z.to_string());
            }
            // Otherwise treat as zone name (will be resolved via API)
            return Ok(z.to_string());
        }

        // Try default zone ID
        if let Some(ref id) = self.zone_id {
            return Ok(id.clone());
        }

        // Try default zone name
        if let Some(ref name) = self.zone_name {
            return Ok(name.clone());
        }

        Err(anyhow!(
            "No zone specified. Use --zone or set CF_ZONE_ID/CF_ZONE_NAME"
        ))
    }
}
