// Offline native check: actual GSMTC clock + owned Lyricsfile payload + production renderer.
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
const page = browser
  .contexts()[0]
  .pages()
  .find((p) => p.url().includes('tauri.localhost'));
await page.emulateMedia({ reducedMotion: 'no-preference' });
const invoke = (cmd, args = {}) =>
  page.evaluate(({ cmd, args }) => window.__TAURI_INTERNALS__.invoke(cmd, args), { cmd, args });
const original = await invoke('get_state');
const fixture = spawn('target/debug/examples/gsmtc_fixture.exe', [], {
  stdio: ['pipe', 'pipe', 'pipe'],
  windowsHide: true,
});
fixture.stdin.on('error', () => {});
const waitFor = async (predicate) => {
  let s;
  for (let i = 0; i < 100; i++) {
    s = await invoke('get_state');
    if (predicate(s)) return s;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw Error(
    `Native state timed out: ${JSON.stringify({ playing: s.playing, position: s.position_ms, line: s.active_line, reliable: s.timing_reliable, confirmed: s.sync_confirmed, lyrics: s.lyrics?.format, words: s.lyrics?.lines.map((l) => l.words?.length), error: s.error })}`,
  );
};
const progress = () =>
  page
    .locator('.karaoke-word')
    .evaluateAll((words) => words.map((w) => Number(w.dataset.progress)));
const near = (value, target) => Math.abs(value - target) < 0.02;
const checks = [];
try {
  await invoke('update_settings', {
    settings: {
      ...original.settings,
      network_allowed: false,
      karaoke_mode: 'word',
      offset_ms: 0,
      reduced_motion: false,
    },
  });
  let s = await waitFor((s) => s.sessions.some((session) => /gsmtc_fixture/i.test(session.name)));
  await invoke('select_session', {
    id: s.sessions.find((session) => /gsmtc_fixture/i.test(session.name)).id,
  });
  await waitFor((s) => s.track?.artist === 'Proyecto Reze');
  await invoke('import_lrc_path', {
    path: fileURLToPath(new URL('../fixtures/karaoke-demo.lyricsfile.yaml', import.meta.url)),
  });
  fixture.stdin.write('pause\nseek 1500\n');
  await waitFor((s) => !s.playing && s.position_ms === 1500 && s.active_line === 0);
  await page.waitForFunction(() => document.querySelectorAll('.karaoke-word').length === 2);
  let values = await progress();
  assert(near(values[0], 0.5));
  assert.equal(values[1], 0);
  checks.push('Validated Lyricsfile word boundaries rendered at the GSMTC position');
  await new Promise((r) => setTimeout(r, 200));
  assert(near((await progress())[0], 0.5));
  checks.push('Pause freezes fill');
  fixture.stdin.write('seek 2500\n');
  await waitFor((s) => s.position_ms === 2500);
  await page.waitForFunction(
    () => Number(document.querySelectorAll('.karaoke-word')[1]?.dataset.progress) > 0.48,
  );
  values = await progress();
  assert.equal(values[0], 1);
  assert(near(values[1], 0.5));
  checks.push('Forward seek recalculates active word');
  fixture.stdin.write('seek 1200\n');
  await waitFor((s) => s.position_ms === 1200);
  await page.waitForFunction(
    () => Number(document.querySelector('.karaoke-word')?.dataset.progress) < 0.22,
  );
  values = await progress();
  assert(near(values[0], 0.2));
  assert.equal(values[1], 0);
  checks.push('Backward seek clears future highlights');
  fixture.stdin.write('play\n');
  await waitFor((s) => s.playing);
  const before = (await progress())[0];
  await new Promise((r) => setTimeout(r, 150));
  assert((await progress())[0] > before);
  checks.push('Lightweight position events advance karaoke between full snapshots');
  fixture.stdin.write('pause\nseek 1500\n');
  await waitFor((s) => !s.playing && s.position_ms === 1500);
  s = await invoke('get_state');
  await invoke('update_settings', { settings: { ...s.settings, karaoke_mode: 'line' } });
  await page.waitForFunction(() => document.querySelectorAll('.karaoke-word').length === 0);
  checks.push('Line selector disables word highlighting');
  s = await invoke('get_state');
  await invoke('update_settings', { settings: { ...s.settings, karaoke_mode: 'word' } });
  await invoke('import_lrc_path', {
    path: fileURLToPath(new URL('../fixtures/demo.lrc', import.meta.url)),
  });
  await page.waitForFunction(
    () =>
      document.querySelectorAll('.karaoke-word').length === 0 &&
      document.querySelector('#karaoke-badge')?.textContent === 'Línea',
  );
  checks.push('LRC without words automatically falls back to line mode');
  await writeFile(
    'artifacts/karaoke-smoke.json',
    JSON.stringify({ date: new Date().toISOString(), network: false, checks }, null, 2),
  );
  console.log(JSON.stringify({ passed: checks.length, checks }));
} finally {
  await invoke('select_session', { id: null }).catch(() => {});
  await invoke('update_settings', { settings: original.settings }).catch(() => {});
  fixture.stdin.end('quit\n');
  fixture.kill();
  await browser.close();
}
