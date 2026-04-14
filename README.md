# Medium

Medium is a local desktop runtime for sprite-based ghost avatars. It combines a Tauri daemon, an MCP bridge, a manifest-driven ghost format, and a small toolchain for validating and importing ghost assets.

## What it provides

- multi-ghost desktop windows
- speech bubbles, animation control, and optional TTS
- a local daemon plus IPC socket
- an MCP bridge for agent control
- ghost validation, preview, and sprite import commands

## Quick start

Initialize Medium for your project:

```bash
npm install
npm run build
npm run redeploy:daemon
medium init
```

This installs dependencies, builds the frontend, starts the daemon, and sets up Claude integration.

## Documentation

- **[Architecture](docs/architecture.md)** — system design, daemon, MCP bridge
- **[Commands](docs/commands.md)** — CLI reference for all `medium` commands
- **[Configuration](docs/configuration.md)** — setup, integration, and config locations
- **[Ghost manifest](docs/ghost-manifest.md)** — TOML schema and validation rules
- **[Importing ghosts](docs/importing-ghosts.md)** — import from Aseprite and other sources
- **[Development](docs/development.md)** — setup, build workflows, key files
- **[Skills](docs/skills.md)** — create new skills with `npx skills add`
- **[Releases](docs/releases.md)** — GitHub Actions build and publish process

## Default ghost

The bundled default ghost is **`vita`**, from the [Arks Digital dino pack](https://arks.digital/) by @ArksDigital.

## Repository layout

```text
medium/
  src/                 Frontend app and bundled ghost assets
    assets/ghosts/     Bundled ghost folders and ghost.toml manifests
  src-tauri/           Rust daemon, CLI, IPC, MCP bridge, validation logic
  scripts/             Developer helper scripts
  docs/                Documentation (see links above)
  templates/           Configuration templates
  skills/medium-rituals/
```
