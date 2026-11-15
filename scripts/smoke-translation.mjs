// Explicit live integration probe: only the project's own two test sentences are sent.
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
const page = browser
  .contexts()[0]
  .pages()
  .find((p) => p.url().includes('tauri.localhost'));
const invoke = (cmd, args = {}) =>
  page.evaluate(({ cmd, args }) => window.__TAURI_INTERNALS__.invoke(cmd, args), { cmd, args });
const fixture = spawn('target/debug/examples/gsmtc_fixture.exe', [], {
  stdio: ['pipe', 'pipe', 'pipe'],
  windowsHide: true,
});
fixture.stdin.on('error', () => {});
const initial = await invoke('get_state');
const waitFor = async (predicate) => {
  for (let i = 0; i < 180; i++) {
    const state = await invoke('get_state');
    if (predicate(state)) return state;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw Error('Native condition timed out');
};
try {
  await invoke('update_settings', { settings: { ...initial.settings, network_allowed: false } });
  let state = await waitFor((s) => s.sessions.some((x) => /gsmtc_fixture/i.test(x.name)));
  await invoke('select_session', {
    id: state.sessions.find((x) => /gsmtc_fixture/i.test(x.name)).id,
  });
  await waitFor((s) => s.track?.artist === 'Proyecto Reze');
  await invoke('import_lrc_path', {
    path: fileURLToPath(new URL('../fixtures/translation-demo.lrc', import.meta.url)),
  });
  await invoke('update_settings', { settings: { ...initial.settings, network_allowed: true } });
  state = await invoke('get_state');
  assert.equal(state.translation, null);
  const original = structuredClone(state.lyrics);
  await invoke('translate_lyrics', {
    source: 'en',
    generation: state.generation,
    lyricsRevision: state.lyrics_revision,
  });
  state = await waitFor(
    (s) => s.translation_status === 'ready' || s.translation_status === 'error',
  );
  assert.equal(state.translation_status, 'ready', state.error ?? 'Translation did not finish');
  assert.deepEqual(state.lyrics, original);
  assert.equal(state.translation.lines.length, 2);
  assert.deepEqual(
    state.translation.lines.map((x) => x.at_ms),
    original.lines.map((x) => x.at_ms),
  );
  assert(state.translation.lines[0].text.length > 0);
  assert.notEqual(state.translation.lines[0].text, original.lines[0].text);
  await writeFile(
    'artifacts/translation-smoke.json',
    JSON.stringify(
      {
        date: new Date().toISOString(),
        provider: 'MyMemory',
        material: 'Owned two-line fixture',
        original_preserved: true,
        timestamps_preserved: true,
      },
      null,
      2,
    ),
  );
  console.log('Live translation passed: original and timestamps preserved.');
} finally {
  await invoke('select_session', { id: null }).catch(() => {});
  await invoke('update_settings', { settings: initial.settings }).catch(() => {});
  fixture.stdin.end('quit\n');
  fixture.kill();
  await browser.close();
}
