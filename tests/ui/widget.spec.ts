import { test, expect } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const callbacks = new Map<number, (event: unknown) => void>();
    const listeners = new Map<string, number>();
    let id = 0;
    const commands: { command: string; args: Record<string, any> }[] = [];
    const state = {
      revision: 1,
      generation: 1,
      sessions: [{ id: 'one', name: 'Reproductor de prueba' }],
      selected_session: 'one',
      track: {
        title: 'Canción de prueba con un título muy largo para comprobar el espacio disponible',
        artist: 'Autor de prueba',
        album: 'Demo',
        duration_ms: 120000,
      },
      playing: true,
      position_ms: 30000,
      timing_reliable: true,
      artwork_revision: 0,
      has_artwork: false,
      lyrics: {
        source: 'Prueba',
        source_id: null,
        format: 'line_synced',
        lines: [
          { at_ms: 0, text: 'Una línea anterior' },
          { at_ms: 30000, text: 'La música sigue aquí' },
          { at_ms: 60000, text: 'Una línea que viene después' },
        ],
        plain_text: '',
        warnings: [],
      },
      active_line: 1,
      lyrics_status: 'ready',
      candidates: [],
      error: null,
      settings: {
        schema_version: 1,
        network_allowed: false,
        offset_ms: 0,
        font_size: 23,
        opacity: 1,
        alignment: 'center',
        reduced_motion: false,
        z_order: 'normal',
        view_mode: 'compact',
        x: null,
        y: null,
        width: 420,
        height: 260,
      },
    };
    const emit = () => {
      state.revision++;
      const callback = callbacks.get(listeners.get('proyecto-reze:state')!);
      callback?.({ payload: structuredClone(state) });
    };
    Object.assign(window, {
      __testCommands: commands,
      __testGetState: () => structuredClone(state),
      __testPosition: (tick: unknown) =>
        callbacks.get(listeners.get('proyecto-reze:position')!)?.({ payload: tick }),
      __testSetState: (patch: Record<string, unknown>) => {
        Object.assign(state, patch);
        emit();
      },
      __TAURI_INTERNALS__: {
        transformCallback(callback: (event: unknown) => void) {
          callbacks.set(++id, callback);
          return id;
        },
        async invoke(command: string, args: Record<string, any>) {
          commands.push({ command, args });
          if (command === 'translate_lyrics') {
            Object.assign(state, {
              translation: structuredClone(state.lyrics),
              translation_status: 'ready',
            });
            (state as any).translation.lines[1].text = 'La música sigue aquí en español';
            emit();
          }
          if (command === 'plugin:event|listen') {
            listeners.set(args.event, args.handler);
            return args.handler;
          }
          if (command === 'get_state') return structuredClone(state);
          if (command === 'get_artwork')
            return args.revision === 1 ? 'data:image/png;base64,iVBORw0KGgo=' : null;
          if (command === 'update_settings') {
            state.settings = args.settings;
            emit();
          }
          if (command === 'select_session') {
            state.selected_session = args.id;
            emit();
          }
        },
      },
    });
  });
  await page.goto('/');
});
test('renders active line, neighbors, and long metadata without overflow', async ({ page }) => {
  await expect(page.locator('#current')).toHaveText('La música sigue aquí');
  await expect(page.locator('#previous')).toHaveText('Una línea anterior');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: 'artifacts/widget-compact.png' });
});
test('settings, keyboard recovery, preview and validation', async ({ page }) => {
  await page.getByRole('button', { name: 'Abrir ajustes' }).click();
  await expect(page.locator('#sessions')).toHaveValue('one');
  await page.locator('#network-allowed').check();
  await page.getByRole('button', { name: 'Guardar ajustes' }).click();
  await expect(page.locator('#settings-panel')).toBeHidden();
  await page.getByRole('button', { name: 'Abrir ajustes' }).click();
  await expect(page.locator('#network-allowed')).toBeChecked();
  await expect(page.locator('#client-contact')).toHaveCount(0);
  await page.locator('#font-size').fill('30');
  await page.getByRole('button', { name: 'Guardar ajustes' }).click();
  await expect(page.locator('#settings-panel')).toBeHidden();
  await expect(page.locator('#current')).toHaveCSS('font-size', '30px');
  await page.getByRole('button', { name: 'Abrir ajustes' }).click();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Abrir ajustes' })).toBeFocused();
});
test('transparent lyrics view remains recoverable and supports small windows', async ({ page }) => {
  await page.getByRole('button', { name: 'Cambiar a solo letras' }).click();
  await expect(page.locator('body')).toHaveClass('lyrics-only');
  await page.setViewportSize({ width: 300, height: 180 });
  await expect(page.getByRole('button', { name: 'Abrir ajustes' })).toBeVisible();
  await page.screenshot({ path: 'artifacts/widget-lyrics.png', omitBackground: true });
});
test('uncertain cover result shows artist and duration warning before selection', async ({
  page,
}) => {
  await page.evaluate(() =>
    (window as any).__testSetState({
      lyrics: null,
      lyrics_status: 'missing',
      candidates: [
        {
          id: 12756272,
          trackName: 'Gamushara',
          artistName: 'The Covers Duo',
          albumName: '',
          duration: 196,
          matchNote: 'Duración distinta por 20 s; confirma esta versión.',
        },
      ],
    }),
  );
  await expect(page.locator('#current')).toHaveText('Encontramos una posible letra');
  await page.getByRole('button', { name: 'Versiones (1)', exact: true }).click();
  await expect(page.locator('#candidates')).toContainText('The Covers Duo');
  await expect(page.locator('#candidates')).toContainText('20 s');
  await page.screenshot({ path: 'artifacts/lyrics-candidate-warning.png' });
});
test('manual title and artist correction submits generation-bound search', async ({ page }) => {
  await page.getByRole('button', { name: 'Corregir búsqueda', exact: true }).click();
  await page.locator('#search-title').fill('He-Man');
  await page.locator('#search-artist').fill('The Covers Duo');
  await page.getByRole('button', { name: 'Buscar con estos datos' }).click();
  const sent = await page.evaluate(() =>
    (window as any).__testCommands.find((c: any) => c.command === 'correct_search'),
  );
  expect(sent.args).toEqual({
    title: 'He-Man',
    artist: 'The Covers Duo',
    generation: 1,
    reset: false,
  });
});
test('translation is opt-in and original remains available', async ({ page }) => {
  await page.evaluate(() =>
    (window as any).__testSetState({
      language: 'en',
      lyrics_revision: 1,
      translation: null,
      translation_status: 'idle',
    }),
  );
  expect(
    await page.evaluate(
      () =>
        (window as any).__testCommands.filter((c: any) => c.command === 'translate_lyrics').length,
    ),
  ).toBe(0);
  await page.getByRole('button', { name: 'Traducir al español', exact: true }).click();
  await expect(page.locator('#translation-panel')).toContainText('MyMemory');
  expect(
    await page.evaluate(
      () =>
        (window as any).__testCommands.filter((c: any) => c.command === 'translate_lyrics').length,
    ),
  ).toBe(0);
  await page.getByRole('button', { name: 'Enviar a MyMemory y traducir' }).click();
  await expect(page.locator('#current')).toHaveText('La música sigue aquí en español');
  await page.getByRole('button', { name: 'Ver original' }).click();
  await expect(page.locator('#current')).toHaveText('La música sigue aquí');
});
test('multiple results show default selection and retain access to alternatives', async ({
  page,
}) => {
  await page.evaluate(() =>
    (window as any).__testSetState({
      selected_candidate_id: 2,
      sync_confirmed: true,
      selection_notice: 'Encontramos 2 letras. Seleccionada por defecto.',
      candidates: [
        { id: 2, trackName: 'Song', artistName: 'Author', albumName: '', duration: 120 },
        { id: 3, trackName: 'Song', artistName: 'Author', albumName: '', duration: 121 },
      ],
    }),
  );
  await expect(page.locator('#selection-alert')).toContainText('2 letras');
  await page.getByRole('button', { name: 'Versiones (2)' }).click();
  await expect(page.locator('#candidates')).toContainText('Seleccionada · Song');
});
test('karaoke words follow source times, pause, seek and ignore stale position packets', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.evaluate(() => {
    const current = (window as any).__testGetState();
    (window as any).__testSetState({
      settings: { ...current.settings, karaoke_mode: 'word' },
      lyrics_revision: 10,
      position_sequence: 10,
      position_ms: 1500,
      active_line: 0,
      sync_confirmed: true,
      lyrics: {
        source: 'Prueba propia',
        format: 'line_synced',
        source_id: 1,
        lines: [
          {
            at_ms: 1000,
            end_ms: 3500,
            text: 'Hola mundo',
            words: [
              { text: 'Hola ', start_ms: 1000, end_ms: 2000 },
              { text: 'mundo', start_ms: 2000, end_ms: 3000 },
            ],
          },
        ],
        plain_text: '',
        warnings: [],
      },
    });
  });
  await expect(page.locator('.karaoke-word')).toHaveCount(2);
  await expect(page.locator('.karaoke-word').first()).toHaveAttribute('data-progress', '0.5');
  await expect(page.locator('.karaoke-word').last()).toHaveAttribute('data-progress', '0');
  await page.screenshot({ path: 'artifacts/karaoke-words.png' });
  await page.evaluate(() =>
    (window as any).__testPosition({
      generation: 1,
      lyrics_revision: 10,
      sequence: 11,
      position_ms: 2500,
      active_line: 0,
      playing: false,
      timing_reliable: true,
    }),
  );
  await expect(page.locator('.karaoke-word').last()).toHaveAttribute('data-progress', '0.5');
  await page.waitForTimeout(200);
  await expect(page.locator('.karaoke-word').last()).toHaveAttribute('data-progress', '0.5');
  await page.evaluate(() =>
    (window as any).__testPosition({
      generation: 1,
      lyrics_revision: 10,
      sequence: 12,
      position_ms: 1200,
      active_line: 0,
      playing: true,
      timing_reliable: true,
    }),
  );
  await expect(page.locator('.karaoke-word').first()).toHaveAttribute('data-progress', '0.2');
  await page.evaluate(() =>
    (window as any).__testPosition({
      generation: 1,
      lyrics_revision: 10,
      sequence: 11,
      position_ms: 2800,
      active_line: 0,
      playing: true,
      timing_reliable: true,
    }),
  );
  await expect(page.locator('.karaoke-word').first()).toHaveAttribute('data-progress', '0.2');
  await page.getByRole('button', { name: 'Abrir ajustes' }).click();
  await page.locator('#karaoke-mode').selectOption('line');
  await page.getByRole('button', { name: 'Guardar ajustes' }).click();
  await expect(page.locator('.karaoke-word')).toHaveCount(0);
  await expect(page.locator('#current')).toHaveText('Hola mundo');
});
test('word preference persists but LRC without words keeps line mode', async ({ page }) => {
  await page.getByRole('button', { name: 'Abrir ajustes' }).click();
  await page.locator('#karaoke-mode').selectOption('word');
  await page.getByRole('button', { name: 'Guardar ajustes' }).click();
  await expect(page.locator('#karaoke-badge')).toHaveText('Línea');
  await expect(page.locator('.karaoke-word')).toHaveCount(0);
  await page.getByRole('button', { name: 'Abrir ajustes' }).click();
  await expect(page.locator('#karaoke-mode')).toHaveValue('word');
  await expect(page.locator('#karaoke-status')).toContainText('Se mantiene el modo por línea');
});
test('untimed lyrics have a prominent persistent notice and neutral text', async ({ page }) => {
  await page.evaluate(() => {
    const s = (window as any).__testGetState();
    (window as any).__testSetState({
      settings: { ...s.settings, karaoke_mode: 'word' },
      sync_confirmed: true,
      lyrics_revision: 12,
      lyrics: {
        source: 'Prueba propia',
        source_id: null,
        format: 'plain',
        lines: [],
        plain_text: Array(40).fill('Texto de prueba para leer sin seguimiento.').join('\n'),
        warnings: [],
      },
    });
  });
  await expect(page.locator('#reading-label')).toHaveText('Modo lectura · sin tiempos');
  await expect(page.locator('#reading-label')).toHaveCSS('font-weight', '700');
  await expect(page.locator('#reading-status')).toBeVisible();
  await expect(page.locator('#reading-detail')).toContainText('sin seguimiento ni resaltado');
  await expect(page.locator('#plain-text')).toHaveCSS('color', 'rgb(213, 220, 210)');
  await expect(page.locator('#plain-text')).toHaveCSS('background-image', 'none');
  await expect(page.locator('#lyrics')).toBeHidden();
  await expect(page.locator('.karaoke-word')).toHaveCount(0);
  await expect(page.locator('#karaoke-badge')).toHaveText('Lectura');
  expect(
    await page.locator('#plain-text').evaluate((element) => element.clientHeight),
  ).toBeGreaterThanOrEqual(38);
  const before = await page.locator('#reading-status').boundingBox();
  await page.locator('#plain-text').evaluate((element) => {
    element.scrollTop = 300;
  });
  expect(await page.locator('#reading-status').boundingBox()).toEqual(before);
  await page.screenshot({ path: 'artifacts/reading-no-timing.png' });
  await page.getByRole('button', { name: 'Cambiar a solo letras' }).click();
  await expect(page.locator('#reading-status')).toBeVisible();
  await expect(page.locator('#plain-text')).toHaveCSS('background-image', 'none');
  await page.setViewportSize({ width: 300, height: 180 });
  await expect(page.locator('#reading-label')).toBeInViewport();
  await page.screenshot({ path: 'artifacts/reading-no-timing-small.png', omitBackground: true });
});
test('position ticks move the clock and lines without a full snapshot', async ({ page }) => {
  await expect(page.locator('#elapsed')).toHaveText('0:30');
  await page.evaluate(() =>
    (window as any).__testPosition({
      generation: 1,
      lyrics_revision: undefined,
      sequence: 5,
      position_ms: 61000,
      active_line: 2,
      playing: true,
      timing_reliable: true,
    }),
  );
  await expect(page.locator('#elapsed')).toHaveText('1:01');
  await expect(page.locator('#current')).toHaveText('Una línea que viene después');
  expect(await page.locator('#progress').evaluate((p: HTMLProgressElement) => p.value)).toBe(61000);
});
test('artwork is requested once per revision and hidden when the track has none', async ({
  page,
}) => {
  const requests = () =>
    page.evaluate(
      () => (window as any).__testCommands.filter((c: any) => c.command === 'get_artwork').length,
    );
  await expect(page.locator('#artwork')).toBeHidden();
  expect(await requests()).toBe(0);
  await page.evaluate(() =>
    (window as any).__testSetState({ artwork_revision: 1, has_artwork: true }),
  );
  await expect(page.locator('#artwork')).toBeVisible();
  await page.evaluate(() => (window as any).__testSetState({ playing: false }));
  await expect(page.locator('#playback')).toHaveText('En pausa');
  expect(await requests()).toBe(1);
  await page.evaluate(() =>
    (window as any).__testSetState({ artwork_revision: 2, has_artwork: false }),
  );
  await expect(page.locator('#artwork')).toBeHidden();
  expect(await requests()).toBe(1);
});
const withTheme = (page: import('@playwright/test').Page, patch: Record<string, unknown> = {}) =>
  page.evaluate((patch) => {
    const state = (window as any).__testGetState();
    (window as any).__testSetState({
      ...patch,
      settings: { ...state.settings, theme_id: 'mochi_cat', ...(patch.settings as object) },
    });
  }, patch);
