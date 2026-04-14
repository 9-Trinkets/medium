use super::{AsepriteImportArgs, GhostImporter};
use anyhow::{Context, Result};
use image::{GenericImageView, RgbaImage};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tauri_app_lib::config;
use tauri_app_lib::manifest::{
    AnimationConfig, GhostManifest, GhostSection, ProvenanceSection, SpriteSection,
};
use tempfile::TempDir;

const SUPPORTED_SOURCE_EXTENSIONS: &[&str] = &["png", "gif", "webp", "jpg", "jpeg"];
const ASEPRITE_SOURCE_EXTENSIONS: &[&str] = &["ase", "aseprite"];
const ASEPRITE_BIN_ENV: &str = "MEDIUM_ASEPRITE_BIN";

pub struct AsepriteImporter {
    args: AsepriteImportArgs,
}

struct ResolvedImportSheet {
    sheet_path: PathBuf,
    data_path: Option<PathBuf>,
    source_type: String,
    source_file: Option<String>,
    used_aseprite_cli: bool,
    _temp_dir: Option<TempDir>,
}

#[derive(Debug, Deserialize)]
struct AsepriteSheetData {
    frames: Vec<AsepriteFrame>,
    #[serde(default)]
    meta: AsepriteSheetMeta,
}

#[derive(Debug, Deserialize)]
struct AsepriteFrame {
    frame: AsepriteRect,
}

#[derive(Debug, Deserialize)]
struct AsepriteRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Debug, Default, Deserialize)]
struct AsepriteSheetMeta {
    #[serde(default, rename = "frameTags")]
    frame_tags: Vec<AsepriteFrameTag>,
}

#[derive(Debug, Deserialize)]
struct AsepriteFrameTag {
    name: String,
    from: u32,
    to: u32,
}

impl AsepriteImporter {
    pub fn new(args: AsepriteImportArgs) -> Self {
        Self { args }
    }
}

impl GhostImporter for AsepriteImporter {
    fn run(&self) -> Result<()> {
        anyhow::ensure!(
            !self.args.name.trim().is_empty(),
            "Ghost name must not be empty."
        );
        anyhow::ensure!(
            self.args.frame_width > 0,
            "--frame-width must be greater than 0."
        );
        anyhow::ensure!(
            self.args.frame_height > 0,
            "--frame-height must be greater than 0."
        );
        anyhow::ensure!(
            self.args.idle_frames > 0,
            "--idle-frames must be greater than 0."
        );
        anyhow::ensure!(self.args.fps > 0, "--fps must be greater than 0.");

        let source_path = Path::new(&self.args.source)
            .canonicalize()
            .with_context(|| format!("Import source not found: {}", self.args.source))?;
        let selected_sheet = resolve_import_sheet(&source_path, self.args.sheet.as_deref())?;
        let target_path = resolve_target_path(&self.args.name, self.args.path.as_deref())?;

        if target_path.exists() {
            anyhow::bail!(
                "Target ghost path already exists: {}",
                target_path.display()
            );
        }

        let animations_dir = target_path.join("resources").join("animations");
        fs::create_dir_all(&animations_dir).with_context(|| {
            format!(
                "Failed to create imported ghost animations directory: {}",
                animations_dir.display()
            )
        })?;

        let extension = selected_sheet
            .sheet_path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .filter(|value| SUPPORTED_SOURCE_EXTENSIONS.contains(&value.as_str()))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Unsupported import source: {}",
                    selected_sheet.sheet_path.display()
                )
            })?;

        let animations = extract_import_animations(
            &selected_sheet,
            &animations_dir,
            &extension,
            self.args.frame_width,
            self.args.frame_height,
            self.args.idle_frames,
        )?;
        let initial_animation = if animations.iter().any(|animation| animation.name == "idle") {
            Some("idle".to_string())
        } else {
            animations.first().map(|animation| animation.name.clone())
        };

        let manifest = GhostManifest {
            ghost: GhostSection {
                name: self.args.name.clone(),
                description: self
                    .args
                    .description
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("An imported ghost created from an Aseprite-style source pack.")
                    .to_string(),
            },
            tts: None,
            provenance: Some(ProvenanceSection {
                source_type: selected_sheet.source_type.clone(),
                source: Some(source_path.display().to_string()),
                source_file: selected_sheet.source_file.clone(),
                artist: normalize_optional(self.args.artist.as_deref()),
                attribution: normalize_optional(self.args.attribution.as_deref()),
                license: normalize_optional(self.args.license_notes.as_deref()),
                notes: normalize_optional(self.args.notes.as_deref()),
            }),
            sprite: SpriteSection {
                enabled: true,
                frame_width: self.args.frame_width,
                frame_height: self.args.frame_height,
                fps: self.args.fps,
                scale: 1.0,
                flip_horizontal: false,
                animations,
                initial_animation,
                balloon_offset_y: None,
            },
        };

        let manifest_path = target_path.join("ghost.toml");
        let manifest_body = toml::to_string_pretty(&manifest)?;
        fs::write(&manifest_path, manifest_body)
            .with_context(|| format!("Failed to write {}", manifest_path.display()))?;

        GhostManifest::load_and_validate(&target_path)?;

        println!("✅ Imported ghost created at: {}", target_path.display());
        println!("  Source: {}", source_path.display());
        println!("  Sheet: {}", selected_sheet.sheet_path.display());
        println!("  Name: {}", self.args.name);
        if selected_sheet.used_aseprite_cli {
            println!("  Note: Exported raw Aseprite source through the Aseprite CLI.");
        } else if !aseprite_cli_available() {
            println!("  Note: Aseprite CLI not found; imported from exported sheet assets.");
        }
        println!(
            "  Next step: medium ghosts preview {}",
            target_path.display()
        );

        Ok(())
    }
}

