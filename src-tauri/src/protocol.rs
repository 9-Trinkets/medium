use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostConfig {
    pub ghost: GhostInfo,
    pub tts: Option<TtsSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutedCommand {
    pub ghost: String,
    pub command: Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostInfo {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TtsSettings {
    pub provider: Option<String>,
    pub voice_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Input {
        text: String,
    },
    Stop,
    GetPosition,
    SwitchGhost {
        name: String,
    },
    PlayAnimation {
        name: String,
        #[serde(default)]
        loop_anim: bool,
    },
    Speak {
        text: String,
        #[serde(default)]
        personality: Option<bool>,
        #[serde(default)]
        voice: Option<bool>,
    },
    Idle,
    Ping,
    SetFacing {
        direction: String,
    },
    MoveTo {
        x: i32,
        y: i32,
    },
    Close,
    Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Thinking,
    ToolStart {
        name: String,
    },
    ToolDone {
        name: String,
    },
    Text {
        text: String,
    },
    Done,
    Idle,
    Error {
        text: String,
    },
    Interrupted {
        feedback: String,
    },
    Pong,
    Position {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        screen_w: i32,
        screen_h: i32,
    },
    Status {
        active_ghost: String,
        known_ghosts: Vec<String>,
    },
}
