use crate::config::GlobalTtsConfig;
use crate::protocol::TtsSettings;
use anyhow::{Context, Result};
use std::env;
use std::io::Write;
use std::process::Command as StdCommand;
use tempfile::NamedTempFile;

fn get_global_config() -> GlobalTtsConfig {
    crate::config::load_global_config()
        .ok()
        .flatten()
        .and_then(|config| config.tts)
        .unwrap_or_default()
}

pub async fn speak(text: &str, tts_settings: Option<TtsSettings>) -> Result<()> {
    let global_config = get_global_config();
    let settings = tts_settings.unwrap_or_default();
    let provider = settings.provider.as_deref().unwrap_or("openai");

    match provider {
        "elevenlabs" => {
            let el_key = env::var("ELEVENLABS_API_KEY")
                .or_else(|_| {
                    global_config
                        .elevenlabs_api_key
                        .clone()
                        .ok_or(anyhow::anyhow!("Missing ElevenLabs API key"))
                })
                .context("ELEVENLABS_API_KEY not found in environment or config.toml")?;
            let el_voice = settings
                .voice_id
                .context("ElevenLabs voice ID missing in ghost.toml [tts] section")?;

            let client = reqwest::Client::new();
            let response = client
                .post(format!(
                    "https://api.elevenlabs.io/v1/text-to-speech/{}",
                    el_voice
                ))
                .header("xi-api-key", el_key)
                .json(&serde_json::json!({
                    "text": text,
                    "model_id": "eleven_turbo_v2_5"
                }))
                .send()
                .await
                .context("Failed to send request to ElevenLabs")?;

            if !response.status().is_success() {
                let err_text = response.text().await?;
                anyhow::bail!("ElevenLabs TTS error: {}", err_text);
            }

            let bytes = response
                .bytes()
                .await
                .context("Failed to read ElevenLabs response")?;
            play_audio_bytes(&bytes).await
        }
        "macos" | "mac" | "native" => {
            let mut cmd = StdCommand::new("say");
            cmd.arg(text);
            if let Some(voice_id) = settings.voice_id {
                cmd.arg("-v").arg(voice_id);
            }

            let status = cmd.status().context("Failed to execute 'say' command")?;
            if !status.success() {
                anyhow::bail!("'say' command failed");
            }
            Ok(())
        }
        _ => {
            let api_key = env::var("OPENAI_API_KEY")
                .or_else(|_| {
                    global_config
                        .openai_api_key
                        .clone()
                        .ok_or(anyhow::anyhow!("Missing OpenAI API key"))
                })
                .context("OPENAI_API_KEY not found in environment or config.toml")?;

            let voice_id = settings.voice_id.unwrap_or_else(|| "nova".to_string());

            let client = reqwest::Client::new();
            let response = client
                .post("https://api.openai.com/v1/audio/speech")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&serde_json::json!({
                    "model": "tts-1",
                    "voice": voice_id,
                    "input": text,
                }))
                .send()
                .await
                .context("Failed to send request to OpenAI TTS")?;

            if !response.status().is_success() {
                let err_text = response.text().await?;
                anyhow::bail!("OpenAI TTS error: {}", err_text);
            }

            let bytes = response
                .bytes()
                .await
                .context("Failed to read OpenAI TTS response")?;
            play_audio_bytes(&bytes).await
        }
    }
}

async fn play_audio_bytes(bytes: &[u8]) -> Result<()> {
    let mut tmp_file = NamedTempFile::new().context("Failed to create temp file for audio")?;
    tmp_file
        .write_all(bytes)
        .context("Failed to write audio to temp file")?;

    let path = tmp_file.path().to_str().context("Invalid temp file path")?;

    let status = StdCommand::new("afplay")
        .arg(path)
        .status()
        .context("Failed to execute afplay")?;

    if !status.success() {
        anyhow::bail!("afplay failed to play audio");
    }

    Ok(())
}
