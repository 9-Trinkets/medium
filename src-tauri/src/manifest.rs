use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path};

const ANIMATION_ROOT: &str = "resources/animations";
const ALLOWED_ANIMATION_EXTENSIONS: &[&str] = &["png", "gif", "webp", "jpg", "jpeg"];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GhostManifest {
    pub ghost: GhostSection,
    pub tts: Option<TtsSection>,
    pub provenance: Option<ProvenanceSection>,
    pub sprite: SpriteSection,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GhostSection {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsSection {
    pub provider: Option<String>,
    pub voice_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceSection {
    pub source_type: String,
    pub source: Option<String>,
    pub source_file: Option<String>,
    pub artist: Option<String>,
    pub attribution: Option<String>,
    pub license: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpriteSection {
    pub enabled: bool,
    pub frame_width: u32,
    pub frame_height: u32,
    pub fps: u32,
    #[serde(default = "default_sprite_scale")]
    pub scale: f64,
    #[serde(default)]
    pub flip_horizontal: bool,
    pub animations: Vec<AnimationConfig>,
    pub initial_animation: Option<String>,
    #[serde(default)]
    pub balloon_offset_y: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationConfig {
    pub file: String,
    pub name: String,
    pub intent: String,
}

impl GhostManifest {
    pub fn load_and_validate(base_path: &Path) -> Result<Self> {
        let base_path = base_path
            .canonicalize()
            .with_context(|| format!("Ghost folder not found: {}", base_path.display()))?;

        if !base_path.is_dir() {
            anyhow::bail!("Ghost path is not a directory: {}", base_path.display());
        }

        let manifest_path = base_path.join("ghost.toml");
        if !manifest_path.exists() {
            anyhow::bail!("ghost.toml not found in {}", base_path.display());
        }

        let content = fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read {}", manifest_path.display()))?;

        let manifest: GhostManifest = toml::from_str(&content).map_err(|error| {
            anyhow::anyhow!("Failed to parse {}: {}", manifest_path.display(), error)
        })?;

        manifest.validate(&base_path)?;

        Ok(manifest)
    }

    fn validate(&self, base_path: &Path) -> Result<()> {
        let mut errors = Vec::new();

        if self.ghost.name.trim().is_empty() {
            errors.push("ghost.name must not be empty.".to_string());
        }

        if self.ghost.description.trim().is_empty() {
            errors.push("ghost.description must not be empty.".to_string());
        }

        if let Some(provenance) = &self.provenance {
            if provenance.source_type.trim().is_empty() {
                errors.push("provenance.source_type must not be empty.".to_string());
            }
            validate_optional_metadata_field("provenance.source", &provenance.source, &mut errors);
            validate_optional_metadata_field(
                "provenance.source_file",
                &provenance.source_file,
                &mut errors,
            );
            validate_optional_metadata_field("provenance.artist", &provenance.artist, &mut errors);
            validate_optional_metadata_field(
                "provenance.attribution",
                &provenance.attribution,
                &mut errors,
            );
            validate_optional_metadata_field(
                "provenance.license",
                &provenance.license,
                &mut errors,
            );
            validate_optional_metadata_field("provenance.notes", &provenance.notes, &mut errors);
        }

        if self.sprite.frame_width == 0 {
            errors.push("sprite.frame_width must be greater than 0.".to_string());
        }

        if self.sprite.frame_height == 0 {
            errors.push("sprite.frame_height must be greater than 0.".to_string());
        }

        if self.sprite.fps == 0 {
            errors.push("sprite.fps must be greater than 0.".to_string());
        }
        if !self.sprite.scale.is_finite() || self.sprite.scale <= 0.0 {
            errors.push("sprite.scale must be greater than 0.".to_string());
        }

        if self.sprite.enabled && self.sprite.animations.is_empty() {
            errors.push(
                "sprite.animations must contain at least one animation when sprite.enabled=true."
                    .to_string(),
            );
        }

        if self.sprite.enabled
            && !self
                .sprite
                .animations
                .iter()
                .any(|animation| animation.name == "idle")
        {
            errors.push("sprite.animations must include an 'idle' animation.".to_string());
        }

        let mut names = HashSet::new();
        for anim in &self.sprite.animations {
            if anim.name.trim().is_empty() {
                errors.push("sprite.animations[].name must not be empty.".to_string());
            }

            if anim.intent.trim().is_empty() {
                errors.push(format!(
                    "Animation '{}' must include a non-empty intent.",
                    anim.name
                ));
            }

            if anim.file.trim().is_empty() {
                errors.push(format!(
                    "Animation '{}' must include a non-empty file path.",
                    anim.name
                ));
                continue;
            }

            if !names.insert(anim.name.clone()) {
                errors.push(format!(
                    "Animation name '{}' is duplicated. Animation names must be unique.",
                    anim.name
                ));
            }

            errors.extend(validate_animation_asset(base_path, anim));
        }

        if !errors.is_empty() {
            anyhow::bail!("Validation failed:\n  - {}", errors.join("\n  - "));
        }

        Ok(())
    }
}

fn default_sprite_scale() -> f64 {
    1.0
}

fn validate_animation_asset(base_path: &Path, animation: &AnimationConfig) -> Vec<String> {
    let mut errors = Vec::new();
    let asset_relative = Path::new(animation.file.trim());

    if asset_relative.is_absolute() {
        errors.push(format!(
            "Animation '{}' must use a relative path inside {}: {}",
            animation.name, ANIMATION_ROOT, animation.file
        ));
    }

    if asset_relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        errors.push(format!(
            "Animation '{}' must not use parent directory traversal: {}",
            animation.name, animation.file
        ));
    }

    if !asset_relative.starts_with(ANIMATION_ROOT) {
        errors.push(format!(
            "Animation '{}' must live under {}/: {}",
            animation.name, ANIMATION_ROOT, animation.file
        ));
    }

    let extension = asset_relative
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    if !extension
        .as_deref()
        .is_some_and(|value| ALLOWED_ANIMATION_EXTENSIONS.contains(&value))
    {
        errors.push(format!(
            "Animation '{}' must use one of {:?}: {}",
            animation.name, ALLOWED_ANIMATION_EXTENSIONS, animation.file
        ));
    }

    if !errors.is_empty() {
        return errors;
    }

    let asset_path = base_path.join(asset_relative);
    if !asset_path.exists() {
        errors.push(format!(
            "Animation '{}' references missing file: {}",
            animation.name, animation.file
        ));
        return errors;
    }

    if !asset_path.is_file() {
        errors.push(format!(
            "Animation '{}' path is not a file: {}",
            animation.name, animation.file
        ));
        return errors;
    }

    let canonical_asset = match asset_path.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            errors.push(format!(
                "Animation '{}' could not resolve asset path {}: {}",
                animation.name, animation.file, error
            ));
            return errors;
        }
    };

    if !path_within_base(base_path, &canonical_asset) {
        errors.push(format!(
            "Animation '{}' resolves outside the ghost folder: {}",
            animation.name, animation.file
        ));
    }

    errors
}

fn validate_optional_metadata_field(
    field_name: &str,
    value: &Option<String>,
    errors: &mut Vec<String>,
) {
    if value
        .as_deref()
        .is_some_and(|inner| inner.trim().is_empty())
    {
        errors.push(format!("{field_name} must not be empty when provided."));
    }
}

fn path_within_base(base_path: &Path, candidate: &Path) -> bool {
    candidate.starts_with(base_path)
}

#[cfg(test)]
mod tests {
    use super::GhostManifest;
    use anyhow::Result;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[test]
    fn accepts_valid_manifest_with_flip_horizontal() -> Result<()> {
        let tmp = tempdir()?;
        write_animation(tmp.path(), "resources/animations/idle.png")?;
        write_manifest(
            tmp.path(),
            r#"
[ghost]
name = "warrior"
description = "A brave warrior."


[sprite]
enabled = true
frame_width = 192
frame_height = 192
fps = 8
flip_horizontal = true

[[sprite.animations]]
file = "resources/animations/idle.png"
name = "idle"
intent = "Stand by."
"#,
        )?;

        let manifest = GhostManifest::load_and_validate(tmp.path())?;
        assert!(manifest.sprite.flip_horizontal);
        Ok(())
    }

    #[test]
    fn rejects_parent_directory_traversal() -> Result<()> {
        let tmp = tempdir()?;
        let outside = tmp.path().join("outside.png");
        fs::write(&outside, b"not-a-real-image")?;
        write_manifest(
            tmp.path(),
            r#"
[ghost]
name = "bad"
description = "Bad ghost."


[sprite]
enabled = true
frame_width = 192
frame_height = 192
fps = 8

[[sprite.animations]]
file = "../outside.png"
name = "idle"
intent = "Oops."
"#,
        )?;

        let err = GhostManifest::load_and_validate(tmp.path()).unwrap_err();
        assert!(err
            .to_string()
            .contains("must not use parent directory traversal"));
        Ok(())
    }

    #[test]
    fn rejects_unknown_fields() -> Result<()> {
        let tmp = tempdir()?;
        write_animation(tmp.path(), "resources/animations/idle.png")?;
        write_manifest(
            tmp.path(),
            r#"
[ghost]
name = "archer"
description = "Archer."
extra = "nope"


[sprite]
enabled = true
frame_width = 192
frame_height = 192
fps = 8

[[sprite.animations]]
file = "resources/animations/idle.png"
name = "idle"
intent = "Stand by."
"#,
        )?;

        let err = GhostManifest::load_and_validate(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("unknown field"));
        Ok(())
    }

    #[test]
    fn rejects_duplicate_animation_names_and_zero_values() -> Result<()> {
        let tmp = tempdir()?;
        write_animation(tmp.path(), "resources/animations/idle.png")?;
        write_animation(tmp.path(), "resources/animations/run.png")?;
        write_manifest(
            tmp.path(),
            r#"
[ghost]
name = ""
description = ""

[sprite]
enabled = true
frame_width = 0
frame_height = 0
fps = 0

[[sprite.animations]]
file = "resources/animations/idle.png"
name = "idle"
intent = ""

[[sprite.animations]]
file = "resources/animations/run.png"
name = "idle"
intent = "Run."
"#,
        )?;

        let err = GhostManifest::load_and_validate(tmp.path()).unwrap_err();
        let error_text = err.to_string();
        assert!(
            error_text.contains("ghost.name must not be empty"),
            "Error was: {}",
            error_text
        );
        assert!(error_text.contains("ghost.description must not be empty"));
        assert!(error_text.contains("sprite.frame_width must be greater than 0"));
        assert!(error_text.contains("sprite.frame_height must be greater than 0"));
        assert!(error_text.contains("sprite.fps must be greater than 0"));
        assert!(error_text.contains("duplicated"));
        Ok(())
    }

    #[test]
    fn rejects_missing_idle_animation() -> Result<()> {
        let tmp = tempdir()?;
        write_animation(tmp.path(), "resources/animations/run.png")?;
        write_manifest(
            tmp.path(),
            r#"
[ghost]
name = "runner"
description = "No idle animation."


[sprite]
enabled = true
frame_width = 192
frame_height = 192
fps = 8

[[sprite.animations]]
file = "resources/animations/run.png"
name = "run"
intent = "Running."
"#,
        )?;

        let err = GhostManifest::load_and_validate(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("must include an 'idle' animation"));
        Ok(())
    }

    #[test]
    fn rejects_assets_outside_expected_folder_or_extension() -> Result<()> {
        let tmp = tempdir()?;
        write_animation(tmp.path(), "idle.txt")?;
        write_manifest(
            tmp.path(),
            r#"
[ghost]
name = "rogue"
description = "Rogue."


[sprite]
enabled = true
frame_width = 192
frame_height = 192
fps = 8

[[sprite.animations]]
file = "idle.txt"
name = "idle"
intent = "Stand by."
"#,
        )?;

        let err = GhostManifest::load_and_validate(tmp.path()).unwrap_err();
        let error_text = err.to_string();
        assert!(error_text.contains("must live under resources/animations/"));
        assert!(error_text.contains("must use one of"));
        Ok(())
    }

    fn write_manifest(base_path: &Path, contents: &str) -> Result<()> {
        fs::write(base_path.join("ghost.toml"), contents)?;
        Ok(())
    }

    fn write_animation(base_path: &Path, relative_path: &str) -> Result<()> {
        let path = base_path.join(PathBuf::from(relative_path));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, b"placeholder")?;
        Ok(())
    }
}
