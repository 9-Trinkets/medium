# Importing ghosts

Medium supports importing ghosts from Aseprite exports, packs, and raw `.ase` files. The importer generates a `ghost.toml` manifest and organizes sprite sheets into the Medium ghost format.

## Import from exported sheet or pack

If you've already exported sprites from Aseprite:

```bash
medium ghosts import aseprite ./dino-pack \
  --name doux \
  --sheet "DinoSprites - doux.png" \
  --artist "Arks" \
  --attribution "@ArksDigital — https://arks.digital/"
```

This imports a single sprite sheet and creates an `idle` animation for it.

## Import from raw `.ase` / `.aseprite`

If you have the full Aseprite CLI installed, Medium can import directly from source files:

```bash
export MEDIUM_ASEPRITE_BIN="/Applications/Aseprite.app/Contents/MacOS/aseprite"

medium ghosts import aseprite ./dino-pack/aseprite/doux.ase \
  --name doux \
  --artist "Arks" \
  --attribution "@ArksDigital — https://arks.digital/"
```

## Import behavior

- **Sheet-only imports** fall back to a single `idle` animation strip
- **Raw `.ase` imports** use the Aseprite CLI to export a temporary sheet and extract metadata
- **Aseprite tags** are converted to separate animation strips (e.g., `idle`, `run`, `jump`)

Each animation is validated against the [manifest specification](ghost-manifest.md) and the resulting ghost can be previewed:

```bash
medium ghosts preview ./path/to/imported-ghost
```

Validate before using:

```bash
medium ghosts validate ./path/to/imported-ghost
```

See [Ghost manifest](ghost-manifest.md) for the format and schema.
