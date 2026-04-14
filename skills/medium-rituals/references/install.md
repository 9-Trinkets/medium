# Medium rituals installation

Use Medium's built-in generators instead of hand-writing MCP config where possible.

## Quick setup

```bash
medium init
medium doctor
```

`medium init` creates `~/.medium/config.toml` if needed and writes a project-local Claude entry to
`.mcp.json`.

## Claude

Project-local:

```bash
medium integrate claude
```

Global:

```bash
medium integrate claude --global
```

Generated paths:

- project: `.mcp.json`
- global: `~/.claude/.mcp.json`

## GitHub Copilot

Workspace-local:

```bash
medium integrate copilot
```

Global:

```bash
medium integrate copilot --global
```

Generated paths:

- workspace: `.vscode/mcp.json`
- global: `~/.copilot/mcp-config.json`

## Default ghost selection

Use the configured default ghost:

```bash
medium config get integration.default_ghost
```

Set it:

```bash
medium config set integration.default_ghost vita
```

Override it per integration:

```bash
medium integrate claude --ghost vita
medium integrate copilot --ghost imported-ghost
```
