import type { ThemeId } from './types';
import classicManifest from './assets/themes/classic_cat/sprites.json';
import windowManifest from './assets/themes/window_cat/sprites.json';
import notebookManifest from './assets/themes/cat_notebook/sprites.json';
import constellationManifest from './assets/themes/cat_constellation/sprites.json';
import classicSheet from './assets/themes/classic_cat/sprites.png?url';
import windowSheet from './assets/themes/window_cat/sprites.png?url';
import notebookSheet from './assets/themes/cat_notebook/sprites.png?url';
import constellationSheet from './assets/themes/cat_constellation/sprites.png?url';

export type MascotState = 'idle' | 'playing' | 'paused' | 'greet' | 'searching' | 'static';

export interface SpriteSequence {
  row: number;
  frames: number;
  fps: number;
  loop: boolean;
  static_frame?: number;
}

export interface SpriteManifest {
  version: number;
  theme_id: string;
  frame_size: number;
  columns: number;
  rows: number;
  sheet: string;
  sequences: Record<string, SpriteSequence>;
  posters: Record<string, { row: number; frame: number }>;
}

export interface MascotClock {
  now(): number;
  setInterval(fn: () => void, ms: number): number;
  clearInterval(id: number): void;
}

const defaultClock: MascotClock = {
  now: () => performance.now(),
  setInterval: (fn, ms) => window.setInterval(fn, ms) as unknown as number,
  clearInterval: (id) => window.clearInterval(id),
};

const manifests: Record<ThemeId, SpriteManifest> = {
  classic_cat: classicManifest as SpriteManifest,
  window_cat: windowManifest as SpriteManifest,
  cat_notebook: notebookManifest as SpriteManifest,
  cat_constellation: constellationManifest as SpriteManifest,
};

const sheets: Record<ThemeId, string> = {
  classic_cat: classicSheet,
  window_cat: windowSheet,
  cat_notebook: notebookSheet,
  cat_constellation: constellationSheet,
};

export function loadManifest(theme: ThemeId): SpriteManifest | null {
  return manifests[theme] ?? null;
}

export function sheetUrl(theme: ThemeId): string | null {
  return sheets[theme] ?? null;
}

export interface MascotInputs {
  themeId: ThemeId;
  enabled: boolean;
  reducedMotion: boolean;
  playing: boolean;
  hasTrack: boolean;
  searching: boolean;
  visible: boolean;
  lyricsOnly: boolean;
  controlsRevealed: boolean;
}

export class MascotController {
  private el: HTMLElement;
  private clock: MascotClock;
  private timer: number | null = null;
  private timerPeriod: number | null = null;
  private themeId: ThemeId = 'classic_cat';
  private manifest: SpriteManifest | null = null;
  private sheet: string | null = null;
  private failed = false;
  private sequence: MascotState = 'static';
  private frame = 0;
  private sequenceStartedAt = 0;
  private lastBase: MascotState = 'idle';
  private greeted = false;
  private searchPlayed = false;
  private pausedPoseApplied = false;
  private idleWaiting = false;
  private inputs: MascotInputs = {
    themeId: 'classic_cat',
    enabled: true,
    reducedMotion: false,
    playing: false,
    hasTrack: false,
    searching: false,
    visible: true,
    lyricsOnly: false,
    controlsRevealed: false,
  };

  constructor(el: HTMLElement, clock: MascotClock = defaultClock) {
    this.el = el;
    this.clock = clock;
  }

  update(partial: Partial<MascotInputs>) {
    const prev = this.inputs;
    this.inputs = { ...this.inputs, ...partial };
    if (this.inputs.themeId !== this.themeId) {
      this.themeId = this.inputs.themeId;
      this.loadTheme(this.themeId);
      this.cancelGesture();
    }
    if (prev.searching && !this.inputs.searching) this.searchPlayed = false;
    if (!this.inputs.hasTrack) this.searchPlayed = false;
    if (prev.playing && !this.inputs.playing) this.pausedPoseApplied = false;
    if (prev.playing !== this.inputs.playing || prev.hasTrack !== this.inputs.hasTrack) this.cancelGesture();
    this.resolve();
  }

  triggerGreet() {
    if (!this.canAnimate() || this.greeted) return;
    this.greeted = true;
    this.startSequence('greet');
  }

  leaveHover() {
    this.greeted = false;
  }

  destroy() {
    this.stopTimer();
  }

  get currentSequence() {
    return this.sequence;
  }

  get currentFrame() {
    return this.frame;
  }

  /** Test helper: inject assets without filesystem modules. */
  useAssets(manifest: SpriteManifest, sheet: string) {
    this.manifest = manifest;
    this.sheet = sheet;
    this.failed = false;
    this.themeId = (manifest.theme_id as ThemeId) || this.themeId;
    this.el.style.backgroundImage = `url("${sheet}")`;
  }

  private loadTheme(theme: ThemeId) {
    this.manifest = loadManifest(theme);
    this.sheet = sheetUrl(theme);
    this.failed = !this.manifest || !this.sheet;
    if (this.sheet) {
      this.el.style.backgroundImage = `url("${this.sheet}")`;
    } else {
      this.el.style.backgroundImage = '';
    }
  }

