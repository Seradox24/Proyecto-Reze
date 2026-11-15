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
const fixture = spawn('target/debug/examples/gsmtc_fixture.exe', [], {
  stdio: ['pipe', 'pipe', 'pipe'],
  windowsHide: true,
});
fixture.stdin.on('error', () => {});
fixture.stdout.on('data', (data) => console.log(String(data).trim()));
fixture.stderr.on('data', (data) => console.error(String(data).trim()));
const checks = [];
const waitFor = async (predicate) => {
  const end = Date.now() + 10000;
  while (Date.now() < end) {
    const state = await invoke('get_state');
    if (predicate(state)) return state;
    await new Promise((r) => setTimeout(r, 150));
  }
  throw new Error('GSMTC condition timed out');
};
try {
  const initial = await invoke('get_state');
  assert.equal(initial.settings.network_allowed, false, 'Keep network disabled during this test');
  const listing = await waitFor((s) =>
    s.sessions.some((session) => /gsmtc_fixture/i.test(session.name)),
  );
  const synthetic = listing.sessions.find((s) => /gsmtc_fixture/i.test(s.name));
  await invoke('select_session', { id: synthetic.id });
  let state = await waitFor((s) => s.track?.title === 'Proyecto Reze — prueba propia');
  assert.equal(state.timing_reliable, true);
  checks.push('Windows enumerates synthetic SMTC session and reads track/timeline');
  await invoke('import_lrc_path', {
    path: new URL('../fixtures/demo.lrc', import.meta.url).pathname.replace(/^\/(\w:)/, '$1'),
  });
  state = await waitFor((s) => s.lyrics?.source === 'Archivo local' && s.active_line !== null);
  assert.equal(state.lyrics.lines.length, 8);
  checks.push('Local LRC loaded through the real backend without network');
  const before = state.position_ms;
  await new Promise((r) => setTimeout(r, 1100));
  state = await invoke('get_state');
  assert(state.position_ms > before);
  fixture.stdin.write('pause\n');
  state = await waitFor((s) => s.track && !s.playing);
  const paused = state.position_ms;
  await new Promise((r) => setTimeout(r, 1100));
  state = await invoke('get_state');
  assert(Math.abs(state.position_ms - paused) < 200);
  checks.push('Pause freezes the estimated clock');
  fixture.stdin.write('seek 30000\n');
  state = await waitFor(
    (s) => s.position_ms >= 30000 && s.position_ms < 30200 && s.active_line === 5,
  );
  checks.push('Seek to 30 seconds highlights the matching LRC line');
  fixture.stdin.write('play\n');
  await waitFor((s) => s.playing && s.position_ms >= 30000);
  checks.push('Seek reanchors and resume advances');
  const generation = state.generation;
  fixture.stdin.write('track Otra pista propia\n');
  state = await waitFor((s) => s.track?.title === 'Otra pista propia');
  assert(state.generation > generation);
  assert.equal(state.lyrics, null);
  checks.push('Track changes invalidate the previous generation');
  fixture.stdin.write('quit\n');
  await waitFor((s) => s.track === null);
  checks.push('Session closure degrades without switching sources');
  await writeFile(
    'artifacts/gsmtc-smoke.json',
    JSON.stringify({ date: new Date().toISOString(), checks }, null, 2),
  );
  console.log(JSON.stringify({ passed: checks.length, checks }));
} finally {
  if (fixture.exitCode === null) {
    fixture.stdin.end('quit\n');
    fixture.kill();
  }
  await invoke('select_session', { id: null }).catch(() => {});
  await browser.close();
}
