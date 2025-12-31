//! Dynamic endpoint registry loaded from JSON files

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::Config;

/// HTTP method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// Endpoint parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointParam {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Parameter type: string, number, boolean, array, object
    #[serde(rename = "type")]
    pub param_type: String,
    /// Is this parameter required?
    #[serde(default)]
    pub required: bool,
    /// Default value
    pub default: Option<serde_json::Value>,
    /// Location: path, query, body
    #[serde(default = "default_location")]
    pub location: String,
}

fn default_location() -> String {
    "body".to_string()
}

/// Endpoint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    /// Endpoint name/identifier
    pub name: String,
    /// HTTP method
    pub method: HttpMethod,
    /// API path (can contain {placeholders})
    pub path: String,
    /// Description
    pub description: String,
    /// Parameters
    #[serde(default)]
    pub params: Vec<EndpointParam>,
    /// Category (dns, firewall, cache, etc.)
    #[serde(default)]
    pub category: String,
    /// Required plan (free, pro, business, enterprise)
    #[serde(default)]
    pub required_plan: Option<String>,
    /// Example usage
    #[serde(default)]
    pub examples: Vec<String>,
}

/// Endpoint group from JSON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointGroup {
    /// Group name
    pub name: String,
    /// Group description
    pub description: String,
    /// API version
    #[serde(default = "default_version")]
    pub version: String,
    /// Endpoints in this group
    pub endpoints: Vec<Endpoint>,
}

fn default_version() -> String {
    "v4".to_string()
}

/// Registry of all loaded endpoints
#[derive(Debug, Default)]
pub struct EndpointRegistry {
    /// All loaded endpoints by name
    pub endpoints: HashMap<String, Endpoint>,
    /// Endpoints grouped by category
    pub by_category: HashMap<String, Vec<String>>,
}

impl EndpointRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Load all endpoint definitions from a directory
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let mut registry = Self::new();

        if !dir.exists() {
            return Ok(registry);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                match registry.load_file(&path) {
                    Ok(_) => tracing::debug!("Loaded endpoints from {:?}", path),
                    Err(e) => tracing::warn!("Failed to load {:?}: {}", path, e),
                }
            }
        }

        Ok(registry)
    }

    /// Load endpoints from a single JSON file
    pub fn load_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let group: EndpointGroup = serde_json::from_str(&content)?;

        for endpoint in group.endpoints {
            let category = if endpoint.category.is_empty() {
                group.name.clone()
            } else {
                endpoint.category.clone()
            };

            // Add to category index
            self.by_category
                .entry(category)
                .or_default()
                .push(endpoint.name.clone());

            // Add to main registry
            self.endpoints.insert(endpoint.name.clone(), endpoint);
        }

        Ok(())
    }

    /// Get endpoint by name
    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&Endpoint> {
        self.endpoints.get(name)
    }

    /// List all endpoint names
    #[allow(dead_code)]
    pub fn list(&self) -> Vec<&str> {
        self.endpoints.keys().map(|s| s.as_str()).collect()
    }

    /// List endpoints by category
    pub fn list_by_category(&self, category: &str) -> Vec<&Endpoint> {
        self.by_category
            .get(category)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.endpoints.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// List all categories
    pub fn categories(&self) -> Vec<&str> {
        self.by_category.keys().map(|s| s.as_str()).collect()
    }

    /// Build path with parameters substituted
    #[allow(dead_code)]
    pub fn build_path(endpoint: &Endpoint, params: &HashMap<String, String>) -> Result<String> {
        let mut path = endpoint.path.clone();

        // Substitute path parameters
        for param in &endpoint.params {
            if param.location == "path" {
                let placeholder = format!("{{{}}}", param.name);
                if let Some(value) = params.get(&param.name) {
                    path = path.replace(&placeholder, value);
                } else if param.required {
                    return Err(anyhow!("Missing required path parameter: {}", param.name));
                }
            }
        }

        // Add query parameters
        let query_params: Vec<String> = endpoint
            .params
            .iter()
            .filter(|p| p.location == "query")
            .filter_map(|p| params.get(&p.name).map(|v| format!("{}={}", p.name, v)))
            .collect();

        if !query_params.is_empty() {
            path = format!("{}?{}", path, query_params.join("&"));
        }

        Ok(path)
    }
}

/// Load the default endpoint registry
pub fn load_registry() -> Result<EndpointRegistry> {
    let endpoints_dir = Config::endpoints_dir()?;
    EndpointRegistry::load_from_dir(&endpoints_dir)
}