  private canAnimate() {
    if (this.failed || !this.manifest || !this.sheet) return false;
    if (!this.inputs.enabled || this.inputs.reducedMotion) return false;
    if (!this.inputs.visible) return false;
    if (this.inputs.lyricsOnly && !this.inputs.controlsRevealed) return false;
    return true;
  }

  private baseState(): MascotState {
    if (this.inputs.searching) return 'searching';
    if (!this.inputs.hasTrack) return 'idle';
    return this.inputs.playing ? 'playing' : 'paused';
  }

  private resolve() {
    if (!this.manifest) this.loadTheme(this.themeId);
    if (!this.canAnimate()) {
      this.stopTimer();
      const base = this.baseState();
      this.lastBase = base === 'searching' ? 'idle' : base;
      this.showPoster(this.lastBase);
      return;
    }

    const base = this.baseState();
    if (base === 'searching' && !this.searchPlayed && this.sequence !== 'searching') {
      this.searchPlayed = true;
      this.startSequence('searching');
      return;
    }

    if (this.sequence === 'greet' || this.sequence === 'searching') {
      this.ensureTimer();
      return;
    }

    if (base !== this.lastBase || this.sequence === 'static') {
      this.lastBase = base;
      if (base === 'paused' && this.pausedPoseApplied) {
        this.showPoster('paused');
        this.stopTimer();
        return;
      }
      this.startSequence(base);
    } else if (!(base === 'paused' && this.pausedPoseApplied) && !this.idleWaiting) {
      this.ensureTimer();
    }
  }

  private cancelGesture() {
    if (this.sequence === 'greet' || this.sequence === 'searching') {
      this.sequence = 'static';
      this.stopTimer();
    }
  }

  private startSequence(name: MascotState) {
    if (!this.manifest) return;
    // A new sequence must replace the slow interval between idle blinks.
    this.stopTimer();
    this.idleWaiting = false;
    this.sequence = name;
    this.frame = 0;
    this.sequenceStartedAt = this.clock.now();
    this.paintFrame();
    if (name === 'paused') this.pausedPoseApplied = false;
    this.ensureTimer();
  }

  private ensureTimer() {
    const fps = this.manifest?.sequences[this.sequence]?.fps ?? 8;
    this.schedule(() => this.tick(), 1000 / fps);
  }

  private schedule(callback: () => void, period: number) {
    if (this.timer != null && this.timerPeriod === period) return;
    this.stopTimer();
    this.timerPeriod = period;
    this.timer = this.clock.setInterval(callback, period);
  }

  private stopTimer() {
    if (this.timer != null) {
      this.clock.clearInterval(this.timer);
      this.timer = null;
    }
    this.timerPeriod = null;
  }

  private tick() {
    if (!this.canAnimate() || !this.manifest) {
      this.stopTimer();
      return;
    }
    const seq = this.manifest.sequences[this.sequence];
    if (!seq || seq.frames <= 1) {
      this.showPoster(this.baseState());
      this.stopTimer();
      return;
    }
    const elapsed = this.clock.now() - this.sequenceStartedAt;
    const frameDuration = 1000 / seq.fps;
    const nextFrame = Math.floor(elapsed / frameDuration);
    if (seq.loop) {
      this.frame = nextFrame % seq.frames;
      this.paintFrame();
      return;
    }
    if (nextFrame >= seq.frames) {
      if (this.sequence === 'paused') {
        this.pausedPoseApplied = true;
        this.frame = seq.static_frame ?? seq.frames - 1;
        this.paintFrame();
        this.stopTimer();
        return;
      }
      if (this.sequence === 'greet' || this.sequence === 'searching') {
        this.sequence = 'static';
        this.lastBase = 'static';
        this.resolve();
        return;
      }
      if (this.sequence === 'idle') {
        this.frame = 0;
        this.paintFrame();
        this.stopTimer();
        this.idleWaiting = true;
        // schedule next idle blink later via next resolve when still idle
        this.sequenceStartedAt = this.clock.now();
        this.schedule(() => {
          if (this.baseState() === 'idle' && this.canAnimate()) this.startSequence('idle');
          else this.resolve();
        }, 7000);
        return;
      }
      this.showPoster(this.baseState());
      this.stopTimer();
      return;
    }
    this.frame = nextFrame;
    this.paintFrame();
  }

  private showPoster(state: MascotState) {
    if (!this.manifest) return;
    const key = state === 'searching' ? 'idle' : state;
    const poster = this.manifest.posters[key] ?? this.manifest.posters.idle;
    this.sequence = 'static';
    this.frame = poster.frame;
    this.paintAt(poster.row, poster.frame);
  }

  private paintFrame() {
    if (!this.manifest) return;
    const seq = this.manifest.sequences[this.sequence] ?? this.manifest.sequences.static;
    const frame = Math.min(this.frame, Math.max(0, seq.frames - 1));
    this.paintAt(seq.row, frame);
  }

  private paintAt(row: number, frame: number) {
    if (!this.manifest) return;
    // Percentages address a whole cell at both 24 CSS px and the 48 px source.
    const { columns, rows } = this.manifest;
    this.el.style.backgroundSize = `${columns * 100}% ${rows * 100}%`;
    this.el.style.backgroundPosition = `${columns > 1 ? frame * 100 / (columns - 1) : 0}% ${rows > 1 ? row * 100 / (rows - 1) : 0}%`;
    this.el.dataset.sequence = this.sequence;
    this.el.dataset.frame = String(frame);
  }
}
