# Ghost manifest specification

Each ghost folder must contain a `ghost.toml` file. The manifest is the source of truth for the frontend and daemon:

- frame dimensions
- animation frame rate (FPS)
- sprite scale
- animation list and metadata
- initial animation on load
- provenance and attribution

## Example manifest

```toml
[ghost]
name = "vita"
description = "A bundled vita ghost imported from the Arks Digital dino pack."

[tts]
provider = "elevenlabs"
voice_id = "21m00Tcm4TlvDq8ikWAM"

[provenance]
source_type = "aseprite-pack"
source = "https://arks.digital/characters"
artist = "Arks"
attribution = "@ArksDigital — https://arks.digital/"
license = "CC BY 4.0"
notes = "Imported from the Arks Digital character pack"

[sprite]
enabled = true
frame_width = 24
frame_height = 24
fps = 8
scale = 4.0
flip_horizontal = false
initial_animation = "idle"

[[sprite.animations]]
file = "resources/animations/idle.png"
name = "idle"
intent = "Idle stance animation"
```

## Schema

### `[ghost]`

- **name** (string, required) — unique identifier for the ghost
- **description** (string, required) — human-readable description

### `[tts]`

Optional text-to-speech configuration:

- **provider** (string, optional) — TTS provider name (e.g., `google`, `elevenlabs`)
- **voice_id** (string, optional) — voice identifier for the provider

### `[provenance]`

- **source_type** (string, required if present) — how the ghost was created (`aseprite-pack`, `aseprite-raw`, `hand-crafted`, etc.)
- **source** (string, optional) — source URL or identifier
- **source_file** (string, optional) — original source file name
- **artist** (string, optional) — creator name or handle
- **attribution** (string, optional) — attribution text or URL
- **license** (string, optional) — license information
- **notes** (string, optional) — additional notes about the ghost's origin or creation

### `[sprite]`

- **enabled** (boolean, default: true) — whether sprite rendering is active
- **frame_width** (integer, required) — pixel width of each frame
- **frame_height** (integer, required) — pixel height of each frame
- **fps** (integer, required) — frames per second for animation playback
- **scale** (float, default: 1.0) — rendering scale multiplier
- **flip_horizontal** (boolean, default: false) — mirror the sprite horizontally
- **initial_animation** (string, required) — which animation to play on load (must match an entry in `sprite.animations`)

### `[[sprite.animations]]`

An array of animation definitions, each with:

- **file** (string, required) — path to the sprite sheet image (relative to ghost folder, must be under `resources/animations/`)
- **name** (string, required) — animation identifier (e.g., `idle`, `run`, `jump`); must be unique within the ghost
- **intent** (string, required) — description of the animation and its origin

## Validation rules

Run `medium ghosts validate <path>` to check a manifest:

**Ghost metadata:**
- `ghost.toml` exists
- `ghost.name` and `ghost.description` are non-empty

**Provenance (when present):**
- `provenance.source_type` is non-empty
- Optional metadata fields (`source`, `source_file`, `artist`, `attribution`, `license`, `notes`) must not be empty if provided

**Sprite configuration:**
- `sprite.frame_width`, `sprite.frame_height`, and `sprite.fps` are greater than zero
- `sprite.scale` must be greater than zero and finite
- When `sprite.enabled = true`, at least one animation must be defined
- When `sprite.enabled = true`, an `idle` animation must be defined

**Animations:**
- Each animation name must be non-empty and unique
- Each animation intent must be non-empty and required
- Each animation file must:
  - Be a relative path (no absolute paths)
  - Not use parent directory traversal (`..`)
  - Live under `resources/animations/`
  - Use an allowed extension (`.png`, `.gif`, `.webp`, `.jpg`, `.jpeg`)
  - Exist as a file in the ghost folder
  - Not resolve outside the ghost directory (no symlink escapes)

Example:

```bash
medium ghosts validate src/assets/ghosts/vita
```
