# Medium ghost authoring

Medium ghosts are sprite-first. A ghost is a folder with a `ghost.toml` manifest plus referenced
animation sheets under `resources/animations/`.

## Minimum structure

```text
my-ghost/
  ghost.toml
  resources/
    animations/
      idle.png
      talk.png
```

## Minimum manifest

```toml
[ghost]
name = "my-ghost"
description = "A simple Medium ghost."

[sprite]
enabled = true
frame_width = 128
frame_height = 128
fps = 8
scale = 1.0
initial_animation = "idle"

[[sprite.animations]]
name = "idle"
file = "resources/animations/idle.png"

[[sprite.animations]]
name = "talk"
file = "resources/animations/talk.png"
```

## Authoring rules

1. Keep every animation on a single sprite sheet with uniform frame size.
2. Include an `idle` animation. Validation fails without it.
3. Use relative paths inside `ghost.toml`.
4. Keep animation names stable. Agents and MCP tools refer to them by name.
5. Put attribution in `[provenance]` when assets come from another source.

## Validation and preview

```bash
medium ghosts validate ./my-ghost
medium ghosts preview ./my-ghost
```

Validation checks the manifest shape, sprite dimensions, required animation names, and asset path
safety.

## Importing from an Aseprite export

```bash
medium ghosts import aseprite ./dino-pack \
  --name my-ghost \
  --sheet exported.png \
  --artist "Artist name" \
  --attribution "@handle — https://example.com/"
```

The importer creates a Medium ghost folder with animation sheets and a matching manifest.

## Local library and default ghost

Store personal ghosts under `~/.medium/ghosts` or set a custom location in
`~/.medium/config.toml`:

```toml
[ghosts]
path = "/absolute/path/to/ghosts"

[integration]
default_ghost = "vita"
```

`integration.default_ghost` is used by `medium integrate` when `--ghost` is omitted.

## Agent integration

For repo-local use:

```bash
medium init
medium integrate copilot
```

For explicit setup:

```bash
medium integrate claude
medium integrate copilot
medium doctor
```

See `../templates/` for ready-to-copy MCP and config examples.
