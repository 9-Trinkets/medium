use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tauri_app_lib::config;

pub fn run(name: &str, path: Option<&str>) -> Result<()> {
    // If no path provided, use the configured ghosts directory
    let path = if let Some(p) = path {
        p.to_string()
    } else {
        let ghosts_path = config::ghosts_dir()?;
        fs::create_dir_all(&ghosts_path)
            .with_context(|| format!("Failed to create ghosts directory: {}", ghosts_path.display()))?;
        ghosts_path.join(name).to_string_lossy().to_string()
    };

    let base_path = Path::new(&path);

    // Create the base directory
    fs::create_dir_all(base_path)
        .with_context(|| format!("Failed to create directory: {}", base_path.display()))?;

    // Create the resources/animations directory
    let animations_dir = base_path.join("resources").join("animations");
    fs::create_dir_all(&animations_dir)
        .with_context(|| format!("Failed to create animations directory: {}", animations_dir.display()))?;

    // Create a minimal ghost.toml
    let manifest_content = format!(
        r#"[ghost]
name = "{}"
description = "A custom ghost created with `medium ghosts scaffold`."

[sprite]
enabled = true
frame_width = 192
frame_height = 192
fps = 8

[[sprite.animations]]
file = "resources/animations/idle.png"
name = "idle"
intent = "Standing still, ready to help."
"#,
        name
    );

    let manifest_path = base_path.join("ghost.toml");
    fs::write(&manifest_path, manifest_content)
        .with_context(|| format!("Failed to write {}", manifest_path.display()))?;

    // Create a placeholder idle animation (1x1 transparent PNG)
    let placeholder_png = create_placeholder_png();
    let idle_path = animations_dir.join("idle.png");
    fs::write(&idle_path, placeholder_png)
        .with_context(|| format!("Failed to write {}", idle_path.display()))?;

    println!("✅ Ghost scaffold created at: {}", base_path.display());
    println!("  📋 ghost.toml");
    println!("  🎬 resources/animations/idle.png (placeholder)");
    println!("\nNext steps:");
    println!("  1. Edit ghost.toml to customize your ghost");
    println!("  2. Replace the placeholder idle.png with your animation");
    println!("  3. Run: medium ghosts preview {}", path);

    Ok(())
}

/// Create a minimal 1x1 transparent PNG
fn create_placeholder_png() -> Vec<u8> {
    // PNG header + IHDR chunk (1x1 RGBA) + IDAT chunk (empty) + IEND chunk
    vec![
        // PNG signature
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
        // IHDR chunk (13 bytes data)
        0x00, 0x00, 0x00, 0x0D, // chunk length
        0x49, 0x48, 0x44, 0x52, // "IHDR"
        0x00, 0x00, 0x00, 0x01, // width: 1
        0x00, 0x00, 0x00, 0x01, // height: 1
        0x08, 0x06, 0x00, 0x00, 0x00, // bit depth 8, color type 6 (RGBA), compression/filter/interlace
        0x1F, 0x15, 0xC4, 0x89, // CRC
        // IDAT chunk (minimal)
        0x00, 0x00, 0x00, 0x0A, // chunk length
        0x49, 0x44, 0x41, 0x54, // "IDAT"
        0x78, 0x9C, 0x62, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00,
        0x18, 0xDD, 0x8D, 0xB4, // CRC
        // IEND chunk
        0x00, 0x00, 0x00, 0x00, // chunk length
        0x49, 0x45, 0x4E, 0x44, // "IEND"
        0xAE, 0x42, 0x60, 0x82, // CRC
    ]
}
