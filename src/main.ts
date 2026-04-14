import { WebviewWindow, getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invoke } from '@tauri-apps/api/core';
import { DEFAULT_GHOST_NAME, GHOSTS, type GhostConfig } from './ghosts';

class SpriteEngine {
  canvas: HTMLCanvasElement;
  ctx: CanvasRenderingContext2D;
  currentGhost: GhostConfig;
  displayScale: number;
  currentAnim: string = 'idle';
  currentImage: HTMLImageElement | null = null;
  frameCount: number = 0;
  frameIndex: number = 0;
  lastUpdate: number = 0;
  isPlaying: boolean = true;
  isLooping: boolean = true;
  currentFacing: 'left' | 'right';

  constructor(canvasId: string, initialGhost: string = DEFAULT_GHOST_NAME, displayScale: number = 1) {
    this.canvas = document.getElementById(canvasId) as HTMLCanvasElement;
    this.ctx = this.canvas.getContext('2d', { alpha: true })!;
    this.currentGhost = GHOSTS[initialGhost] || GHOSTS[DEFAULT_GHOST_NAME];
    this.displayScale = displayScale;
    this.currentFacing = this.currentGhost.initialFacing;

    this.initCanvas();
    this.applyFacing();

    // Use animation from config or default to idle
    const animToPlay = this.currentGhost.initial_animation || 'idle';
    this.loadAnimation(animToPlay);
    this.animate(0);
  }

  initCanvas() {
    const renderWidth = Math.max(1, Math.round(this.currentGhost.frame_width * this.displayScale));
    const renderHeight = Math.max(1, Math.round(this.currentGhost.frame_height * this.displayScale));
    this.canvas.width = renderWidth;
    this.canvas.height = renderHeight;
    this.canvas.style.width = `${renderWidth}px`;
    this.canvas.style.height = `${renderHeight}px`;
    // Disable anti-aliasing during drawImage to prevent transparent edge artifacts
    this.ctx.imageSmoothingEnabled = false;
    // Clear canvas with transparent background
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }

  async switchGhost(name: string) {
    if (GHOSTS[name]) {
      this.currentGhost = GHOSTS[name];
      this.displayScale = this.currentGhost.scale ?? 1;
      this.currentFacing = this.currentGhost.initialFacing;
      this.initCanvas();
      this.applyFacing();
      await this.loadAnimation('idle');
    }
  }

  setFacing(direction: 'left' | 'right') {
    this.currentFacing = direction;
    this.applyFacing();
  }

  async loadAnimation(name: string, loop: boolean = true) {
    const url = this.currentGhost.animations[name] || this.currentGhost.animations['idle'];
    this.currentAnim = name;
    this.isLooping = loop;

    return new Promise<void>((resolve) => {
      const img = new Image();
      img.onload = () => {
        this.currentImage = img;

        // Use detected dimensions if available, otherwise calculate from image
        if (this.currentGhost.animation_dimensions && name in this.currentGhost.animation_dimensions) {
          const dims = this.currentGhost.animation_dimensions[name];
          this.frameCount = Math.floor(dims[0] / this.currentGhost.frame_width);
        } else {
          this.frameCount = Math.floor(img.width / this.currentGhost.frame_width);
        }

        // Ensure at least 1 frame
        if (this.frameCount < 1) {
          this.frameCount = 1;
        }

        this.frameIndex = 0;
        resolve();
      };

      img.onerror = () => {
        console.error(`Failed to load animation '${name}' from URL:`, url);
        // Use a default frame count
        this.frameCount = 1;
        this.frameIndex = 0;
        resolve();
      };

      img.src = url;
    });
  }

  update(timestamp: number) {
    if (!this.isPlaying || !this.currentImage) return;

    const elapsed = timestamp - this.lastUpdate;
    const frameDuration = 1000 / this.currentGhost.fps;

    if (elapsed > frameDuration) {
      this.frameIndex++;
      if (this.frameIndex >= this.frameCount) {
        if (this.isLooping) {
          this.frameIndex = 0;
        } else {
          this.frameIndex = this.frameCount - 1;
          this.isPlaying = false;
          setTimeout(() => {
              this.isPlaying = true;
              this.loadAnimation('idle')
          }, 500);
        }
      }
      this.lastUpdate = timestamp;
    }
  }

  draw() {
    // Always clear with transparent background
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    if (!this.currentImage) {
        return;
    }

    // Ensure smoothing is always off to prevent outline artifacts on transparent pixels
    this.ctx.imageSmoothingEnabled = false;

    const sx = this.frameIndex * this.currentGhost.frame_width;
    this.ctx.drawImage(
      this.currentImage,
      sx, 0, this.currentGhost.frame_width, this.currentGhost.frame_height,
      0, 0, this.canvas.width, this.canvas.height
    );
  }

  applyFacing() {
    this.canvas.style.transform = this.currentFacing === 'left' ? 'scaleX(-1)' : 'scaleX(1)';
  }

  animate(timestamp: number) {
    this.update(timestamp);
    this.draw();
    requestAnimationFrame((t) => this.animate(t));
  }
}

