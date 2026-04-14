use anyhow::{Context, Result};
use std::fs;
use toml::map::Map;
use toml::Value;

use tauri_app_lib::config::{ensure_default_config_exists, resolve_config_path};

#[derive(clap::Subcommand)]
pub enum ConfigCommand {
    /// Print the path to Medium's config file
    Path,
    /// Get a config value using dot notation (for example: integration.default_ghost)
    Get {
        #[arg()]
        key: Option<String>,
    },
    /// Set a config value using dot notation (for example: integration.default_ghost vita)
    Set {
        #[arg()]
        key: String,
        #[arg()]
        value: String,
    },
}

pub fn run(command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Path => {
            let resolved = resolve_config_path()?;
            println!("{}", resolved.path.display());
            Ok(())
        }
        ConfigCommand::Get { key } => {
            let resolved = resolve_config_path()?;
            let root = load_config_value()?;
            if let Some(key) = key {
                let value = get_value(&root, &key)
                    .ok_or_else(|| anyhow::anyhow!("Config key not found: {}", key))?;
                println!("{}", render_value(value)?);
            } else if resolved.path.exists() {
                let content = fs::read_to_string(&resolved.path)
                    .with_context(|| format!("Failed to read {}", resolved.path.display()))?;
                print!("{content}");
            } else {
                println!("{}", toml::to_string_pretty(&root)?);
            }
            Ok(())
        }
        ConfigCommand::Set { key, value } => {
            let path = ensure_default_config_exists()?;
            let mut root = load_config_value()?;
            set_value(&mut root, &key, parse_scalar(&value));
            fs::write(&path, toml::to_string_pretty(&root)?)
                .with_context(|| format!("Failed to write {}", path.display()))?;
            println!("✅ Updated {} in {}", key, path.display());
            Ok(())
        }
    }
}

fn load_config_value() -> Result<Value> {
    let resolved = resolve_config_path()?;
    if !resolved.path.exists() {
        return Ok(Value::Table(Map::new()));
    }

    let content = fs::read_to_string(&resolved.path)
        .with_context(|| format!("Failed to read {}", resolved.path.display()))?;
    Ok(toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", resolved.path.display()))?)
}

fn parse_scalar(input: &str) -> Value {
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("true") {
        Value::Boolean(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        Value::Boolean(false)
    } else if let Ok(int_value) = trimmed.parse::<i64>() {
        Value::Integer(int_value)
    } else if let Ok(float_value) = trimmed.parse::<f64>() {
        Value::Float(float_value)
    } else if trimmed.starts_with('{') || trimmed.starts_with('[') {
        serde_json::from_str::<serde_json::Value>(trimmed)
            .ok()
            .and_then(|json_value| toml::Value::try_from(json_value).ok())
            .unwrap_or_else(|| Value::String(input.to_string()))
    } else {
        Value::String(input.to_string())
    }
}

fn get_value<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn set_value(root: &mut Value, path: &str, value: Value) {
    let parts: Vec<_> = path.split('.').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return;
    }

    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        if !current.is_table() {
            *current = Value::Table(Map::new());
        }

        let table = current.as_table_mut().expect("table");
        current = table
            .entry((*part).to_string())
            .or_insert_with(|| Value::Table(Map::new()));
    }

    if !current.is_table() {
        *current = Value::Table(Map::new());
    }
    current
        .as_table_mut()
        .expect("table")
        .insert(parts[parts.len() - 1].to_string(), value);
}

fn render_value(value: &Value) -> Result<String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Integer(i) => Ok(i.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Boolean(b) => Ok(b.to_string()),
        Value::Datetime(dt) => Ok(dt.to_string()),
        Value::Array(_) | Value::Table(_) => Ok(toml::to_string_pretty(value)?),
    }
}

#[cfg(test)]
mod tests {
    use super::{get_value, parse_scalar, set_value};
    use toml::Value;

    #[test]
    fn parses_basic_scalars() {
        assert_eq!(parse_scalar("true"), Value::Boolean(true));
        assert_eq!(parse_scalar("42"), Value::Integer(42));
        assert_eq!(parse_scalar("vita"), Value::String("vita".to_string()));
    }

    #[test]
    fn sets_nested_values() {
        let mut value = Value::Table(Default::default());
        set_value(
            &mut value,
            "integration.default_ghost",
            Value::String("vita".to_string()),
        );
        assert_eq!(
            get_value(&value, "integration.default_ghost"),
            Some(&Value::String("vita".to_string()))
        );
    }
}
