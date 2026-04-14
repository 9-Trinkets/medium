# Releases

Medium uses GitHub Actions to build and publish binaries for macOS and Linux.

## Release workflow

The release workflow is defined at `.github/workflows/release.yml`.

### Steps

1. Sync the latest `medium/` snapshot to the public repository
2. Push a tag like `v0.1.0`, or trigger the workflow manually with a `release_tag` parameter
3. GitHub Actions builds Tauri binaries on macOS and Linux
4. The workflow creates a draft GitHub Release and uploads the build artifacts

### Access the release

Builds are published as draft releases on GitHub. Once approved, they become public.

## Build targets

- **macOS** — universal binary (Intel + Apple Silicon)
- **Linux** — x86_64 ELF binary
- **Windows** — not yet published (IPC still depends on Unix sockets)

## Notes

- The release workflow lives in `medium/.github/workflows/` in the monorepo, so it lands at the public repo root after sync
- The public sync workflow publishes only the tracked `medium/` snapshot, so local asset folders do not leak
- Release artifacts include the `medium` binary and optional installer bundles
