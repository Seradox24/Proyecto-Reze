// Uses an existing GSMTC session and the user's enabled online search preference.
import { chromium } from '@playwright/test';
import assert from 'node:assert/strict';
const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
const page = browser
  .contexts()[0]
  .pages()
  .find((p) => p.url().includes('tauri.localhost'));
const invoke = (cmd, args = {}) =>
  page.evaluate(({ cmd, args }) => window.__TAURI_INTERNALS__.invoke(cmd, args), { cmd, args });
try {
  let state = await invoke('get_state');
  assert.equal(state.settings.network_allowed, true);
  assert(!('client_contact' in state.settings));
  await page.getByRole('button', { name: 'Abrir ajustes', exact: true }).click();
  assert.equal(await page.locator('#client-contact').count(), 0);
  assert(await page.locator('#network-allowed').isChecked());
  await page.getByRole('button', { name: 'Volver', exact: true }).first().click();
  const session = state.sessions.find((s) => /brave/i.test(s.name)) ?? state.sessions[0];
  if (session) {
    await invoke('select_session', { id: session.id });
    const deadline = Date.now() + 20000;
    do {
      await new Promise((r) => setTimeout(r, 250));
      state = await invoke('get_state');
    } while (Date.now() < deadline && (!state.track || state.lyrics_status === 'loading'));
    assert(state.track, 'Session did not publish a track');
    assert.notEqual(state.lyrics_status, 'loading', 'Search did not finish within 20 seconds');
    console.log(
      JSON.stringify({
        automatic: true,
        personal_contact: false,
        status: state.lyrics_status,
        candidates: state.candidates.length,
        error: state.error,
      }),
    );
  } else console.log(JSON.stringify({ automatic: true, personal_contact: false, sessions: 0 }));
} finally {
  await browser.close();
}