fn resolve_target_path(name: &str, output_path: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = output_path {
        return Ok(PathBuf::from(path));
    }

    let ghosts_dir = config::ghosts_dir()?;
    fs::create_dir_all(&ghosts_dir).with_context(|| {
        format!(
            "Failed to create ghosts directory: {}",
            ghosts_dir.display()
        )
    })?;
    Ok(ghosts_dir.join(name))
}

fn resolve_import_sheet(source_path: &Path, sheet: Option<&str>) -> Result<ResolvedImportSheet> {
    if source_path.is_file() {
        return resolve_source_file(source_path);
    }

    if !source_path.is_dir() {
        anyhow::bail!(
            "Import source must be a file or directory: {}",
            source_path.display()
        );
    }

    if let Some(sheet_name) = sheet {
        let explicit = source_path.join(sheet_name);
        let nested = source_path.join("sheets").join(sheet_name);
        if explicit.exists() {
            return Ok(ResolvedImportSheet {
                sheet_path: resolve_source_file(&explicit)?.sheet_path,
                data_path: None,
                source_type: "aseprite-pack".to_string(),
                source_file: Some(explicit.display().to_string()),
                used_aseprite_cli: false,
                _temp_dir: None,
            });
        }
        if nested.exists() {
            return Ok(ResolvedImportSheet {
                sheet_path: resolve_source_file(&nested)?.sheet_path,
                data_path: None,
                source_type: "aseprite-pack".to_string(),
                source_file: Some(nested.display().to_string()),
                used_aseprite_cli: false,
                _temp_dir: None,
            });
        }
        anyhow::bail!(
            "Could not find sheet '{}' in {} or {}/sheets",
            sheet_name,
            source_path.display(),
            source_path.display()
        );
    }

    let candidates = discover_sheet_candidates(source_path)?;
    match candidates.as_slice() {
        [] => anyhow::bail!(
            "No exported sheet files found under {}. Pass a file path directly or use --sheet.",
            source_path.display()
        ),
        [single] => Ok(ResolvedImportSheet {
            sheet_path: single.clone(),
            data_path: None,
            source_type: "aseprite-pack".to_string(),
            source_file: Some(single.display().to_string()),
            used_aseprite_cli: false,
            _temp_dir: None,
        }),
        many => {
            let listed = many
                .iter()
                .map(|path| {
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!(
                "Multiple exported sheets were found under {}. Re-run with --sheet <filename>. Candidates: {}",
                source_path.display(),
                listed
            );
        }
    }
}

fn discover_sheet_candidates(source_path: &Path) -> Result<Vec<PathBuf>> {
    let mut candidates = Vec::new();
    for base in [source_path.join("sheets"), source_path.to_path_buf()] {
        if !base.exists() || !base.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&base).with_context(|| {
            format!("Failed to read import source directory: {}", base.display())
        })? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            let ext = ext.to_ascii_lowercase();
            if SUPPORTED_SOURCE_EXTENSIONS.contains(&ext.as_str()) {
                candidates.push(path);
            }
        }
        if !candidates.is_empty() {
            candidates.sort();
            candidates.dedup();
            break;
        }
    }

    Ok(candidates)
}

