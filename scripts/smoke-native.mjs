// Run against Proyecto Reze started with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=
// --remote-debugging-port=9222. Debugging is not enabled in normal launches.
import { chromium } from '@playwright/test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';

const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
console.log('CDP connected');
const page = browser
  .contexts()[0]
  .pages()
  .find((p) => p.url().includes('tauri.localhost'));
assert(page, 'Proyecto Reze WebView not found');
const invoke = (cmd, args = {}) =>
  page.evaluate(({ cmd, args }) => window.__TAURI_INTERNALS__.invoke(cmd, args), { cmd, args });
const initial = await invoke('get_state');
console.log('Native state received');
const checks = [];
try {
  assert.equal(initial.settings.network_allowed, false, 'Run smoke with network disabled');
  assert.equal(initial.error, null, initial.error ?? '');
  assert.equal(initial.track, null);
  checks.push('Native startup without a selected session; network disabled');
  for (const z_order of ['top', 'bottom', 'normal']) {
    console.log('Checking z-order', z_order);
    await invoke('update_settings', { settings: { ...initial.settings, z_order } });
    assert.equal((await invoke('get_state')).settings.z_order, z_order);
  }
  checks.push('Native top / bottom / normal calls succeeded');
  await invoke('update_settings', { settings: { ...initial.settings, view_mode: 'lyrics_only' } });
  await page.locator('body.lyrics-only').waitFor();
  await page.getByRole('button', { name: 'Abrir ajustes' }).click();
  await page.locator('#settings-panel').waitFor();
  await page.keyboard.press('Escape');
  checks.push('Transparent view and keyboard recovery');
  await invoke('reset_window');
  checks.push('Native monitor position recovery');
  await invoke('update_settings', { settings: initial.settings });
  await page.locator('body:not(.lyrics-only)').waitFor();
  await mkdir('artifacts', { recursive: true });
  await page.screenshot({ path: 'artifacts/native-startup.png' });
  await writeFile(
    'artifacts/native-smoke.json',
    JSON.stringify({ date: new Date().toISOString(), checks }, null, 2),
  );
  console.log(JSON.stringify({ passed: checks.length, checks }));
} finally {
  await invoke('update_settings', { settings: initial.settings }).catch(() => {});
  await browser.close();
}