window.addEventListener('DOMContentLoaded', async () => {
    // Force transparent background immediately
    document.documentElement.style.background = 'transparent';
    document.body.style.background = 'transparent';

  const tauriWindow = getCurrentWebviewWindow();
  const windowLabel = tauriWindow.label;
  const isBubbleWindow = windowLabel.startsWith('bubble-');
  const isSpriteWindow = windowLabel.startsWith('ghost-');
  let ghostName = isBubbleWindow
      ? windowLabel.slice('bubble-'.length)
      : isSpriteWindow
        ? windowLabel.slice('ghost-'.length)
      : DEFAULT_GHOST_NAME;

  // Check if there's a custom ghost path to load (from preview)
  if (isSpriteWindow) {
    try {
      const previewPath = await invoke<string | null>('get_preview_ghost_path');
      if (previewPath) {
        const customGhost = await invoke<any>('load_ghost_from_path', { ghostPath: previewPath });
        if (customGhost) {
          GHOSTS[customGhost.name] = customGhost;
          ghostName = customGhost.name;
        }
      }
    } catch (e) {
      console.log('Could not load preview ghost:', e);
    }

    // If ghost is not in GHOSTS (not built-in), try to load it from configured ghosts directory
    if (!GHOSTS[ghostName]) {
      try {
        const customGhost = await invoke<any>('load_ghost_from_name', { ghostName });
        if (customGhost) {
          GHOSTS[customGhost.name] = customGhost;
        }
      } catch (e) {
        console.log('Could not load custom ghost:', e);
      }
    }
  }
  const bubbleLabel = `bubble-${ghostName}`;
  document.body.classList.add(`window-${windowLabel}`);
  if (isBubbleWindow) {
    document.body.classList.add('window-bubble');
  } else if (isSpriteWindow) {
    document.body.classList.add('window-ghost');
  } else {
    document.body.classList.add('window-main');
  }

    // Try to set window background to transparent via Tauri
    try {
      await tauriWindow.setDecorations(false);
    } catch (e) {
      console.log('Decorations already set');
    }

  const displayScale = isSpriteWindow ? (GHOSTS[ghostName]?.scale ?? 1) : 1;
  const engine = isSpriteWindow ? new SpriteEngine('sprite-canvas', ghostName, displayScale) : null;
    const bubble = document.getElementById('speech-bubble');
    const bubbleContent = document.querySelector('.bubble-content');
    let bubbleTimeout: number | null = null;

    // Initialize bubble position as soon as possible
  if (isSpriteWindow) {
      setTimeout(syncBubblePosition, 500); // Small delay to let windows spawn
      window.setInterval(() => {
        void syncBubblePosition();
      }, 75);
  }

  async function syncBubblePosition() {
      if (!isSpriteWindow) return;
      const pos = await tauriWindow.outerPosition();
      await invoke('sync_bubble', { ghostName, mainX: pos.x, mainY: pos.y });
  }

  async function showBubble(text: string) {
    if (isSpriteWindow) {
        const bubbleWin = await WebviewWindow.getByLabel(bubbleLabel);
        if (bubbleWin) {
            await syncBubblePosition();
              await tauriWindow.emitTo(bubbleLabel, 'update-bubble', { text });
            }
       } else {
          // In the bubble window
          if (bubble && bubbleContent) {
              bubbleContent.textContent = text;
              bubble.classList.remove('hidden');
              if (bubbleTimeout) clearTimeout(bubbleTimeout);
              bubbleTimeout = window.setTimeout(() => {
                  bubble.classList.add('hidden');
              }, 5000);
          }
      }
    }

    if (isBubbleWindow) {
        tauriWindow.listen('update-bubble', (event: any) => {
            console.log('Bubble received text:', event.payload.text);
            showBubble(event.payload.text);
            if (bubble) bubble.classList.remove('hidden');
        });

        const initialBubble = await invoke<string | null>('get_bubble_text', { ghostName });
        if (initialBubble) {
          await showBubble(initialBubble);
        }
    }

    // Manual drag with Rust invoke - attach to app div
    const appDiv = document.getElementById('app');
    let isDragging = false;
    let dragStartX = 0;
    let dragStartY = 0;
    let windowStartX = 0;
    let windowStartY = 0;

    if (appDiv && isSpriteWindow) {
      appDiv.addEventListener('mousedown', async (e: MouseEvent) => {
        isDragging = true;
        dragStartX = e.screenX;
        dragStartY = e.screenY;
        const pos = await tauriWindow.outerPosition();
        windowStartX = pos.x;
        windowStartY = pos.y;
      });
    }

    document.addEventListener('mousemove', async (e: MouseEvent) => {
      if (!isDragging) return;
      const deltaX = e.screenX - dragStartX;
      const deltaY = e.screenY - dragStartY;
      const newX = windowStartX + deltaX;
      const newY = windowStartY + deltaY;
      await invoke('move_window', { x: newX, y: newY });
      if (isSpriteWindow) {
          syncBubblePosition();
      }
    });

    document.addEventListener('mouseup', () => {
      isDragging = false;
    });

    tauriWindow.listen('ipc-command', (event: any) => {
      const cmd = event.payload;
      if (isSpriteWindow) {
          switch (cmd.type) {
            case 'switch_ghost':
              engine?.switchGhost(cmd.name);
              break;
            case 'play_animation':
              engine?.loadAnimation(cmd.name, cmd.loop_anim);
              break;
            case 'speak':
            case 'bubble':
              showBubble(cmd.text);
              break;
            case 'set_facing':
              if (engine) {
                  engine.setFacing(cmd.direction === 'left' ? 'left' : 'right');
               }
               break;
            case 'idle':
            case 'stop':
              engine?.loadAnimation('idle', true);
              break;
            case 'close':
              tauriWindow.close();
              break;
          }
      }
    });
});
