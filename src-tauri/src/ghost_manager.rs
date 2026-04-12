use crate::logging;
use crate::protocol::{Command, Event, RoutedCommand, TtsSettings};
use crate::tts;
use crate::{BubbleMessage, BubbleStore};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::broadcast;

const BUILTIN_VITA_MANIFEST: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/assets/ghosts/vita/ghost.toml"));

#[derive(Deserialize)]
struct BuiltinGhostManifest {
    sprite: BuiltinSpriteSection,
}

#[derive(Deserialize)]
struct BuiltinSpriteSection {
    frame_width: u32,
    frame_height: u32,
    #[serde(default = "default_builtin_scale")]
    scale: f64,
}

fn default_builtin_scale() -> f64 {
    1.0
}

pub struct GhostManager {
    active_ghost: String,
    known_ghosts: BTreeSet<String>,
    ghost_order: Vec<String>,
    bubble_store: BubbleStore,
    evt_tx: broadcast::Sender<Event>,
}

impl GhostManager {
    pub fn new(
        initial_ghost: String,
        evt_tx: broadcast::Sender<Event>,
        bubble_store: BubbleStore,
    ) -> Self {
        let mut known_ghosts = BTreeSet::new();
        known_ghosts.insert(initial_ghost.clone());

        Self {
            active_ghost: initial_ghost.clone(),
            known_ghosts,
            ghost_order: vec![initial_ghost],
            bubble_store,
            evt_tx,
        }
    }

    pub fn spawn_initial_ghost(&mut self, app_handle: &AppHandle) {
        if app_handle
            .get_webview_window(&sprite_label(&self.active_ghost))
            .is_none()
        {
            self.create_ghost_windows(app_handle, &self.active_ghost.clone());
        }
    }

    pub fn handle_command(&mut self, app_handle: &AppHandle, routed: RoutedCommand) {
        let RoutedCommand { ghost, command } = routed;

        let is_global = matches!(command, Command::Status | Command::Ping);
        if !is_global && !matches!(command, Command::Close) {
            self.ensure_ghost_windows(app_handle, &ghost);
            self.active_ghost = ghost.clone();
        }

        match &command {
            Command::SwitchGhost { name } => {
                logging::info(format!("Summoned {}", name));
                self.ensure_ghost_windows(app_handle, name);
                self.active_ghost = name.clone();
            }
            Command::Speak {
                text,
                personality: _,
                voice,
            } => {
                logging::info(format!(
                    "Speaking as {} (voice={})",
                    ghost,
                    voice.unwrap_or(true)
                ));
                self.deliver_speech(app_handle, &ghost, text.clone(), voice.unwrap_or(true));
                return;
            }
            Command::SetFacing { direction } => {
                logging::info(format!("Setting {} facing {}", ghost, direction));
                app_handle
                    .emit_to(
                        sprite_label(&ghost),
                        "ipc-command",
                        serde_json::json!({
                            "type": "set_facing",
                            "direction": direction
                        }),
                    )
                    .ok();
            }
            Command::Idle | Command::Stop => {
                logging::info(format!("Idling {}", ghost));
                app_handle
                    .emit_to(
                        sprite_label(&ghost),
                        "ipc-command",
                        serde_json::json!({
                            "type": "idle"
                        }),
                    )
                    .ok();
            }
            Command::Close => {
                logging::info(format!("Closing ghost {}", ghost));
                self.dismiss_ghost(app_handle, &ghost);
                return;
            }
            Command::MoveTo { x, y } => {
                logging::info(format!("Moving {} to {},{}", ghost, x, y));
                if let Some(window) = app_handle.get_webview_window(&sprite_label(&ghost)) {
                    let _ = window.set_position(tauri::PhysicalPosition { x: *x, y: *y });
                }
            }
            Command::GetPosition => {
                if let Some(window) = app_handle.get_webview_window(&sprite_label(&ghost)) {
                    if let Ok(pos) = window.outer_position() {
                        if let Ok(size) = window.outer_size() {
                            let evt = Event::Position {
                                x: pos.x,
                                y: pos.y,
                                width: size.width as i32,
                                height: size.height as i32,
                                screen_w: 0,
                                screen_h: 0,
                            };
                            let _ = self.evt_tx.send(evt);
                        }
                    }
                }
            }
            Command::Input { .. }
            | Command::PlayAnimation { .. }
            | Command::Ping
            | Command::Status => {
                if matches!(command, Command::Ping) {
                    let _ = self.evt_tx.send(Event::Pong);
                } else if matches!(command, Command::Status) {
                    let evt = Event::Status {
                        active_ghost: self.active_ghost.clone(),
                        known_ghosts: self.known_ghosts.iter().cloned().collect(),
                    };
                    let _ = self.evt_tx.send(evt);
                    return;
                } else if let Command::PlayAnimation { name, loop_anim } = &command {
                    logging::info(format!(
                        "Playing animation {} for {} (loop={})",
                        name, ghost, loop_anim
                    ));
                }
            }
        }

        app_handle
            .emit_to(sprite_label(&ghost), "ipc-command", &command)
            .unwrap();
    }

