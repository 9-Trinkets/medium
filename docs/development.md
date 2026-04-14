# Development

## Setup

Install frontend dependencies once:

```bash
npm install
```

## Common workflows

### Build the frontend

```bash
npm run build
```

### Rebuild and restart the daemon

The safest workflow after runtime changes:

```bash
npm run redeploy:daemon
```

This script:
1. builds the frontend
2. reinstalls the `medium` binary
3. stops the old daemon
4. removes stale socket state if needed
5. starts the daemon again
6. prints `medium status`

## Key implementation files

**Frontend:**
- `src/ghosts.ts` — loader for bundled ghost manifests
- `src/main.ts` — sprite rendering and bubble coordination

**Daemon (Rust):**
- `src-tauri/src/main.rs` — CLI entrypoint
- `src-tauri/src/manifest.rs` — manifest schema and validation
- `src-tauri/src/ghost_manager.rs` — ghost window lifecycle and sizing

**Import system:**
- `src-tauri/src/cli/import.rs` — importer dispatch and extension point
- `src-tauri/src/cli/import/aseprite.rs` — Aseprite importer implementation
- `src-tauri/src/cli/validate.rs` — manifest validation command

**Utilities:**
- `scripts/redeploy-daemon.sh` — rebuild and daemon restart helper

## Repository layout

```text
medium/
  src/                 Frontend app and bundled ghost assets
    assets/ghosts/     Bundled ghost folders and ghost.toml manifests
  src-tauri/           Rust daemon, CLI, IPC, MCP bridge, validation logic
  scripts/             Developer helper scripts
  skills/medium-rituals/
  docs/                Documentation
  templates/           Configuration templates
```