fn resolve_source_file(path: &Path) -> Result<ResolvedImportSheet> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| {
            anyhow::anyhow!("Import source has no file extension: {}", path.display())
        })?;

    if SUPPORTED_SOURCE_EXTENSIONS.contains(&extension.as_str()) {
        return Ok(ResolvedImportSheet {
            sheet_path: path.to_path_buf(),
            data_path: None,
            source_type: "spritesheet".to_string(),
            source_file: None,
            used_aseprite_cli: false,
            _temp_dir: None,
        });
    }

    if ASEPRITE_SOURCE_EXTENSIONS.contains(&extension.as_str()) {
        if !aseprite_cli_available() {
            anyhow::bail!(
                "Aseprite CLI not found, and '{}' is a raw Aseprite source file. Export a spritesheet first or point the importer at a pack directory with sheets/.",
                path.display()
            );
        }
        return export_raw_aseprite_sheet(path);
    }

    anyhow::bail!("Unsupported import source: {}", path.display())
}

fn extract_idle_strip(
    source_sheet: &Path,
    output_path: &Path,
    frame_width: u32,
    frame_height: u32,
    idle_frames: u32,
) -> Result<()> {
    let image = image::open(source_sheet)
        .with_context(|| format!("Failed to open imported sheet: {}", source_sheet.display()))?;
    let required_width = frame_width * idle_frames;
    let (width, height) = image.dimensions();

    anyhow::ensure!(
        width >= required_width,
        "Imported sheet is too narrow for {} frames of width {}: {}",
        idle_frames,
        frame_width,
        source_sheet.display()
    );
    anyhow::ensure!(
        height >= frame_height,
        "Imported sheet is shorter than the requested frame height {}: {}",
        frame_height,
        source_sheet.display()
    );

    let idle_strip = image.crop_imm(0, 0, required_width, frame_height);
    idle_strip.save(output_path).with_context(|| {
        format!(
            "Failed to save imported idle strip: {}",
            output_path.display()
        )
    })?;

    Ok(())
}

fn aseprite_cli_available() -> bool {
    StdCommand::new(aseprite_binary())
        .arg("--version")
        .output()
        .is_ok()
}

fn export_raw_aseprite_sheet(path: &Path) -> Result<ResolvedImportSheet> {
    let temp_dir = tempfile::tempdir().context("Failed to create temporary export directory.")?;
    let sheet_path = temp_dir.path().join("sheet.png");
    let data_path = temp_dir.path().join("sheet.json");
    let aseprite_bin = aseprite_binary();
    let output = StdCommand::new(&aseprite_bin)
        .arg("--batch")
        .arg(path)
        .arg("--sheet-type")
        .arg("horizontal")
        .arg("--sheet")
        .arg(&sheet_path)
        .arg("--data")
        .arg(&data_path)
        .arg("--format")
        .arg("json-array")
        .arg("--list-tags")
        .output()
        .with_context(|| format!("Failed to launch Aseprite CLI for {}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            "Aseprite CLI exited without an error message.".to_string()
        } else {
            stderr
        };
        anyhow::bail!(
            "Aseprite CLI failed to export '{}': {}",
            path.display(),
            detail
        );
    }

    if !sheet_path.exists() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let combined = [stderr.as_str(), stdout.as_str()]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let combined_lower = combined.to_ascii_lowercase();
        if combined_lower.contains("trial version")
            || combined_lower.contains("save operation is not supported in trial version")
        {
            anyhow::bail!(
                "Aseprite CLI at '{}' appears to be the trial build, which cannot export sheets from raw .ase/.aseprite files. Use the full Aseprite build or import from an exported sheet/pack directory instead.",
                aseprite_bin
            );
        }

        let detail = if combined.is_empty() {
            "Aseprite CLI exited successfully but did not write the expected sheet image."
                .to_string()
        } else {
            combined
        };
        anyhow::bail!(
            "Aseprite CLI did not produce an exported sheet for {}: {}",
            path.display(),
            detail
        );
    }

    Ok(ResolvedImportSheet {
        sheet_path,
        data_path: Some(data_path),
        source_type: "aseprite-file".to_string(),
        source_file: Some(path.display().to_string()),
        used_aseprite_cli: true,
        _temp_dir: Some(temp_dir),
    })
}

