use anyhow::Result;
use std::path::Path;
use tauri_app_lib::manifest::GhostManifest;

pub async fn run(path: &str) -> Result<()> {
    let base_path = Path::new(path);
    println!("Validating ghost at: {}", base_path.display());

    let manifest = GhostManifest::load_and_validate(base_path)
        .map_err(|error| anyhow::anyhow!("Validation failed:\n{}", error))?;

    println!("✅ Manifest is valid!");
    println!("  Name: {}", manifest.ghost.name);
    if let Some(provenance) = manifest.provenance.as_ref() {
        if let Some(source) = provenance.source.as_deref() {
            println!("  Provenance: {} ({})", source, provenance.source_type);
        } else {
            println!("  Provenance Type: {}", provenance.source_type);
        }
        if let Some(artist) = provenance.artist.as_deref() {
            println!("  Artist: {}", artist);
        }
    }
    println!("  Animations: {}", manifest.sprite.animations.len());
    for anim in manifest.sprite.animations {
        println!("    - {} ({})", anim.name, anim.file);
    }

    Ok(())
}
