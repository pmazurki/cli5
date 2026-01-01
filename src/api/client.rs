//! HTTP client for Cloudflare API

use anyhow::{anyhow, Result};
use reqwest::{Client, Method, RequestBuilder};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use tracing::{debug, trace};

use crate::api::response::ApiResponse;
use crate::config::Config;

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";
const CF_GRAPHQL_URL: &str = "https://api.cloudflare.com/client/v4/graphql";

/// Cloudflare API client
pub struct CloudflareClient {
    client: Client,
    config: Config,
}

impl CloudflareClient {
    /// Create a new Cloudflare API client
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder().user_agent("cli5/0.1.0").build()?;

        Ok(Self { client, config })
    }

    /// Build request with authentication headers
    fn build_request(&self, method: Method, url: &str) -> RequestBuilder {
        let mut req = self.client.request(method, url);

        for (key, value) in self.config.auth_headers() {
            req = req.header(key, value);
        }

        req = req.header("Content-Type", "application/json");

        req
    }

    /// Make a GET request to the API
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<ApiResponse<T>> {
        let url = format!("{}{}", CF_API_BASE, path);
        debug!("GET {}", url);

        let response = self.build_request(Method::GET, &url).send().await?;

        let status = response.status();
        let text = response.text().await?;
        trace!("Response: {}", text);

        if !status.is_success() {
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let api_response: ApiResponse<T> = serde_json::from_str(&text)?;

        if !api_response.success {
            let errors: Vec<String> = api_response
                .errors
                .iter()
                .map(|e| format!("{}: {}", e.code, e.message))
                .collect();
            return Err(anyhow!("API errors: {}", errors.join(", ")));
        }

        Ok(api_response)
    }

    /// Make a POST request to the API (typed response)
    #[allow(dead_code)]
    pub async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Value,
    ) -> Result<ApiResponse<T>> {
        let url = format!("{}{}", CF_API_BASE, path);
        debug!("POST {} with body: {}", url, body);

        let response = self
            .build_request(Method::POST, &url)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        trace!("Response: {}", text);

        if !status.is_success() {
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let api_response: ApiResponse<T> = serde_json::from_str(&text)?;

        if !api_response.success {
            let errors: Vec<String> = api_response
                .errors
                .iter()
                .map(|e| format!("{}: {}", e.code, e.message))
                .collect();
            return Err(anyhow!("API errors: {}", errors.join(", ")));
        }

        Ok(api_response)
    }

    /// Make a PATCH request to the API (typed response)
    #[allow(dead_code)]
    pub async fn patch<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Value,
    ) -> Result<ApiResponse<T>> {
        let url = format!("{}{}", CF_API_BASE, path);
        debug!("PATCH {} with body: {}", url, body);

        let response = self
            .build_request(Method::PATCH, &url)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        trace!("Response: {}", text);

        if !status.is_success() {
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let api_response: ApiResponse<T> = serde_json::from_str(&text)?;

        if !api_response.success {
            let errors: Vec<String> = api_response
                .errors
                .iter()
                .map(|e| format!("{}: {}", e.code, e.message))
                .collect();
            return Err(anyhow!("API errors: {}", errors.join(", ")));
        }

        Ok(api_response)
    }

    /// Make a PUT request to the API (typed response)
    #[allow(dead_code)]
    pub async fn put<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Value,
    ) -> Result<ApiResponse<T>> {
        let url = format!("{}{}", CF_API_BASE, path);
        debug!("PUT {} with body: {}", url, body);

        let response = self
            .build_request(Method::PUT, &url)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        trace!("Response: {}", text);

        if !status.is_success() {
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let api_response: ApiResponse<T> = serde_json::from_str(&text)?;

        if !api_response.success {
            let errors: Vec<String> = api_response
                .errors
                .iter()
                .map(|e| format!("{}: {}", e.code, e.message))
                .collect();
            return Err(anyhow!("API errors: {}", errors.join(", ")));
        }

        Ok(api_response)
    }

    /// Make a DELETE request to the API (typed response)
    #[allow(dead_code)]
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<ApiResponse<T>> {
        let url = format!("{}{}", CF_API_BASE, path);
        debug!("DELETE {}", url);

        let response = self.build_request(Method::DELETE, &url).send().await?;

        let status = response.status();
        let text = response.text().await?;
        trace!("Response: {}", text);

        if !status.is_success() {
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let api_response: ApiResponse<T> = serde_json::from_str(&text)?;

        if !api_response.success {
            let errors: Vec<String> = api_response
                .errors
                .iter()
                .map(|e| format!("{}: {}", e.code, e.message))
                .collect();
            return Err(anyhow!("API errors: {}", errors.join(", ")));
        }

        Ok(api_response)
    }

    /// Make a raw GET request (returns Value)
    pub async fn get_raw(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", CF_API_BASE, path);
        debug!("GET (raw) {}", url);

        let response = self.build_request(Method::GET, &url).send().await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let value: Value = serde_json::from_str(&text)?;
        Ok(value)
    }

    /// Make a raw POST request (returns Value)
    pub async fn post_raw(&self, path: &str, body: Value) -> Result<Value> {
        let url = format!("{}{}", CF_API_BASE, path);
        debug!("POST (raw) {} with body: {}", url, body);

        let response = self
            .build_request(Method::POST, &url)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let value: Value = serde_json::from_str(&text)?;
        Ok(value)
    }

    /// Make a raw PATCH request (returns Value)
    pub async fn patch_raw(&self, path: &str, body: Value) -> Result<Value> {
        let url = format!("{}{}", CF_API_BASE, path);
        debug!("PATCH (raw) {} with body: {}", url, body);

        let response = self
            .build_request(Method::PATCH, &url)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let value: Value = serde_json::from_str(&text)?;
        Ok(value)
    }

    /// Make a raw PUT request (returns Value)
    pub async fn put_raw(&self, path: &str, body: Value) -> Result<Value> {
        let url = format!("{}{}", CF_API_BASE, path);
        debug!("PUT (raw) {} with body: {}", url, body);

        let response = self
            .build_request(Method::PUT, &url)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let value: Value = serde_json::from_str(&text)?;
        Ok(value)
    }

    /// Make a raw DELETE request (returns Value)
    pub async fn delete_raw(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", CF_API_BASE, path);
        debug!("DELETE (raw) {}", url);

        let response = self.build_request(Method::DELETE, &url).send().await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let value: Value = serde_json::from_str(&text)?;
        Ok(value)
    }

    /// Upload a Worker script (ES modules format)
    pub async fn put_worker_script(
        &self,
        path: &str,
        script: &str,
        es_modules: bool,
    ) -> Result<Value> {
        let url = format!("{}{}", CF_API_BASE, path);
        debug!("PUT worker script to {}", url);

        let mut req = self.client.request(Method::PUT, &url);

        // Add auth headers
        for (key, value) in self.config.auth_headers() {
            req = req.header(key, value);
        }

        if es_modules {
            // ES modules format requires multipart
            use reqwest::multipart::{Form, Part};

            let metadata = json!({
                "main_module": "worker.js",
                "compatibility_date": "2024-01-01"
            });

            let form = Form::new()
                .part(
                    "metadata",
                    Part::text(metadata.to_string()).mime_str("application/json")?,
                )
                .part(
                    "worker.js",
                    Part::text(script.to_string()).mime_str("application/javascript+module")?,
                );

            req = req.multipart(form);
        } else {
            // Service worker format (legacy)
            req = req
                .header("Content-Type", "application/javascript")
                .body(script.to_string());
        }

        let response = req.send().await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let value: Value = serde_json::from_str(&text)?;
        Ok(value)
    }

    /// Execute a GraphQL query
    pub async fn graphql(&self, query: &str, variables: Option<Value>) -> Result<Value> {
        debug!("GraphQL query: {}", query);

        let body = json!({
            "query": query,
            "variables": variables.unwrap_or(json!({}))
        });

        let response = self
            .build_request(Method::POST, CF_GRAPHQL_URL)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        trace!("GraphQL Response: {}", text);

        if !status.is_success() {
            return Err(anyhow!("GraphQL error ({}): {}", status, text));
        }

        let value: Value = serde_json::from_str(&text)?;

        // Check for GraphQL errors
        if let Some(errors) = value.get("errors") {
            if let Some(arr) = errors.as_array() {
                if !arr.is_empty() {
                    let error_msgs: Vec<String> = arr
                        .iter()
                        .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                        .map(|s| s.to_string())
                        .collect();
                    return Err(anyhow!("GraphQL errors: {}", error_msgs.join(", ")));
                }
            }
        }

        Ok(value)
    }

    /// Get zone ID by name
    pub async fn get_zone_id(&self, name: &str) -> Result<String> {
        let response: ApiResponse<Vec<Value>> = self.get(&format!("/zones?name={}", name)).await?;

        if let Some(zones) = response.result {
            if let Some(zone) = zones.first() {
                if let Some(id) = zone.get("id").and_then(|v| v.as_str()) {
                    return Ok(id.to_string());
                }
            }
        }

        Err(anyhow!("Zone not found: {}", name))
    }

    /// Resolve zone - return ID if looks like ID, otherwise lookup by name
    pub async fn resolve_zone_id(&self, zone: &str) -> Result<String> {
        // Check if it looks like a zone ID (32 hex chars)
        if zone.len() == 32 && zone.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(zone.to_string());
        }

        // Otherwise lookup by name
        self.get_zone_id(zone).await
    }
}
