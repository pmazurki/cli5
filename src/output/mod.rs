//! Output formatting module

use anyhow::Result;
use owo_colors::OwoColorize;
use serde::Serialize;
use serde_json::Value;

use crate::config::OutputFormat;

/// Print output in the configured format
pub fn print_output<T: Serialize>(data: &T, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => print_json(data),
        OutputFormat::Compact => print_compact(data),
        OutputFormat::Table => print_json_pretty(data), // Default to pretty JSON for now
    }
}

/// Print as formatted JSON
pub fn print_json<T: Serialize>(data: &T) -> Result<()> {
    println!("{}", serde_json::to_string(data)?);
    Ok(())
}

/// Print as pretty JSON
pub fn print_json_pretty<T: Serialize>(data: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(data)?);
    Ok(())
}

/// Print compact output
pub fn print_compact<T: Serialize>(data: &T) -> Result<()> {
    let value = serde_json::to_value(data)?;
    print_value_compact(&value, 0);
    Ok(())
}

fn print_value_compact(value: &Value, indent: usize) {
    let prefix = "  ".repeat(indent);

    match value {
        Value::Object(map) => {
            for (key, val) in map {
                match val {
                    Value::Object(_) | Value::Array(_) => {
                        println!("{}{}: ", prefix, key.cyan());
                        print_value_compact(val, indent + 1);
                    }
                    _ => {
                        println!("{}{}: {}", prefix, key.cyan(), format_value(val));
                    }
                }
            }
        }
        Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                println!("{}[{}]", prefix, i.to_string().dimmed());
                print_value_compact(val, indent + 1);
            }
        }
        _ => {
            println!("{}{}", prefix, format_value(value));
        }
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "null".dimmed().to_string(),
        Value::Bool(b) => {
            if *b {
                "true".green().to_string()
            } else {
                "false".red().to_string()
            }
        }
        Value::Number(n) => n.to_string().yellow().to_string(),
        Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

/// Print success message
pub fn success(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg);
}

/// Print error message
pub fn error(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), msg);
}

/// Print warning message
pub fn warning(msg: &str) {
    eprintln!("{} {}", "⚠".yellow().bold(), msg);
}

/// Print info message
pub fn info(msg: &str) {
    println!("{} {}", "ℹ".blue().bold(), msg);
}

/// Print a table header
pub fn table_header(columns: &[&str]) {
    let header: Vec<String> = columns
        .iter()
        .map(|c| c.bold().underline().to_string())
        .collect();
    println!("{}", header.join("\t"));
}

/// Print DNS record in table format
pub fn print_dns_record(record: &Value) {
    let id = record.get("id").and_then(|v| v.as_str()).unwrap_or("-");
    let rtype = record.get("type").and_then(|v| v.as_str()).unwrap_or("-");
    let name = record.get("name").and_then(|v| v.as_str()).unwrap_or("-");
    let content = record
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let proxied = record
        .get("proxied")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let ttl = record.get("ttl").and_then(|v| v.as_u64()).unwrap_or(0);

    let proxied_str = if proxied {
        "●".bright_yellow().to_string()
    } else {
        "○".dimmed().to_string()
    };

    let ttl_str = if ttl == 1 {
        "Auto".to_string()
    } else {
        format!("{}s", ttl)
    };

    println!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        rtype.cyan(),
        name.bold(),
        content,
        proxied_str,
        ttl_str.dimmed(),
        id.dimmed()
    );
}

/// Print zone in table format
pub fn print_zone(zone: &Value) {
    let id = zone.get("id").and_then(|v| v.as_str()).unwrap_or("-");
    let name = zone.get("name").and_then(|v| v.as_str()).unwrap_or("-");
    let status = zone.get("status").and_then(|v| v.as_str()).unwrap_or("-");
    let plan = zone
        .get("plan")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("-");

    let status_colored = match status {
        "active" => status.green().to_string(),
        "pending" => status.yellow().to_string(),
        "moved" => status.red().to_string(),
        _ => status.to_string(),
    };

    println!(
        "{}\t{}\t{}\t{}",
        name.bold(),
        status_colored,
        plan.cyan(),
        id.dimmed()
    );
}

/// Print firewall rule in table format
pub fn print_firewall_rule(rule: &Value) {
    let id = rule.get("id").and_then(|v| v.as_str()).unwrap_or("-");
    let mode = rule
        .get("mode")
        .or_else(|| rule.get("action"))
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let notes = rule.get("notes").and_then(|v| v.as_str()).unwrap_or("");
    let config = rule.get("configuration").unwrap_or(&Value::Null);
    let target = config.get("target").and_then(|v| v.as_str()).unwrap_or("-");
    let value = config.get("value").and_then(|v| v.as_str()).unwrap_or("-");

    let mode_colored = match mode {
        "block" => mode.red().to_string(),
        "challenge" | "js_challenge" => mode.yellow().to_string(),
        "whitelist" | "allow" => mode.green().to_string(),
        _ => mode.to_string(),
    };

    println!(
        "{}\t{}\t{}\t{}\t{}",
        mode_colored,
        target.cyan(),
        value,
        notes.dimmed(),
        id.dimmed()
    );
}

/// Print analytics result
pub fn print_analytics_row(count: u64, dimensions: &Value) {
    let dims: Vec<String> = dimensions
        .as_object()
        .map(|obj| {
            obj.values()
                .filter_map(|v| v.as_str().or_else(|| v.as_i64().map(|_| "")))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    println!("{}\t{}", count.to_string().yellow().bold(), dims.join("\t"));
}