fn aseprite_binary() -> String {
    std::env::var(ASEPRITE_BIN_ENV).unwrap_or_else(|_| "aseprite".to_string())
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|inner| !inner.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_import_animations(
    selected_sheet: &ResolvedImportSheet,
    animations_dir: &Path,
    extension: &str,
    frame_width: u32,
    frame_height: u32,
    idle_frames: u32,
) -> Result<Vec<AnimationConfig>> {
    if let Some(data_path) = &selected_sheet.data_path {
        let tagged_animations = extract_tagged_animations(
            &selected_sheet.sheet_path,
            data_path,
            animations_dir,
            frame_width,
            frame_height,
        )?;
        if !tagged_animations.is_empty() {
            return Ok(tagged_animations);
        }
    }

    let imported_animation = animations_dir.join(format!("idle.{extension}"));
    extract_idle_strip(
        &selected_sheet.sheet_path,
        &imported_animation,
        frame_width,
        frame_height,
        idle_frames,
    )?;

    Ok(vec![AnimationConfig {
        file: format!("resources/animations/idle.{extension}"),
        name: "idle".to_string(),
        intent: "Imported idle animation from an Aseprite-style source pack.".to_string(),
    }])
}

fn extract_tagged_animations(
    source_sheet: &Path,
    data_path: &Path,
    animations_dir: &Path,
    frame_width: u32,
    frame_height: u32,
) -> Result<Vec<AnimationConfig>> {
    let data = fs::read_to_string(data_path).with_context(|| {
        format!(
            "Failed to read exported Aseprite metadata: {}",
            data_path.display()
        )
    })?;
    let export: AsepriteSheetData = serde_json::from_str(&data).with_context(|| {
        format!(
            "Failed to parse exported Aseprite metadata: {}",
            data_path.display()
        )
    })?;

    if export.meta.frame_tags.is_empty() {
        return Ok(Vec::new());
    }

    let sheet = image::open(source_sheet)
        .with_context(|| format!("Failed to open imported sheet: {}", source_sheet.display()))?;
    let mut animations = Vec::new();
    let mut seen_names = HashSet::new();

    for tag in export.meta.frame_tags {
        anyhow::ensure!(
            tag.from <= tag.to,
            "Invalid Aseprite tag '{}' with range {}..{}",
            tag.name,
            tag.from,
            tag.to
        );

        let normalized_name = normalize_animation_name(&tag.name);
        anyhow::ensure!(
            !normalized_name.is_empty(),
            "Aseprite tag '{}' could not be converted into a usable animation name.",
            tag.name
        );
        anyhow::ensure!(
            seen_names.insert(normalized_name.clone()),
            "Aseprite tag '{}' collides with another animation name after normalization ('{}').",
            tag.name,
            normalized_name
        );

        let frames = export
            .frames
            .get(tag.from as usize..=tag.to as usize)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Aseprite tag '{}' references frames outside the exported sheet.",
                    tag.name
                )
            })?;
        let frame_count = frames.len() as u32;
        let mut strip = RgbaImage::new(frame_width * frame_count, frame_height);

        for (index, frame) in frames.iter().enumerate() {
            anyhow::ensure!(
                frame.frame.w == frame_width && frame.frame.h == frame_height,
                "Aseprite tag '{}' exported frame {} has dimensions {}x{}, expected {}x{}.",
                tag.name,
                index,
                frame.frame.w,
                frame.frame.h,
                frame_width,
                frame_height
            );
            let cropped =
                sheet.crop_imm(frame.frame.x, frame.frame.y, frame.frame.w, frame.frame.h);
            image::imageops::replace(
                &mut strip,
                &cropped.to_rgba8(),
                i64::from((index as u32) * frame_width),
                0,
            );
        }

        let output_path = animations_dir.join(format!("{normalized_name}.png"));
        strip.save(&output_path).with_context(|| {
            format!(
                "Failed to save imported animation strip '{}': {}",
                normalized_name,
                output_path.display()
            )
        })?;

        animations.push(AnimationConfig {
            file: format!("resources/animations/{normalized_name}.png"),
            name: normalized_name,
            intent: format!("Imported '{}' animation from Aseprite tags.", tag.name),
        });
    }

    Ok(animations)
}