test('classic stays the default and an unknown skin falls back to it', async ({ page }) => {
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'classic');
  await expect(page.locator('#mascot')).toBeHidden();
  await expect(page.locator('.brand-cat')).toBeHidden();
  await withTheme(page, { settings: { theme_id: 'from_the_future' } });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'classic');
});
test('cat skin restyles the widget and its mascot follows playback', async ({ page }) => {
  await withTheme(page);
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'mochi_cat');
  await expect(page.locator('.brand-cat')).toBeVisible();
  await expect(page.locator('.brand-classic')).toBeHidden();
  await expect(page.locator('#mascot')).toBeVisible();
  await expect(page.locator('#mascot .cat-eyes-open')).toBeVisible();
  await expect(page.locator('#mascot .cat-zzz')).toBeHidden();
  expect(
    await page.locator('#progress-wrap').evaluate((e) => e.style.getPropertyValue('--progress')),
  ).toBe('0.2500');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: 'artifacts/skin-mochi-playing.png' });
  await withTheme(page, { playing: false });
  await expect(page.locator('#mascot .cat-eyes-closed')).toBeVisible();
  await expect(page.locator('#mascot .cat-eyes-open')).toBeHidden();
  await page.screenshot({ path: 'artifacts/skin-mochi-paused.png' });
  await withTheme(page, { track: null, lyrics: null, playing: false });
  await expect(page.locator('#mascot')).toBeVisible();
  await page.screenshot({ path: 'artifacts/skin-mochi-empty.png' });
});
test('cat skin is previewed from settings, saved with the form and usable when small', async ({
  page,
}) => {
  await page.getByRole('button', { name: 'Abrir ajustes' }).click();
  await page.locator('#theme').selectOption('mochi_cat');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'mochi_cat');
  await page.screenshot({ path: 'artifacts/skin-mochi-settings.png' });
  await page.getByRole('button', { name: 'Volver' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'classic');
  await page.getByRole('button', { name: 'Abrir ajustes' }).click();
  await page.locator('#theme').selectOption('mochi_cat');
  await page.getByRole('button', { name: 'Guardar ajustes' }).click();
  const saved = await page.evaluate(() =>
    (window as any).__testCommands.findLast((c: any) => c.command === 'update_settings'),
  );
  expect(saved.args.settings.theme_id).toBe('mochi_cat');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'mochi_cat');
  await page.setViewportSize({ width: 300, height: 180 });
  await expect(page.getByRole('button', { name: 'Abrir ajustes' })).toBeVisible();
  await expect(page.locator('#current')).toBeVisible();
  await expect(page.locator('#mascot')).toBeHidden();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: 'artifacts/skin-mochi-small.png' });
  await page.setViewportSize({ width: 420, height: 260 });
  await page.getByRole('button', { name: 'Cambiar a solo letras' }).click();
  await expect(page.locator('#mascot')).toBeHidden();
  await page.screenshot({ path: 'artifacts/skin-mochi-lyrics-only.png' });
  // Controls over the desktop turn white; the ones inside a panel must stay readable.
  const color = (id: string) => page.locator(id).evaluate((e) => getComputedStyle(e).color);
  expect(await color('#correct-search')).toBe('rgb(255, 255, 255)');
  await page.getByRole('button', { name: 'Abrir ajustes' }).click();
  expect(await color('#close-settings')).not.toBe('rgb(255, 255, 255)');
  await page.screenshot({ path: 'artifacts/skin-mochi-lyrics-only-settings.png' });
});