    fn deliver_speech(&self, app_handle: &AppHandle, ghost_name: &str, text: String, voice: bool) {
        let bubble_text = text.clone();
        if let Ok(mut bubble_store) = self.bubble_store.lock() {
            bubble_store.insert(
                ghost_name.to_string(),
                BubbleMessage {
                    text: bubble_text.clone(),
                    updated_at: Instant::now(),
                },
            );
        }

        app_handle
            .emit_to(
                bubble_label(ghost_name),
                "update-bubble",
                serde_json::json!({
                    "text": bubble_text
                }),
            )
            .ok();

        if voice {
            let tts_settings = Self::tts_settings_for(ghost_name);
            let speak_text = text.clone();
            let log_ghost = ghost_name.to_string();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = tts::speak(&speak_text, tts_settings).await {
                    logging::error(format!("TTS error for {}: {}", log_ghost, e));
                    eprintln!("TTS Error: {}", e);
                }
            });
        }

        app_handle
            .emit_to(
                sprite_label(ghost_name),
                "ipc-command",
                serde_json::json!({
                    "type": "speak",
                    "text": text
                }),
            )
            .ok();
    }

    fn get_ghost_window_size(&self, ghost_name: &str) -> (f64, f64) {
        // Try to load the ghost manifest from the preview path first (for custom ghosts)
        if let Ok(preview_path) = std::env::var("MEDIUM_PREVIEW_GHOST_PATH") {
            if let Ok(manifest) = crate::manifest::GhostManifest::load_and_validate(std::path::Path::new(&preview_path)) {
                return scaled_window_size(
                    manifest.sprite.frame_width,
                    manifest.sprite.frame_height,
                    manifest.sprite.scale,
                );
            }
        }

        // Try to load from configured ghosts directory
        if let Ok(ghosts_dir) = crate::config::ghosts_dir() {
            let ghost_path = ghosts_dir.join(ghost_name);
            if let Ok(manifest) = crate::manifest::GhostManifest::load_and_validate(&ghost_path) {
                return scaled_window_size(
                    manifest.sprite.frame_width,
                    manifest.sprite.frame_height,
                    manifest.sprite.scale,
                );
            }
        }

        if let Some(size) = builtin_ghost_window_size(ghost_name) {
            return size;
        }

        // Fall back to a simple 1x default when no manifest is available.
        (240.0, 260.0)
    }

    pub fn active_ghost(&self) -> &str {
        &self.active_ghost
    }

    pub fn known_ghosts(&self) -> impl Iterator<Item = &String> {
        self.known_ghosts.iter()
    }

    #[cfg(test)]
    fn ghost_order(&self) -> &[String] {
        &self.ghost_order
    }

    #[cfg(test)]
    fn switch_ghost(&mut self, ghost_name: String) {
        if self.known_ghosts.insert(ghost_name.clone()) {
            self.ghost_order.push(ghost_name.clone());
        }
        self.active_ghost = ghost_name;
    }

    fn ensure_ghost_windows(&mut self, app_handle: &AppHandle, ghost_name: &str) {
        if self.known_ghosts.insert(ghost_name.to_string()) {
            self.ghost_order.push(ghost_name.to_string());
            self.create_ghost_windows(app_handle, ghost_name);
        }
    }

    fn dismiss_ghost(&mut self, app_handle: &AppHandle, ghost_name: &str) {
        let removed_active = self.active_ghost == ghost_name;
        self.known_ghosts.remove(ghost_name);
        self.ghost_order.retain(|name| name != ghost_name);
        if let Ok(mut bubble_store) = self.bubble_store.lock() {
            bubble_store.remove(ghost_name);
        }

        if let Some(window) = app_handle.get_webview_window(&sprite_label(ghost_name)) {
            let _ = window.close();
        }
        if let Some(window) = app_handle.get_webview_window(&bubble_label(ghost_name)) {
            let _ = window.close();
        }

        if self.known_ghosts.is_empty() {
            let _ = app_handle.exit(0);
            return;
        }

        if removed_active {
            if let Some(next_ghost) = self.ghost_order.first().cloned() {
                self.active_ghost = next_ghost;
            }
        }
    }

    fn create_ghost_windows(&self, app_handle: &AppHandle, ghost_name: &str) {
        let ghost_index = self
            .ghost_order
            .iter()
            .position(|name| name == ghost_name)
            .unwrap_or(0);
        let (x, y) = ghost_layout(ghost_index);
        let sprite_label = sprite_label(ghost_name);
        let bubble_label = bubble_label(ghost_name);
        let title = title_case(ghost_name);

        // Try to determine window size from ghost manifest
        let (window_width, window_height) = self.get_ghost_window_size(ghost_name);

        let sprite = WebviewWindowBuilder::new(
            app_handle,
            sprite_label.clone(),
            WebviewUrl::App("index.html".into()),
        )
        .title(&title)
        .inner_size(window_width, window_height)
        .position(x, y)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .resizable(false)
        .focused(false)
        .build();

        if let Err(err) = sprite {
            logging::error(format!(
                "Failed to create sprite window for {}: {}",
                ghost_name, err
            ));
            eprintln!("Failed to create sprite window for {ghost_name}: {err}");
            return;
        }

        let bubble = WebviewWindowBuilder::new(
            app_handle,
            bubble_label.clone(),
            WebviewUrl::App("index.html".into()),
        )
        .title(format!("{title} Bubble"))
        .inner_size(400.0, 240.0)
        .position(x - 80.0, y - 240.0)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .resizable(false)
        .focused(false)
        .build();

        if let Ok(bubble) = bubble {
            let _ = bubble.set_focusable(false);
            let _ = bubble.set_ignore_cursor_events(true);
        }

        let emit_handle = app_handle.clone();
        let ghost_name = ghost_name.to_string();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            emit_handle
                .emit_to(
                    sprite_label,
                    "ipc-command",
                    serde_json::json!({
                        "type": "switch_ghost",
                        "name": ghost_name
                    }),
                )
                .ok();
        });
    }

    fn tts_settings_for(ghost_name: &str) -> Option<TtsSettings> {
        match ghost_name {
            "pawn" => Some(TtsSettings {
                provider: Some("openai".to_string()),
                voice_id: Some("nova".to_string()),
            }),
            "archer" => Some(TtsSettings {
                provider: Some("elevenlabs".to_string()),
                voice_id: Some("VU16byTywsWv5JpI8rbc".to_string()),
            }),
            "warrior" => Some(TtsSettings {
                provider: Some("elevenlabs".to_string()),
                voice_id: Some("G7ILShrCNLfmS0A37SXS".to_string()),
            }),
            _ => None,
        }
    }
}