fn normalize_animation_name(name: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_dash = false;

    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !normalized.is_empty() && !last_was_dash {
            normalized.push('-');
            last_was_dash = true;
        }
    }

    while normalized.ends_with('-') {
        normalized.pop();
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    #[test]
    fn imports_first_idle_frames_from_sheet_directory() -> Result<()> {
        let temp = tempdir()?;
        let source_dir = temp.path().join("source");
        let sheets_dir = source_dir.join("sheets");
        fs::create_dir_all(&sheets_dir)?;

        let mut image = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(96, 24);
        for (x, _, pixel) in image.enumerate_pixels_mut() {
            let frame = (x / 24) as u8;
            *pixel = Rgba([frame * 40, 100, 200, 255]);
        }
        image.save(sheets_dir.join("demo.png"))?;

        let target = temp.path().join("ghosts").join("demo");
        AsepriteImporter::new(AsepriteImportArgs {
            source: source_dir.to_string_lossy().to_string(),
            name: "demo".to_string(),
            path: Some(target.to_string_lossy().to_string()),
            sheet: Some("demo.png".to_string()),
            description: Some("Imported demo ghost.".to_string()),
            artist: Some("Artist".to_string()),
            attribution: Some("Credit the artist.".to_string()),
            license_notes: Some("Allowed for testing.".to_string()),
            notes: Some("Imported in a test.".to_string()),
            frame_width: 24,
            frame_height: 24,
            idle_frames: 4,
            fps: 8,
        })
        .run()?;

        let manifest = GhostManifest::load_and_validate(&target)?;
        assert_eq!(manifest.ghost.name, "demo");
        assert_eq!(manifest.sprite.animations.len(), 1);
        assert_eq!(
            manifest.provenance.unwrap().artist.as_deref(),
            Some("Artist")
        );
        assert!(target.join("resources/animations/idle.png").exists());

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn imports_raw_aseprite_file_through_cli() -> Result<()> {
        let temp = tempdir()?;
        let source_file = temp.path().join("demo.aseprite");
        fs::write(&source_file, b"fake aseprite source")?;

        let fixture_sheet = temp.path().join("fixture.png");
        let mut image = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(96, 24);
        for (x, _, pixel) in image.enumerate_pixels_mut() {
            let frame = (x / 24) as u8;
            *pixel = Rgba([frame * 50, 120, 180, 255]);
        }
        image.save(&fixture_sheet)?;

        let fake_aseprite = temp.path().join("fake-aseprite.sh");
        let metadata_path = temp.path().join("fixture.json");
        fs::write(
            &metadata_path,
            r#"{
  "frames": [
    { "frame": { "x": 0, "y": 0, "w": 24, "h": 24 } },
    { "frame": { "x": 24, "y": 0, "w": 24, "h": 24 } },
    { "frame": { "x": 48, "y": 0, "w": 24, "h": 24 } },
    { "frame": { "x": 72, "y": 0, "w": 24, "h": 24 } }
  ],
  "meta": {
    "frameTags": [
      { "name": "Idle", "from": 0, "to": 1 },
      { "name": "Run", "from": 2, "to": 3 }
    ]
  }
}"#,
        )?;
        fs::write(
            &fake_aseprite,
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  echo \"Aseprite 1.3\"\n  exit 0\nfi\nsheet=\"\"\ndata=\"\"\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in\n    --sheet)\n      sheet=\"$2\"\n      shift 2\n      ;;\n    --data)\n      data=\"$2\"\n      shift 2\n      ;;\n    *)\n      shift\n      ;;\n  esac\ndone\ncp \"{}\" \"$sheet\"\ncp \"{}\" \"$data\"\n",
                fixture_sheet.display(),
                metadata_path.display()
            ),
        )?;
        let mut perms = fs::metadata(&fake_aseprite)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_aseprite, perms)?;

        let target = temp.path().join("ghosts").join("demo-cli");
        let previous = std::env::var_os(ASEPRITE_BIN_ENV);
        std::env::set_var(ASEPRITE_BIN_ENV, &fake_aseprite);

        let result = AsepriteImporter::new(AsepriteImportArgs {
            source: source_file.to_string_lossy().to_string(),
            name: "demo-cli".to_string(),
            path: Some(target.to_string_lossy().to_string()),
            sheet: None,
            description: Some("Imported through the CLI.".to_string()),
            artist: Some("Artist".to_string()),
            attribution: Some("Credit the artist.".to_string()),
            license_notes: None,
            notes: None,
            frame_width: 24,
            frame_height: 24,
            idle_frames: 4,
            fps: 8,
        })
        .run();

        match previous {
            Some(value) => std::env::set_var(ASEPRITE_BIN_ENV, value),
            None => std::env::remove_var(ASEPRITE_BIN_ENV),
        }

        result?;

        let manifest = GhostManifest::load_and_validate(&target)?;
        let provenance = manifest.provenance.unwrap();
        assert_eq!(provenance.source_type, "aseprite-file");
        assert_eq!(
            provenance.source_file,
            Some(source_file.canonicalize()?.display().to_string())
        );
        assert_eq!(manifest.sprite.animations.len(), 2);
        assert_eq!(manifest.sprite.initial_animation.as_deref(), Some("idle"));
        assert!(target.join("resources/animations/idle.png").exists());
        assert!(target.join("resources/animations/run.png").exists());

        Ok(())
    }
}
