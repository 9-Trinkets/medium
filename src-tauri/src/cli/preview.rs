use anyhow::{Context, Result};
use std::path::Path;
use tauri_app_lib::manifest::GhostManifest;

pub fn run(path: &str) -> Result<()> {
    let custom_ghost_path = Path::new(path)
        .canonicalize()
        .with_context(|| format!("Ghost folder not found: {}", path))?;

    // Validate the manifest first
    println!("Loading ghost from: {}", custom_ghost_path.display());
    let manifest = GhostManifest::load_and_validate(&custom_ghost_path)
        .map_err(|error| anyhow::anyhow!("Validation failed:\n{}", error))?;

    println!("✅ Ghost manifest is valid!");
    println!("  Name: {}", manifest.ghost.name);
    println!("  Animations: {}", manifest.sprite.animations.len());

    let ghost_name = manifest.ghost.name.clone();

    // Pass the custom ghost path to the daemon via environment variable
    std::env::set_var(
        "MEDIUM_PREVIEW_GHOST_PATH",
        custom_ghost_path.to_string_lossy().to_string(),
    );

    println!("Starting Medium daemon with '{}' ghost...", ghost_name);
    println!("(Press Ctrl+C to stop)\n");

    tauri_app_lib::run(ghost_name, "preview".to_string());

    Ok(())
}
