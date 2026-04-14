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

[provenance]
source_type = "aseprite-pack"
artist = "Arks"
attribution = "@ArksDigital — https://arks.digital/"

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
intent = "Imported idle animation from an Aseprite-style source pack."
```

## Schema

### `[ghost]`

- **name** (string, required) — unique identifier for the ghost
- **description** (string, required) — human-readable description

### `[provenance]`

- **source_type** (string, required if present) — how the ghost was created (`aseprite-pack`, `aseprite-raw`, `hand-crafted`, etc.)
- **artist** (string, optional) — creator name or handle
- **attribution** (string, optional) — attribution text or URL
- **license** (string, optional) — license information

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

- **file** (string, required) — path to the sprite sheet image (relative to ghost folder)
- **name** (string, required) — animation identifier (e.g., `idle`, `run`, `jump`)
- **intent** (string, optional) — description of the animation and its origin

## Validation rules

Run `medium ghosts validate <path>` to check a manifest:

- `ghost.toml` exists
- `ghost.name` and `ghost.description` are non-empty
- `provenance.source_type` is non-empty when provenance is present
- `sprite.frame_width`, `sprite.frame_height`, and `sprite.fps` are greater than zero
- `sprite.scale` must be greater than zero
- `sprite.animations` includes an `idle` animation
- sprite sheet files exist at their declared paths

Example:

```bash
medium ghosts validate src/assets/ghosts/vita
```
