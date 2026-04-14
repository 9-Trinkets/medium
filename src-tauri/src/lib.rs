pub mod config;
pub mod ghost_manager;
pub mod ipc;
pub mod logging;
pub mod manifest;
pub mod mcp;
pub mod protocol;
pub mod tts;

use crate::ghost_manager::GhostManager;
use crate::ipc::{get_socket_paths, IpcServer};
use crate::protocol::{Event, RoutedCommand};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Manager, State};
use tokio::sync::{broadcast, mpsc};

const BUBBLE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct BubbleMessage {
    pub text: String,
    pub updated_at: Instant,
}

pub type BubbleStore = Arc<Mutex<HashMap<String, BubbleMessage>>>;

// Ghost listing functionality
use serde::Serialize;
use std::fs;

pub const DEFAULT_BUNDLED_GHOST: &str = "vita";

#[derive(Serialize, Debug, Clone)]
pub struct GhostList {
    pub builtin: Vec<String>,
    pub custom: Vec<String>,
}

pub fn get_ghost_list() -> anyhow::Result<GhostList> {
    let builtin_ghosts = vec![DEFAULT_BUNDLED_GHOST.to_string()];

    // Custom ghosts from configured directory
    let mut custom_ghosts = Vec::new();
    if let Ok(ghosts_dir) = config::ghosts_dir() {
        if ghosts_dir.exists() {
            if let Ok(entries) = fs::read_dir(&ghosts_dir) {
                custom_ghosts = entries
                    .filter_map(|entry| {
                        entry.ok().and_then(|e| {
                            let path = e.path();
                            if path.is_dir() {
                                path.file_name()
                                    .and_then(|name| name.to_str().map(|s| s.to_string()))
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                custom_ghosts.sort();
            }
        }
    }

    Ok(GhostList {
        builtin: builtin_ghosts,
        custom: custom_ghosts,
    })
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn push_event(event: Event, evt_tx: State<'_, broadcast::Sender<Event>>) {
    let _ = evt_tx.send(event);
}

#[tauri::command]
fn move_window(x: i32, y: i32, window: tauri::Window) {
    let _ = window.set_position(tauri::PhysicalPosition { x, y });
}

#[tauri::command]
fn sync_bubble(ghost_name: String, main_x: i32, main_y: i32, app_handle: tauri::AppHandle) {
    const DEFAULT_BUBBLE_VERTICAL_OFFSET: i32 = 24;

    let bubble_label = format!("bubble-{ghost_name}");
    let sprite_label = format!("ghost-{ghost_name}");
    if let Some(bubble) = app_handle.get_webview_window(&bubble_label) {
        if let Some(main_win) = app_handle.get_webview_window(&sprite_label) {
            if let (Ok(main_size), Ok(bubble_size)) = (main_win.outer_size(), bubble.outer_size()) {
                let offset_x = (main_size.width as i32 - bubble_size.width as i32) / 2;
                let target_y = main_y - bubble_size.height as i32 + DEFAULT_BUBBLE_VERTICAL_OFFSET;

                let _ = bubble.set_position(tauri::PhysicalPosition {
                    x: main_x + offset_x,
                    y: target_y,
                });
            }
        }
    }
}

#[tauri::command]
fn get_bubble_text(ghost_name: String, bubble_store: State<'_, BubbleStore>) -> Option<String> {
    let mut bubble_store = bubble_store.lock().ok()?;
    bubble_store.retain(|_, message| message.updated_at.elapsed() <= BUBBLE_TTL);
    bubble_store
        .get(&ghost_name)
        .map(|message| message.text.clone())
}

#[tauri::command]
fn get_preview_ghost_path() -> Option<String> {
    std::env::var("MEDIUM_PREVIEW_GHOST_PATH").ok()
}

#[tauri::command]
fn load_ghost_from_path(ghost_path: String) -> Result<serde_json::Value, String> {
    use base64::Engine;
    use image::GenericImageView;
    use std::fs;
    use std::path::Path;

    let path = Path::new(&ghost_path);
    let manifest = crate::manifest::GhostManifest::load_and_validate(path)
        .map_err(|e| format!("Failed to load ghost manifest: {}", e))?;

    let ghost_name = manifest.ghost.name.clone();
    let mut animations: HashMap<String, String> = HashMap::new();
    let mut animation_dimensions: HashMap<String, (u32, u32)> = HashMap::new();

    // Load each animation as a base64 data URL and detect dimensions
    for anim in &manifest.sprite.animations {
        let anim_path = path.join(&anim.file);
        let file_data = fs::read(&anim_path)
            .map_err(|e| format!("Failed to read animation {}: {}", anim.file, e))?;

        // Try to read image dimensions
        if let Ok(img) = image::load_from_memory(&file_data) {
            let (width, height) = img.dimensions();
            animation_dimensions.insert(anim.name.clone(), (width, height));
        }

        // Determine MIME type from file extension
        let mime_type = match std::path::Path::new(&anim.file)
            .extension()
            .and_then(|ext| ext.to_str())
        {
            Some("png") => "image/png",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            _ => "image/png",
        };

        let b64 = base64::engine::general_purpose::STANDARD.encode(&file_data);
        let data_url = format!("data:{};base64,{}", mime_type, b64);
        animations.insert(anim.name.clone(), data_url);
    }

    Ok(serde_json::json!({
        "name": ghost_name,
        "frame_width": manifest.sprite.frame_width,
        "frame_height": manifest.sprite.frame_height,
        "fps": manifest.sprite.fps,
        "scale": manifest.sprite.scale,
        "initialFacing": if manifest.sprite.flip_horizontal { "left" } else { "right" },
        "animations": animations,
        "animation_dimensions": animation_dimensions,
        "initial_animation": manifest.sprite.initial_animation
    }))
}

#[tauri::command]
fn load_ghost_from_name(ghost_name: String) -> Result<serde_json::Value, String> {
    // Try to load from configured ghosts directory
    let ghosts_dir =
        config::ghosts_dir().map_err(|e| format!("Failed to get ghosts directory: {}", e))?;

    let ghost_path = ghosts_dir.join(&ghost_name);
    if !ghost_path.exists() {
        return Err(format!(
            "Ghost '{}' not found in ghosts directory",
            ghost_name
        ));
    }

    load_ghost_from_path(ghost_path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(ghost_name: String, instance_name: String) {
    logging::info(format!(
        "Starting Medium daemon (instance={}, ghost={})",
        instance_name, ghost_name
    ));

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<RoutedCommand>(100);
    let (evt_tx, _evt_rx) = broadcast::channel::<Event>(100);
    let bubble_store: BubbleStore = Arc::new(Mutex::new(HashMap::new()));

    let evt_tx_for_tauri = evt_tx.clone();
    let evt_tx_for_manager = evt_tx.clone();
    let bubble_store_for_tauri = bubble_store.clone();
    let bubble_store_for_manager = bubble_store.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(evt_tx_for_tauri)
        .manage(bubble_store_for_tauri)
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let mut ghost_manager = GhostManager::new(
                ghost_name.clone(),
                evt_tx_for_manager.clone(),
                bubble_store_for_manager.clone(),
            );

            ghost_manager.spawn_initial_ghost(&app_handle);

            let (cmd_path, _) = get_socket_paths(&instance_name);
            let cmd_tx_for_ipc = cmd_tx.clone();
            let evt_rx_for_ipc = evt_tx.subscribe();
            tauri::async_runtime::spawn(async move {
                let ipc_server = IpcServer::new(&cmd_path);
                if let Err(e) = ipc_server.run(cmd_tx_for_ipc, evt_rx_for_ipc).await {
                    logging::error(format!("IPC server error: {}", e));
                    eprintln!("IPC Server error: {}", e);
                }
            });

            tauri::async_runtime::spawn(async move {
                while let Some(cmd) = cmd_rx.recv().await {
                    ghost_manager.handle_command(&app_handle, cmd);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            push_event,
            move_window,
            sync_bubble,
            get_bubble_text,
            get_preview_ghost_path,
            load_ghost_from_path,
            load_ghost_from_name
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