fn sprite_label(ghost_name: &str) -> String {
    format!("ghost-{ghost_name}")
}

fn bubble_label(ghost_name: &str) -> String {
    format!("bubble-{ghost_name}")
}

fn scaled_window_size(frame_width: u32, frame_height: u32, scale: f64) -> (f64, f64) {
    let width = frame_width as f64 * scale;
    let height = (frame_height as f64 * scale) + 20.0;
    (width, height)
}

fn builtin_ghost_window_size(ghost_name: &str) -> Option<(f64, f64)> {
    let raw_manifest = match ghost_name {
        "vita" => BUILTIN_VITA_MANIFEST,
        _ => return None,
    };

    let manifest: BuiltinGhostManifest = toml::from_str(raw_manifest).ok()?;
    Some(scaled_window_size(
        manifest.sprite.frame_width,
        manifest.sprite.frame_height,
        manifest.sprite.scale,
    ))
}

fn title_case(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => "Ghost".to_string(),
    }
}

fn ghost_layout(index: usize) -> (f64, f64) {
    let column = (index % 2) as f64;
    let row = (index / 2) as f64;
    let x = 860.0 + (column * 360.0);
    let y = 520.0 + (row * 260.0);
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_active_and_known_ghosts() {
        let (evt_tx, _) = broadcast::channel(10);
        let mut manager = GhostManager::new(
            "pawn".to_string(),
            evt_tx,
            std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        );

        manager.switch_ghost("archer".to_string());

        assert_eq!(manager.active_ghost(), "archer");
        assert!(manager.known_ghosts().any(|ghost| ghost == "pawn"));
        assert!(manager.known_ghosts().any(|ghost| ghost == "archer"));
    }

    #[test]
    fn keeps_known_ghosts_when_switching() {
        let (evt_tx, _) = broadcast::channel(10);
        let mut manager = GhostManager::new(
            "pawn".to_string(),
            evt_tx,
            std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        );

        manager.switch_ghost("warrior".to_string());
        manager.switch_ghost("archer".to_string());

        assert_eq!(manager.active_ghost(), "archer");
        assert_eq!(manager.known_ghosts().count(), 3);
    }

    #[test]
    fn preserves_summon_order_for_layout() {
        let (evt_tx, _) = broadcast::channel(10);
        let mut manager = GhostManager::new(
            "pawn".to_string(),
            evt_tx,
            std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        );

        manager.switch_ghost("archer".to_string());
        manager.switch_ghost("warrior".to_string());

        assert_eq!(
            manager.ghost_order(),
            &[
                "pawn".to_string(),
                "archer".to_string(),
                "warrior".to_string()
            ]
        );
    }

    #[test]
    fn generates_window_labels() {
        assert_eq!(sprite_label("pawn"), "ghost-pawn");
        assert_eq!(bubble_label("archer"), "bubble-archer");
        assert_eq!(title_case("warrior"), "Warrior");
    }

    #[test]
    fn lays_out_ghosts_in_non_overlapping_grid() {
        assert_eq!(ghost_layout(0), (860.0, 520.0));
        assert_eq!(ghost_layout(1), (1220.0, 520.0));
        assert_eq!(ghost_layout(2), (860.0, 780.0));
    }

    #[test]
    fn sizes_builtin_vita_from_embedded_manifest() {
        assert_eq!(builtin_ghost_window_size("vita"), Some((96.0, 116.0)));
    }
}
