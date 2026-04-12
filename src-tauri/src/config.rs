use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const MEDIUM_CONFIG_ENV: &str = "MEDIUM_CONFIG";
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GlobalConfig {
    pub tts: Option<GlobalTtsConfig>,
    pub ghosts: Option<GhostsConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GhostsConfig {
    pub path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GlobalTtsConfig {
    pub provider: Option<String>,
    pub openai_api_key: Option<String>,
    pub elevenlabs_api_key: Option<String>,
    pub elevenlabs_voice_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedConfigPath {
    pub path: PathBuf,
    pub source: &'static str,
}

impl GlobalTtsConfig {
    pub fn has_openai_api_key(&self) -> bool {
        has_non_empty_value(&self.openai_api_key)
    }

    pub fn has_elevenlabs_api_key(&self) -> bool {
        has_non_empty_value(&self.elevenlabs_api_key)
    }

    pub fn provider_name(&self) -> Option<&str> {
        self.provider
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }
}

pub fn home_dir() -> Result<PathBuf> {
    let home = env::var("HOME").context("Could not find HOME directory")?;
    Ok(PathBuf::from(home))
}

pub fn medium_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".medium"))
}

pub fn default_config_path() -> Result<PathBuf> {
    Ok(medium_dir()?.join("config.toml"))
}

pub fn resolve_config_path() -> Result<ResolvedConfigPath> {
    if let Ok(config_path) = env::var(MEDIUM_CONFIG_ENV) {
        let trimmed = config_path.trim();
        if !trimmed.is_empty() {
            return Ok(ResolvedConfigPath {
                path: PathBuf::from(trimmed),
                source: MEDIUM_CONFIG_ENV,
            });
        }
    }

    Ok(ResolvedConfigPath {
        path: default_config_path()?,
        source: "default",
    })
}

pub fn load_global_config() -> Result<Option<GlobalConfig>> {
    let resolved = resolve_config_path()?;
    load_global_config_from_path(&resolved.path)
}

pub fn load_global_config_from_path(path: &Path) -> Result<Option<GlobalConfig>> {
    if !path.exists() {
        return Ok(None);
    }

    let config_content =
        fs::read_to_string(path).with_context(|| format!("Failed to read config at {:?}", path))?;
    let config = toml::from_str::<GlobalConfig>(&config_content)
        .with_context(|| format!("Failed to parse config at {:?}", path))?;
    Ok(Some(config))
}

pub fn log_file_path() -> Result<PathBuf> {
    Ok(medium_dir()?.join("daemon.log"))
}

pub fn ghosts_dir() -> Result<PathBuf> {
    // Check config for custom ghosts path
    if let Ok(Some(config)) = load_global_config() {
        if let Some(ghosts_config) = config.ghosts {
            if let Some(path) = ghosts_config.path {
                let path_trimmed = path.trim();
                if !path_trimmed.is_empty() {
                    return Ok(PathBuf::from(path_trimmed));
                }
            }
        }
    }

    // Default to ~/.medium/ghosts
    Ok(medium_dir()?.join("ghosts"))
}

pub fn global_claude_mcp_path() -> Result<PathBuf> {
    Ok(home_dir()?.join(".claude").join(".mcp.json"))
}

pub fn find_nearest_project_mcp_path(start_dir: &Path) -> Option<PathBuf> {
    start_dir
        .ancestors()
        .map(|dir| dir.join(".mcp.json"))
        .find(|path| path.exists())
}

fn has_non_empty_value(value: &Option<String>) -> bool {
    value
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn resolves_env_override_when_present() -> Result<()> {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempdir()?;
        let target = tmp.path().join("medium.toml");
        std::env::set_var(MEDIUM_CONFIG_ENV, &target);

        let resolved = resolve_config_path()?;
        assert_eq!(resolved.path, target);
        assert_eq!(resolved.source, MEDIUM_CONFIG_ENV);

        std::env::remove_var(MEDIUM_CONFIG_ENV);
        Ok(())
    }

    #[test]
    fn parses_tts_keys_from_config() -> Result<()> {
        let tmp = tempdir()?;
        let path = tmp.path().join("config.toml");
        fs::write(
            &path,
            r#"[tts]
provider = "openai"
openai_api_key = "test-key"
"#,
        )?;

        let config = load_global_config_from_path(&path)?.unwrap();
        let tts = config.tts.unwrap();
        assert_eq!(tts.provider_name(), Some("openai"));
        assert!(tts.has_openai_api_key());
        assert!(!tts.has_elevenlabs_api_key());
        Ok(())
    }

}
