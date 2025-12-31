//! Settings command

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

#[derive(Args, Debug)]
pub struct SettingsArgs {
    /// Zone name or ID
    #[arg(short, long)]
    pub zone: Option<String>,

    #[command(subcommand)]
    pub command: SettingsCommand,
}

#[derive(Subcommand, Debug)]
pub enum SettingsCommand {
    /// List all zone settings
    List,

    /// Get a specific setting
    Get {
        /// Setting name (e.g., "ssl", "always_use_https")
        name: String,
    },

    /// Set SSL mode
    Ssl {
        /// SSL mode: off, flexible, full, strict
        mode: String,
    },

    /// Enable/disable Always Use HTTPS
    Https {
        /// on or off
        value: String,
    },

    /// Set security level
    Security {
        /// Level: off, essentially_off, low, medium, high, under_attack
        level: String,
    },

    /// Set cache level
    CacheLevel {
        /// Level: bypass, basic, simplified, aggressive
        level: String,
    },

    /// Set browser cache TTL
    BrowserCacheTtl {
        /// TTL in seconds (0 = respect origin)
        seconds: u32,
    },

    /// Enable/disable minification
    Minify {
        /// Enable CSS minification
        #[arg(long)]
        css: Option<bool>,

        /// Enable HTML minification
        #[arg(long)]
        html: Option<bool>,

        /// Enable JS minification
        #[arg(long)]
        js: Option<bool>,
    },

    /// Set a custom setting value
    Set {
        /// Setting name
        name: String,

        /// Setting value (as JSON)
        value: String,
    },
}

pub async fn execute(config: &Config, args: SettingsArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;
    let zone = config.resolve_zone(args.zone.as_deref())?;
    let zone_id = client.resolve_zone_id(&zone).await?;

    match args.command {
        SettingsCommand::List => {
            let response = client
                .get_raw(&format!("/zones/{}/settings", zone_id))
                .await?;

            if let Some(settings) = response.get("result").and_then(|r| r.as_array()) {
                for setting in settings {
                    let id = setting.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                    let value = setting.get("value");
                    let editable = setting
                        .get("editable")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let editable_str = if editable { "" } else { " (read-only)" };

                    println!(
                        "{}: {}{}",
                        id,
                        serde_json::to_string(value.unwrap_or(&json!(null)))?,
                        editable_str
                    );
                }
            }
        }

        SettingsCommand::Get { name } => {
            let response = client
                .get_raw(&format!("/zones/{}/settings/{}", zone_id, name))
                .await?;
            output::print_output(&response.get("result"), &config.output_format)?;
        }

        SettingsCommand::Ssl { mode } => {
            let valid = ["off", "flexible", "full", "strict"];
            if !valid.contains(&mode.as_str()) {
                output::error(&format!(
                    "Invalid SSL mode. Valid options: {}",
                    valid.join(", ")
                ));
                return Ok(());
            }

            let body = json!({"value": mode});
            client
                .patch_raw(&format!("/zones/{}/settings/ssl", zone_id), body)
                .await?;
            output::success(&format!("SSL mode set to: {}", mode));
        }

        SettingsCommand::Https { value } => {
            let val = match value.to_lowercase().as_str() {
                "on" | "true" | "1" => "on",
                "off" | "false" | "0" => "off",
                _ => {
                    output::error("Invalid value. Use 'on' or 'off'");
                    return Ok(());
                }
            };

            let body = json!({"value": val});
            client
                .patch_raw(
                    &format!("/zones/{}/settings/always_use_https", zone_id),
                    body,
                )
                .await?;
            output::success(&format!("Always Use HTTPS: {}", val));
        }

        SettingsCommand::Security { level } => {
            let valid = [
                "off",
                "essentially_off",
                "low",
                "medium",
                "high",
                "under_attack",
            ];
            if !valid.contains(&level.as_str()) {
                output::error(&format!(
                    "Invalid security level. Valid options: {}",
                    valid.join(", ")
                ));
                return Ok(());
            }

            let body = json!({"value": level});
            client
                .patch_raw(&format!("/zones/{}/settings/security_level", zone_id), body)
                .await?;
            output::success(&format!("Security level set to: {}", level));
        }

        SettingsCommand::CacheLevel { level } => {
            let valid = ["bypass", "basic", "simplified", "aggressive"];
            if !valid.contains(&level.as_str()) {
                output::error(&format!(
                    "Invalid cache level. Valid options: {}",
                    valid.join(", ")
                ));
                return Ok(());
            }

            let body = json!({"value": level});
            client
                .patch_raw(&format!("/zones/{}/settings/cache_level", zone_id), body)
                .await?;
            output::success(&format!("Cache level set to: {}", level));
        }

        SettingsCommand::BrowserCacheTtl { seconds } => {
            let body = json!({"value": seconds});
            client
                .patch_raw(
                    &format!("/zones/{}/settings/browser_cache_ttl", zone_id),
                    body,
                )
                .await?;
            output::success(&format!("Browser cache TTL set to: {} seconds", seconds));
        }

        SettingsCommand::Minify { css, html, js } => {
            // Get current minify settings
            let current = client
                .get_raw(&format!("/zones/{}/settings/minify", zone_id))
                .await?;
            let current_value = current
                .get("result")
                .and_then(|r| r.get("value"))
                .cloned()
                .unwrap_or(json!({"css": "off", "html": "off", "js": "off"}));

            let new_value = json!({
                "css": css.map(|b| if b { "on" } else { "off" }).unwrap_or_else(||
                    current_value.get("css").and_then(|v| v.as_str()).unwrap_or("off")
                ),
                "html": html.map(|b| if b { "on" } else { "off" }).unwrap_or_else(||
                    current_value.get("html").and_then(|v| v.as_str()).unwrap_or("off")
                ),
                "js": js.map(|b| if b { "on" } else { "off" }).unwrap_or_else(||
                    current_value.get("js").and_then(|v| v.as_str()).unwrap_or("off")
                ),
            });

            let body = json!({"value": new_value});
            client
                .patch_raw(&format!("/zones/{}/settings/minify", zone_id), body)
                .await?;
            output::success(&format!("Minification updated: {}", new_value));
        }

        SettingsCommand::Set { name, value } => {
            let parsed_value: serde_json::Value =
                serde_json::from_str(&value).unwrap_or_else(|_| json!(value));

            let body = json!({"value": parsed_value});
            client
                .patch_raw(&format!("/zones/{}/settings/{}", zone_id, name), body)
                .await?;
            output::success(&format!("Setting {} updated", name));
        }
    }

    Ok(())
}

// Add patch_raw helper
impl CloudflareClient {
    pub async fn patch_raw(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, anyhow::Error> {
        use crate::api::response::ApiResponse;
        let response: ApiResponse<serde_json::Value> = self.patch(path, body).await?;
        Ok(serde_json::json!({"result": response.result}))
    }
}
