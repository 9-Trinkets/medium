# Architecture

Medium combines a Tauri daemon, an MCP bridge, and a manifest-driven ghost format into a cohesive system for controlling sprite-based avatars.

## System design

```mermaid
flowchart LR
    Agent[Agent or user command] --> MCP[medium mcp]
    MCP --> IPC[Local IPC socket]
    IPC --> Daemon[medium daemon]
    Daemon --> Sprite[Ghost sprite window]
    Daemon --> Bubble[Speech bubble window]
    Daemon --> Config[ghost.toml + local config]
```

## Command flow

```mermaid
sequenceDiagram
    participant U as User or agent
    participant M as medium mcp
    participant D as daemon
    participant S as sprite window
    participant B as bubble window

    U->>M: speak / summon / animate
    M->>D: routed command
    D->>S: update ghost state
    D->>B: update bubble text
    D-->>U: status / logs / visible ghost response
```

## Key components

- **Frontend (TypeScript)** — sprite rendering, bubble coordination, ghost manifest loading
- **Daemon (Rust)** — CLI, IPC socket server, window lifecycle management
- **MCP bridge** — exposes daemon commands to agents and Claude Code
- **Ghost manifests** — TOML-based format for frame size, animations, metadata, and provenance

See [Development](development.md#key-implementation-files) for a detailed file map.
