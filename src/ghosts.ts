/// <reference types="vite/client" />

import * as TOML from '@iarna/toml';

export interface GhostConfig {
  name: string;
  frame_width: number;
  frame_height: number;
  fps: number;
  scale?: number;
  initialFacing: 'left' | 'right';
  animations: Record<string, string>;
  animation_dimensions?: Record<string, [number, number]>;
  initial_animation?: string;
}

const DEFAULT_BUNDLED_GHOST_NAME = 'vita';

interface GhostManifest {
  ghost: {
    name: string;
  };
  sprite: {
    enabled: boolean;
    frame_width: number;
    frame_height: number;
    fps: number;
    scale?: number;
    flip_horizontal?: boolean;
    animations: Array<{
      file: string;
      name: string;
      intent: string;
    }>;
  };
}

const manifestSources = import.meta.glob('./assets/ghosts/*/ghost.toml', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

const animationSources = import.meta.glob('./assets/ghosts/*/resources/animations/*.{png,gif,webp,jpg,jpeg}', {
  eager: true,
  import: 'default',
}) as Record<string, string>;

export const GHOSTS = buildGhostConfigs();
export const DEFAULT_GHOST_NAME = resolveDefaultGhostName(GHOSTS);

function buildGhostConfigs(): Record<string, GhostConfig> {
  const configs: Record<string, GhostConfig> = {};
  const animationIndex = buildAnimationIndex();

  for (const [manifestPath, rawToml] of Object.entries(manifestSources)) {
    const folderName = extractGhostFolder(manifestPath, /\/ghost\.toml$/);
    const manifest = TOML.parse(rawToml) as unknown as GhostManifest;

    if (!manifest.sprite?.enabled) {
      continue;
    }

    const animations: Record<string, string> = {};
    const assets = animationIndex[folderName] ?? {};

    for (const animation of manifest.sprite.animations) {
      const url = assets[animation.file];
      if (!url) {
        throw new Error(
          `Missing bundled animation asset for ghost '${manifest.ghost.name}': ${animation.file}`
        );
      }
      animations[animation.name] = url;
    }

    configs[manifest.ghost.name] = {
      name: manifest.ghost.name,
      frame_width: manifest.sprite.frame_width,
      frame_height: manifest.sprite.frame_height,
      fps: manifest.sprite.fps,
      scale: manifest.sprite.scale ?? 1,
      initialFacing: manifest.sprite.flip_horizontal ? 'left' : 'right',
      animations,
    };
  }

  if (Object.keys(configs).length === 0) {
    throw new Error('No bundled ghost manifests were found.');
  }

  return configs;
}

function resolveDefaultGhostName(configs: Record<string, GhostConfig>): string {
  if (configs[DEFAULT_BUNDLED_GHOST_NAME]) {
    return DEFAULT_BUNDLED_GHOST_NAME;
  }

  const [firstGhost] = Object.keys(configs).sort();
  if (!firstGhost) {
    throw new Error('No bundled ghosts are available.');
  }

  return firstGhost;
}

function buildAnimationIndex(): Record<string, Record<string, string>> {
  const index: Record<string, Record<string, string>> = {};

  for (const [assetPath, url] of Object.entries(animationSources)) {
    const folderName = extractGhostFolder(
      assetPath,
      /\/resources\/animations\/[^/]+$/
    );
    const relativePath = assetPath.split(`/assets/ghosts/${folderName}/`)[1];
    if (!relativePath) {
      continue;
    }

    if (!index[folderName]) {
      index[folderName] = {};
    }
    index[folderName][relativePath] = url;
  }

  return index;
}

function extractGhostFolder(path: string, suffixPattern: RegExp): string {
  const normalized = path.replace(/\\/g, '/');
  const match = normalized.match(/\/assets\/ghosts\/([^/]+)(?:\/|$)/);
  if (!match) {
    throw new Error(`Could not derive ghost folder from path: ${path}`);
  }

  const folderName = match[1];
  if (!suffixPattern.test(normalized)) {
    throw new Error(`Unexpected ghost asset path shape: ${path}`);
  }

  return folderName;
}
