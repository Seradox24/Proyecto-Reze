import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
const page = browser
  .contexts()[0]
  .pages()
  .find((p) => p.url().includes('tauri.localhost'));
const invoke = (cmd, args = {}) =>
  page.evaluate(({ cmd, args }) => window.__TAURI_INTERNALS__.invoke(cmd, args), { cmd, args });
const initial = await invoke('get_state');
const fixture = spawn('target/debug/examples/gsmtc_fixture.exe', [], {
  stdio: ['pipe', 'pipe', 'pipe'],
  windowsHide: true,
});
fixture.stdin.on('error', () => {});
const waitFor = async (predicate) => {
  for (let i = 0; i < 180; i++) {
    const s = await invoke('get_state');
    if (predicate(s)) return s;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw Error('Condition timed out');
};
try {
  await invoke('update_settings', { settings: { ...initial.settings, network_allowed: true } });
  let state = await waitFor((s) => s.sessions.some((x) => /gsmtc_fixture/i.test(x.name)));
  await invoke('select_session', {
    id: state.sessions.find((x) => /gsmtc_fixture/i.test(x.name)).id,
  });
  state = await waitFor((s) => s.track?.artist === 'Proyecto Reze');
  const original = structuredClone(state.track);
  await page.getByRole('button', { name: 'Corregir búsqueda', exact: true }).click();
  await page.locator('#search-title').fill('HE-MAN Y Los Amos Del Universo Intro Cover!');
  await page.locator('#search-artist').fill('The Covers Duo');
  await page.getByRole('button', { name: 'Buscar con estos datos' }).click();
  state = await waitFor((s) => s.search_override && s.lyrics_status !== 'loading');
  assert.equal(state.lyrics_status, 'ready', state.error ?? 'No results');
  assert(state.candidates.length > 1);
  assert(state.selected_candidate_id);
  assert(state.lyrics);
  assert.equal(state.sync_confirmed, false);
  assert(state.selection_notice.includes('letras'));
  assert.deepEqual(state.track, original);
  await page.getByRole('button', { name: `Versiones (${state.candidates.length})` }).click();
  assert((await page.locator('#candidates').innerText()).includes('Seleccionada'));
  await page.screenshot({ path: 'artifacts/he-man-alternatives.png' });
  await invoke('select_session', { id: null });
  state = await invoke('get_state');
  assert.equal(state.search_override, null);
  assert.equal(state.translation, null);
  await assert.rejects(() =>
    invoke('correct_search', {
      title: 'Old',
      artist: 'Old',
      generation: state.generation - 1,
      reset: false,
    }),
  );
  await writeFile(
    'artifacts/search-controls-smoke.json',
    JSON.stringify(
      {
        date: new Date().toISOString(),
        manual_override: true,
        original_metadata_preserved: true,
        multiple_results_defaulted: true,
        uncertain_timing_disabled: true,
        track_reset_clears_override: true,
      },
      null,
      2,
    ),
  );
  console.log(
    'Native search controls passed; HE-MAN results are explicitly other artists, not the exact cover.',
  );
} finally {
  await invoke('select_session', { id: null }).catch(() => {});
  await invoke('update_settings', { settings: initial.settings }).catch(() => {});
  fixture.stdin.end('quit\n');
  fixture.kill();
  await browser.close();
}
