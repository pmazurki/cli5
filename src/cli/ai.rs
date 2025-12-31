//! AI command - Cloudflare Workers AI

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::CloudflareClient;
use crate::config::Config;
use crate::output;

const DEFAULT_MODEL: &str = "@cf/meta/llama-3.2-1b-instruct";

#[derive(Args, Debug)]
pub struct AiArgs {
    /// Model to use
    #[arg(short, long, default_value = DEFAULT_MODEL)]
    pub model: String,

    #[command(subcommand)]
    pub command: AiCommand,
}

#[derive(Subcommand, Debug)]
pub enum AiCommand {
    /// Chat with AI
    Chat {
        /// Your message/prompt
        prompt: String,

        /// System prompt (optional)
        #[arg(short, long)]
        system: Option<String>,
    },

    /// List available AI models
    Models,

    /// Generate text completion
    Complete {
        /// Text to complete
        prompt: String,

        /// Max tokens to generate
        #[arg(short, long, default_value = "256")]
        max_tokens: u32,
    },

    /// Summarize text
    Summarize {
        /// Text to summarize
        text: String,
    },

    /// Translate text
    Translate {
        /// Text to translate
        text: String,

        /// Target language
        #[arg(short, long, default_value = "English")]
        to: String,
    },
}

pub async fn execute(config: &Config, args: AiArgs) -> Result<()> {
    let client = CloudflareClient::new(config.clone())?;

    // Get account ID from zone
    let account_id = get_account_id(&client).await?;

    match args.command {
        AiCommand::Chat { prompt, system } => {
            let mut messages = vec![];

            if let Some(sys) = system {
                messages.push(json!({"role": "system", "content": sys}));
            }

            messages.push(json!({"role": "user", "content": prompt}));

            let body = json!({ "messages": messages });

            let path = format!("/accounts/{}/ai/run/{}", account_id, args.model);
            let response = client.post_raw(&path, body).await?;

            if let Some(result) = response.get("result") {
                if let Some(text) = result.get("response").and_then(|r| r.as_str()) {
                    println!("{}", text);
                }

                // Show usage
                if let Some(usage) = result.get("usage") {
                    let total = usage
                        .get("total_tokens")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0);
                    output::info(&format!("Tokens used: {}", total));
                }
            }
        }

        AiCommand::Models => {
            output::info("Popular Workers AI models (Free tier):");
            println!();
            println!("Chat/LLM:");
            println!("  @cf/meta/llama-3.2-1b-instruct     (fast, small)");
            println!("  @cf/meta/llama-3.2-3b-instruct     (balanced)");
            println!("  @cf/mistral/mistral-7b-instruct-v0.1");
            println!();
            println!("Text:");
            println!("  @cf/facebook/bart-large-cnn        (summarization)");
            println!("  @cf/huggingface/distilbert-sst-2-int8 (sentiment)");
            println!();
            println!("Speech:");
            println!("  @cf/openai/whisper                 (transcription)");
            println!();
            println!("Image:");
            println!("  @cf/stabilityai/stable-diffusion-xl-base-1.0");
            println!();
            println!("Embeddings:");
            println!("  @cf/baai/bge-base-en-v1.5");
            println!("  @cf/baai/bge-small-en-v1.5");
        }

        AiCommand::Complete { prompt, max_tokens } => {
            let body = json!({
                "prompt": prompt,
                "max_tokens": max_tokens
            });

            let path = format!("/accounts/{}/ai/run/{}", account_id, args.model);
            let response = client.post_raw(&path, body).await?;

            if let Some(result) = response.get("result") {
                if let Some(text) = result.get("response").and_then(|r| r.as_str()) {
                    println!("{}", text);
                }
            }
        }

        AiCommand::Summarize { text } => {
            let messages = vec![
                json!({"role": "system", "content": "You are a helpful assistant that summarizes text concisely."}),
                json!({"role": "user", "content": format!("Summarize the following text:\n\n{}", text)}),
            ];

            let body = json!({ "messages": messages });
            let path = format!("/accounts/{}/ai/run/{}", account_id, args.model);
            let response = client.post_raw(&path, body).await?;

            if let Some(result) = response.get("result") {
                if let Some(text) = result.get("response").and_then(|r| r.as_str()) {
                    println!("{}", text);
                }
            }
        }

        AiCommand::Translate { text, to } => {
            let messages = vec![
                json!({"role": "system", "content": format!("You are a translator. Translate the text to {}. Only output the translation, nothing else.", to)}),
                json!({"role": "user", "content": text}),
            ];

            let body = json!({ "messages": messages });
            let path = format!("/accounts/{}/ai/run/{}", account_id, args.model);
            let response = client.post_raw(&path, body).await?;

            if let Some(result) = response.get("result") {
                if let Some(text) = result.get("response").and_then(|r| r.as_str()) {
                    println!("{}", text);
                }
            }
        }
    }

    Ok(())
}

async fn get_account_id(client: &CloudflareClient) -> Result<String> {
    // Try to get account ID from first zone
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

    Err(anyhow::anyhow!(
        "Could not determine account ID. Make sure you have at least one zone."
    ))
}
