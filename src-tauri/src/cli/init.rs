use anyhow::Result;

use super::integrate::{self, IntegrationScope, IntegrationTool};
use tauri_app_lib::config::{ensure_default_config_exists, resolve_config_path};

pub fn run() -> Result<()> {
    let config_path = ensure_default_config_exists()?;
    let cwd = std::env::current_dir()?;
    let project_path =
        integrate::run(IntegrationTool::Claude, IntegrationScope::Project, None, &cwd)?;
    let global_hint = resolve_config_path()?;

    println!("✅ Medium config ready at {:?}", config_path);
    println!("✅ Project-local Claude MCP configured at {:?}", project_path);
    println!(
        "\nConfiguration complete. Use 'medium integrate claude --global' or 'medium integrate copilot' for additional targets."
    );
    println!("Current config source: {} ({})", global_hint.path.display(), global_hint.source);

    Ok(())
}
