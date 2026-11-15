import { describe, expect, it } from 'vitest';
import { MascotController, type MascotClock, type SpriteManifest } from './mascot';

function fakeManifest(): SpriteManifest {
  return {
    version: 1,
    theme_id: 'classic_cat',
    frame_size: 48,
    columns: 8,
    rows: 6,
    sheet: 'sprites.png',
    sequences: {
      idle: { row: 0, frames: 4, fps: 4, loop: false },
      playing: { row: 1, frames: 8, fps: 8, loop: true },
      paused: { row: 2, frames: 6, fps: 4, loop: false, static_frame: 5 },
      greet: { row: 3, frames: 6, fps: 8, loop: false },
      searching: { row: 4, frames: 6, fps: 6, loop: false },
      static: { row: 5, frames: 1, fps: 1, loop: false },
    },
    posters: {
      idle: { row: 5, frame: 0 },
      playing: { row: 1, frame: 0 },
      paused: { row: 2, frame: 5 },
      searching: { row: 4, frame: 0 },
    },
  };
}

class FakeClock implements MascotClock {
  nowMs = 0;
  private timers = new Map<number, { fn: () => void; ms: number; next: number }>();
  private nextId = 1;
  now() { return this.nowMs; }
  setInterval(fn: () => void, ms: number) {
    const id = this.nextId++;
    this.timers.set(id, { fn, ms, next: this.nowMs + ms });
    return id;
  }
  clearInterval(id: number) { this.timers.delete(id); }
  get activeTimers() { return this.timers.size; }
  advance(ms: number) {
    const end = this.nowMs + ms;
    while (this.nowMs < end && this.timers.size) {
      let stepTo = end;
      for (const timer of this.timers.values()) stepTo = Math.min(stepTo, timer.next);
      this.nowMs = stepTo;
      for (const [id, timer] of [...this.timers.entries()]) {
        if (!this.timers.has(id)) continue;
        if (timer.next <= this.nowMs) {
          timer.fn();
          if (this.timers.has(id)) timer.next = this.nowMs + timer.ms;
        }
      }
    }
    this.nowMs = end;
  }
}

function fakeEl(): HTMLElement {
  return {
    style: { backgroundImage: '', backgroundPosition: '' },
    dataset: {} as DOMStringMap,
  } as HTMLElement;
}

describe('MascotController', () => {
  it('preserves the rest between idle blinks across frequent snapshots', () => {
    const clock = new FakeClock();
    const controller = new MascotController(fakeEl(), clock);
    controller.useAssets(fakeManifest(), 'data:image/png;base64,xx');
    controller.update({enabled:true,reducedMotion:false,visible:true,hasTrack:false});
    clock.advance(1100);
    for (let i = 0; i < 8; i++) {
      controller.update({hasTrack:false});
      clock.advance(250);
      expect(controller.currentFrame).toBe(0);
    }
    controller.destroy();
  });
  it('switches from the idle wait to the playing frame rate immediately', () => {
    const clock = new FakeClock();
    const controller = new MascotController(fakeEl(), clock);
    controller.useAssets(fakeManifest(), 'data:image/png;base64,xx');
    controller.update({enabled:true,reducedMotion:false,visible:true,hasTrack:false});
    clock.advance(1100);
    controller.update({hasTrack:true,playing:true});
    clock.advance(250);
    expect(controller.currentSequence).toBe('playing');
    expect(controller.currentFrame).toBe(2);
    controller.update({playing:false});
    clock.advance(1600);
    controller.update({playing:false});
    expect(clock.activeTimers).toBe(0);
    controller.destroy();
  });
  it('keeps a static poster when animation is disabled', () => {
    const el = fakeEl();
    const clock = new FakeClock();
    const controller = new MascotController(el, clock);
    controller.useAssets(fakeManifest(), 'data:image/png;base64,xx');
    controller.update({
      themeId: 'classic_cat',
      enabled: false,
      reducedMotion: false,
      playing: true,
      hasTrack: true,
      searching: false,
      visible: true,
      lyricsOnly: false,
      controlsRevealed: true,
    });
    expect(controller.currentSequence).toBe('static');
    expect(clock.activeTimers).toBe(0);
    controller.destroy();
  });

  it('does not restart playing sequence on repeated identical updates', () => {
    const el = fakeEl();
    const clock = new FakeClock();
    const controller = new MascotController(el, clock);
    controller.useAssets(fakeManifest(), 'data:image/png;base64,xx');
    const inputs = {
      themeId: 'classic_cat' as const,
      enabled: true,
      reducedMotion: false,
      playing: true,
      hasTrack: true,
      searching: false,
      visible: true,
      lyricsOnly: false,
      controlsRevealed: true,
    };
    controller.update(inputs);
    expect(controller.currentSequence).toBe('playing');
    clock.advance(250);
    const frame = controller.currentFrame;
    controller.update(inputs);
    controller.update(inputs);
    expect(controller.currentSequence).toBe('playing');
    expect(controller.currentFrame).toBe(frame);
    controller.destroy();
  });

  it('suspends work when not visible and resumes base state later', () => {
    const el = fakeEl();
    const clock = new FakeClock();
    const controller = new MascotController(el, clock);
    controller.useAssets(fakeManifest(), 'data:image/png;base64,xx');
    controller.update({
      themeId: 'classic_cat',
      enabled: true,
      reducedMotion: false,
      playing: true,
      hasTrack: true,
      searching: false,
      visible: true,
      lyricsOnly: false,
      controlsRevealed: true,
    });
    expect(clock.activeTimers).toBeGreaterThan(0);
    controller.update({ visible: false });
    expect(clock.activeTimers).toBe(0);
    expect(controller.currentSequence).toBe('static');
    controller.update({ visible: true });
    expect(controller.currentSequence).toBe('playing');
    controller.destroy();
  });
});
