// Native playback controls through GSMTC, using only our synthetic fixture.
// Run against a temporary local WebView2 debug instance; no music-service API.
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { writeFile, mkdir } from 'node:fs/promises';
import assert from 'node:assert/strict';

const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
const page = browser
  .contexts()[0]
  .pages()
  .find((p) => p.url().includes('tauri.localhost'));
assert(page, 'Open the native application with temporary local debugging first');
const invoke = (command, args = {}) =>
  page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), {
    command,
    args,
  });
const wait = async (predicate) => {
  for (let i = 0; i < 100; i++) {
    const s = await invoke('get_state');
    if (predicate(s)) return s;
    await page.waitForTimeout(100);
  }
  throw new Error('Timed out waiting for fixture playback state');
};
const original = await invoke('get_state');
const checks = [];
let fixture;
try {
  await invoke('select_session', { id: null });
  await invoke('update_settings', {
    settings: { ...original.settings, network_allowed: false, view_mode: 'compact' },
  });
  fixture = spawn(
    fileURLToPath(new URL('../target/debug/examples/gsmtc_fixture.exe', import.meta.url)),
    [],
    { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true },
  );
  fixture.stdin.on('error', () => {});
  let state = await wait((s) => s.sessions.some((x) => /gsmtc_fixture/i.test(x.name)));
  await invoke('select_session', {
    id: state.sessions.find((x) => /gsmtc_fixture/i.test(x.name)).id,
  });
  state = await wait(
    (s) =>
      s.track?.artist === 'Proyecto Reze' &&
      s.playback_controls?.pause &&
      s.playback_controls?.next,
  );
  const initialTitle = state.track.title;
  assert.equal(state.playback_controls.previous, false);
  assert.equal(await page.locator('#transport-previous').isDisabled(), true);
  checks.push('Unsupported previous action is disabled by real GSMTC capabilities');

  await page.getByRole('button', { name: 'Pausar', exact: true }).click();
  state = await wait((s) => !s.playing);
  // GSMTC publishes playback status and the final paused timeline separately.
  await page.waitForTimeout(200);
  const frozen = (await invoke('get_state')).position_ms;
  await page.waitForTimeout(250);
  assert.equal((await invoke('get_state')).position_ms, frozen);
  checks.push('Pause button reaches the fixture and freezes the real GSMTC clock');

  await page.getByRole('button', { name: 'Reproducir', exact: true }).click();
  state = await wait((s) => s.playing);
  await invoke('control_playback', {
    sessionId: state.selected_session,
    generation: state.generation,
    action: 'play',
  });
  await wait((s) => s.playing && s.position_ms > frozen);
  checks.push('Play resumes playback and repeated explicit play does not toggle back to pause');

  await invoke('import_lrc_path', {
    path: fileURLToPath(new URL('../fixtures/demo.lrc', import.meta.url)),
  });
  const beforeNext = await invoke('get_state');
  await page.getByRole('button', { name: 'Canción siguiente', exact: true }).click();
  state = await wait(
    (s) => s.track?.title && s.track.title !== initialTitle && s.playback_controls.previous,
  );
  assert.equal(state.lyrics, null);
  assert.equal(state.playback_controls.previous, true);
  checks.push('Next changes fixture track and clears the previous local lyrics');

  await page.getByRole('button', { name: 'Canción anterior', exact: true }).click();
  state = await wait((s) => s.track?.title === initialTitle);
  checks.push('Previous returns to the original fixture track');

  await assert.rejects(
    invoke('control_playback', {
      sessionId: 'another-session',
      generation: state.generation,
      action: 'next',
    }),
  );
  await assert.rejects(
    invoke('control_playback', {
      sessionId: state.selected_session,
      generation: beforeNext.generation,
      action: 'next',
    }),
  );
  assert.equal((await invoke('get_state')).track.title, initialTitle);
  checks.push('Wrong session and stale generation commands are rejected without skipping');

  await invoke('update_settings', {
    settings: { ...(await invoke('get_state')).settings, view_mode: 'lyrics_only' },
  });
  await page.locator('#transport-play-pause').focus();
  assert.equal(await page.locator('#transport-play-pause').isVisible(), true);
  await page.getByRole('button', { name: 'Pausar', exact: true }).click();
  await wait((s) => !s.playing);
  checks.push('Playback controls also work when revealed in lyrics-only view');

  await mkdir('artifacts', { recursive: true });
  await page.screenshot({
    path: 'artifacts/transport-native.png',
    omitBackground: true,
    animations: 'disabled',
  });
  await writeFile(
    'artifacts/transport-native.json',
    JSON.stringify({ network: false, checks }, null, 2),
  );
  console.log(JSON.stringify({ passed: checks.length, checks }, null, 2));
} finally {
  await invoke('select_session', { id: null }).catch(() => {});
  await invoke('update_settings', { settings: original.settings }).catch(() => {});
  if (fixture) {
    fixture.stdin.end('quit\n');
    fixture.kill();
  }
  await browser.close();
}
