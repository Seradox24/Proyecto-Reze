import { chromium } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import { writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
const page = browser
  .contexts()[0]
  .pages()
  .find((p) => p.url().includes('tauri.localhost'));
const invoke = (cmd, args = {}) =>
  page.evaluate(({ cmd, args }) => window.__TAURI_INTERNALS__.invoke(cmd, args), { cmd, args });
const waitFor = async (predicate) => {
  for (let i = 0; i < 60; i++) {
    const s = await invoke('get_state');
    if (predicate(s)) return s;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw Error('Timed out waiting for media state');
};
try {
  const initial = await invoke('get_state');
  const session = initial.sessions.find((s) => /brave/i.test(s.name));
  assert(session, 'YouTube Music browser session not available');
  await invoke('select_session', { id: session.id });
  await waitFor((s) => s.track && s.timing_reliable);
  await page.getByRole('button', { name: 'Abrir ajustes', exact: true }).click();
  await page.locator('summary').click();
  await page
    .locator('#local-lrc-path')
    .fill(fileURLToPath(new URL('../fixtures/timing-check.lrc', import.meta.url)));
  await page.getByRole('button', { name: 'Cargar LRC local', exact: true }).click();
  await waitFor((s) => s.lyrics?.source === 'Archivo local' && s.active_line !== null);
  const samples = [];
  for (let i = 0; i < 6; i++) {
    const s = await invoke('get_state');
    assert.equal(s.settings.network_allowed, false);
    assert.equal(s.lyrics.lines.length, 601);
    const effective = s.position_ms + s.settings.offset_ms;
    assert.equal(s.active_line, Math.min(600, Math.floor(effective / 1000)));
    const expected = s.lyrics.lines[s.active_line].text;
    assert.equal(await page.locator('#current').innerText(), expected);
    samples.push({ position_ms: s.position_ms, line: s.active_line, playing: s.playing });
    await new Promise((r) => setTimeout(r, 350));
  }
  assert(
    samples.at(-1).position_ms > samples[0].position_ms,
    'Leave YouTube Music playing for this test',
  );
  await writeFile(
    'artifacts/youtube-lrc-check.json',
    JSON.stringify(
      {
        date: new Date().toISOString(),
        source: 'Windows GSMTC / Brave (YouTube Music confirmed by user)',
        network: false,
        samples,
      },
      null,
      2,
    ),
  );
  console.log(
    JSON.stringify({
      passed: true,
      checks: [
        'GSMTC session selected',
        'LRC imported through UI path form',
        'Six positions matched their active LRC line and visible text',
        'Clock advanced between Windows samples',
        'Network remained disabled',
      ],
    }),
  );
} finally {
  await browser.close();
}
