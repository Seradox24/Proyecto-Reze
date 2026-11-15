import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Settings, Snapshot, PositionTick, ThemeId, LyricsBackdrop, PlaybackAction } from './types';
import { DEFAULT_SETTINGS, normalizeLyricsBackdrop, normalizeThemeId } from './types';
import { lyricView, time, readingStatus, readingText } from './view';
import { canHighlightWords, modeStatus, WordRenderer } from './karaoke';
import { MascotController } from './mascot';
import './style.css';

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const input = (id: string) => $<HTMLInputElement>(id);
const select = (id: string) => $<HTMLSelectElement>(id);
const native = '__TAURI_INTERNALS__' in window;
let state: Snapshot;
const wordRenderer = new WordRenderer();
const mascot = new MascotController($('mascot'));
let sessionSignature = '';
let candidateSignature = '';
let documentSignature = '';
let saving = false;
let translated = false;
let displayedRevision = -1;
let correctionGeneration = 0;
let translationGeneration = 0;
let translationRevision = 0;
let noticeTimer: ReturnType<typeof setTimeout>;
let focusReturn: HTMLElement | null = null;
let previewSettings: Settings | null = null;
let transportBusy = false;

function renderTransport() {
  const active = !!state?.selected_session && !!state?.track;
  const toggleAction = state?.playing ? 'pause' : 'play';
  const toggleLabel = state?.playing ? 'Pausar' : 'Reproducir';
  const controls: [string, PlaybackAction, string][] = [
    ['transport-previous','previous','Canción anterior'],
    ['transport-play-pause',toggleAction,toggleLabel],
    ['transport-next','next','Canción siguiente'],
  ];
  for (const [id,action,label] of controls) {
    const button = $<HTMLButtonElement>(id);
    const supported = active && !!state.playback_controls?.[action];
    button.disabled = !supported || transportBusy;
    button.setAttribute('aria-label',label);
    button.title = !active ? 'Selecciona una sesión con una pista.' : transportBusy
      ? 'Esperando al reproductor…' : !supported ? `${label}: esta sesión no admite este control.` : label;
  }
  $('transport').setAttribute('aria-busy',String(transportBusy));
  // SVGElement does not reflect HTMLElement.hidden; update the attribute.
  $('transport-play-icon').toggleAttribute('hidden',!!state?.playing);
  $('transport-pause-icon').toggleAttribute('hidden',!state?.playing);
}

async function controlPlayback(action: PlaybackAction) {
  if (transportBusy || !state.selected_session || !state.track || !state.playback_controls?.[action]) return;
  const sessionId = state.selected_session;
  const generation = state.generation;
  transportBusy = true;
  renderTransport();
  try {
    await command('control_playback',{sessionId,generation,action});
    // Windows supplies the actual state; do not optimistically move the clock,
    // change the track, or infer that a request has already taken effect.
    render(await command<Snapshot>('get_state'));
  } catch (error) {
    notice(String(error));
  } finally {
    transportBusy = false;
    renderTransport();
  }
}

$('transport-previous').onclick = () => { void controlPlayback('previous'); };
$('transport-play-pause').onclick = () => { void controlPlayback(state.playing ? 'pause' : 'play'); };
$('transport-next').onclick = () => { void controlPlayback('next'); };

const THEME_HELP: Record<ThemeId, string> = {
  classic_cat: 'Clásico con gato conserva la identidad habitual sobre verde carbón.',
  window_cat: 'Gatito en la ventana aporta un tono cálido de atardecer.',
  cat_notebook: 'Cuaderno de gatitos es una presencia clara y amable.',
  cat_constellation: 'Constelación felina destaca la letra sobre azul noche.',
};

function notice(message: string) {
  $('notice').textContent = message;
  $('notice').hidden = false;
  clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => { $('notice').hidden = true; }, 7000);
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  if (native) return invoke<T>(name, args);
  throw new Error('Esta acción necesita la aplicación de escritorio.');
}

async function action(name: string, args?: Record<string, unknown>) {
  try { return await command(name, args); } catch (error) { notice(String(error)); }
}

function effectiveSettings(): Settings {
  return previewSettings ?? state.settings;
}

function applyAppearance(settings: Settings) {
  const theme = normalizeThemeId(settings.theme_id);
  const backdrop = normalizeLyricsBackdrop(settings.lyrics_backdrop);
  document.documentElement.dataset.theme = theme;
  document.documentElement.dataset.backdrop = backdrop;
  document.documentElement.style.setProperty('--lyric-size', `${settings.font_size}px`);
  document.documentElement.style.setProperty('--opacity', String(settings.opacity));
  document.documentElement.style.setProperty('--alignment', settings.alignment);
  document.body.classList.toggle('lyrics-only', settings.view_mode === 'lyrics_only');
  document.body.classList.toggle('reduced-motion', settings.reduced_motion);
  $('view-toggle').setAttribute('aria-label', settings.view_mode === 'compact' ? 'Cambiar a solo letras' : 'Cambiar a vista compacta');
  syncMascot(settings);
}

function systemReducedMotion() {
  return matchMedia('(prefers-reduced-motion: reduce)').matches;
}

function syncMascot(settings: Settings = effectiveSettings()) {
  const lyricsOnly = settings.view_mode === 'lyrics_only';
  const controlsRevealed = !lyricsOnly || document.body.classList.contains('controls-visible') || document.body.matches(':hover') || !!document.querySelector('#titlebar:focus-within, .popover:not([hidden]), .panel:not([hidden])');
  const reduced = settings.reduced_motion || systemReducedMotion();
  mascot.update({
    themeId: normalizeThemeId(settings.theme_id),
    enabled: settings.cat_animation_enabled,
    reducedMotion: reduced,
    playing: !!state?.playing,
    hasTrack: !!state?.track,
    searching: state?.lyrics_status === 'loading',
    visible: document.visibilityState !== 'hidden',
    lyricsOnly,
    controlsRevealed,
  });
  const help = $('cat-animation-help');
  if (reduced) help.textContent = 'Animaciones detenidas por movimiento reducido';
  else help.textContent = 'El gatito se mueve mientras escuchas. Puedes dejarlo quieto cuando quieras.';
}

function closePopover(id: string, returnFocus = true) {
  const el = $(id);
  if (el.hidden) return;
  el.hidden = true;
  if (id === 'lyrics-menu') $('lyrics-menu-button').setAttribute('aria-expanded', 'false');
  if (id === 'karaoke-menu') $('karaoke-badge').setAttribute('aria-expanded', 'false');
  if (id === 'cat-menu') $('cat-options').setAttribute('aria-expanded', 'false');
  if (returnFocus && focusReturn) focusReturn.focus();
}

function closeAllPopovers(returnFocus = true) {
  for (const id of ['lyrics-menu', 'karaoke-menu', 'cat-menu']) closePopover(id, false);
  if (returnFocus && focusReturn) focusReturn.focus();
}

function placePopover(popover: HTMLElement, anchor: HTMLElement) {
  popover.hidden = false;
  const rect = anchor.getBoundingClientRect();
  const pad = 8;
  requestAnimationFrame(() => {
    const width = popover.offsetWidth;
    const height = popover.offsetHeight;
    let left = rect.left;
    let top = rect.bottom + 4;
    if (left + width > window.innerWidth - pad) left = Math.max(pad, window.innerWidth - width - pad);
    if (top + height > window.innerHeight - pad) top = Math.max(pad, rect.top - height - 4);
    popover.style.left = `${left}px`;
    popover.style.top = `${top}px`;
  });
}

function openPopover(id: string, anchor: HTMLElement) {
  closeAllPopovers(false);
  focusReturn = anchor;
  const popover = $(id);
  if (id === 'lyrics-menu') $('lyrics-menu-button').setAttribute('aria-expanded', 'true');
  if (id === 'karaoke-menu') $('karaoke-badge').setAttribute('aria-expanded', 'true');
  if (id === 'cat-menu') $('cat-options').setAttribute('aria-expanded', 'true');
  placePopover(popover, anchor);
}

function hideAuxiliary() {
  for (const id of ['correction-panel', 'translation-panel', 'candidates-panel']) $(id).hidden = true;
}

function render(next: Snapshot) {
  if (state && next.revision < state.revision) return;
  if (state && next.generation === state.generation && next.lyrics_revision === state.lyrics_revision
    && (next.position_sequence ?? 0) < (state.position_sequence ?? 0)) {
    next = {
      ...next,
      position_sequence: state.position_sequence,
      position_ms: state.position_ms,
      active_line: state.active_line,
      playing: state.playing,
      timing_reliable: state.timing_reliable,
    };
  }
  const previousError = state?.error;
  state = next;
  if (displayedRevision !== (state.lyrics_revision ?? 0)) {
    translated = false;
    displayedRevision = state.lyrics_revision ?? 0;
  }
  const lyrics = translated && state.translation ? state.translation : state.lyrics;
  $('selection-alert').textContent = state.selection_notice ?? '';
  $('selection-alert').hidden = !state.selection_notice;
  $<HTMLButtonElement>('correct-search').disabled = !state.track;
  $('translate').hidden = !state.lyrics || state.lyrics.format === 'instrumental' || (state.language === 'es' && !state.translation);
  $('translate').textContent = state.translation_status === 'loading'
    ? 'Traduciendo…'
    : state.translation
      ? (translated ? 'Ver original' : 'Ver español')
      : 'Traducir al español…';
  $<HTMLButtonElement>('translate').disabled = state.translation_status === 'loading';
  $('cancel-translation').hidden = state.translation_status !== 'loading';
  $('track-title').textContent = state.track?.title || (state.track ? 'Título no disponible' : 'Tu música, a la vista.');
  $('track-artist').textContent = state.track ? state.track.artist || 'Artista no disponible' : 'Selecciona una sesión para empezar';
  $('track-title').title = state.track?.title ?? '';
  $('playback').textContent = !state.track ? 'Sin sesión' : state.playing ? 'Reproduciendo' : 'En pausa';
  $('playback').classList.toggle('playing', state.playing);
  renderTransport();
  $('source-info').textContent = state.lyrics
    ? `Fuente: ${state.lyrics.source}${state.lyrics.source_id !== null ? ` · ID ${state.lyrics.source_id}` : ''}. Derechos de exhibición según la fuente o el archivo aportado.`
    : 'Sin fuente de letras seleccionada.';
  const art = $<HTMLImageElement>('artwork');
  art.hidden = !state.artwork;
  if (state.artwork && art.getAttribute('src') !== state.artwork) art.src = state.artwork;
  const reading = readingStatus(state, lyrics);
  $('reading').hidden = !reading;
  $('lyrics').hidden = !!reading;
  document.body.classList.toggle('reading-mode', !!reading);
  if (reading) {
    $('reading-label').textContent = reading.title;
    $('reading-detail').textContent = reading.detail;
  }
  if (reading && lyrics) {
    const text = readingText(lyrics);
    if ($('plain-text').textContent !== text) $('plain-text').textContent = text;
  }
  renderTimedLyrics(lyrics);
  $('elapsed').textContent = time(state.position_ms);
  const progress = $<HTMLProgressElement>('progress');
  progress.max = state.track?.duration_ms || 1;
  progress.value = state.position_ms;
  const fill = $('progress-fill');
  const ratio = state.track?.duration_ms ? Math.min(1, state.position_ms / state.track.duration_ms) : 0;
  fill.style.width = `${ratio * 100}%`;
  $('progress-wrap').hidden = !state.track?.duration_ms;
  const signature = JSON.stringify(state.sessions);
  if (signature !== sessionSignature) {
    const placeholder = new Option('Selecciona una sesión', '');
    select('sessions').replaceChildren(placeholder, ...state.sessions.map(session => new Option(session.name, session.id)));
    sessionSignature = signature;
  }
  select('sessions').value = state.selected_session ?? '';
  $('select-source').textContent = state.selected_session ? 'Cambiar sesión' : 'Elegir sesión';
  const candidates = JSON.stringify([state.generation, state.selected_candidate_id, state.candidates]);
  if (candidates !== candidateSignature) {
    $('candidates').replaceChildren(...state.candidates.map(candidate => {
      const button = document.createElement('button');
      button.className = 'candidate';
      const title = document.createElement('strong');
      title.textContent = `${candidate.id === state.selected_candidate_id ? 'Seleccionada · ' : ''}${candidate.trackName}`;
      const detail = document.createElement('span');
      detail.textContent = `${candidate.artistName} · ${candidate.albumName || 'Sin álbum'} · ${time(candidate.duration * 1000)}${candidate.instrumental ? ' · Instrumental' : ''}`;
      button.append(title, detail);
      if (candidate.matchNote) {
        const note = document.createElement('span');
        note.className = 'match-note';
        note.textContent = candidate.matchNote;
        button.append(note);
      }
      const generation = state.generation;
      button.onclick = async () => {
        button.disabled = true;
        try {
          await command('choose_candidate', { id: candidate.id, generation });
          $('candidates-panel').hidden = true;
        } catch (error) {
          notice(String(error));
        } finally {
          button.disabled = false;
        }
      };
      return button;
    }));
    candidateSignature = candidates;
  }
  const searchBtn = $<HTMLButtonElement>('search');
  searchBtn.textContent = state.candidates.length
    ? `Versiones (${state.candidates.length})`
    : state.lyrics_status === 'loading'
      ? 'Buscando…'
      : state.lyrics
        ? 'Volver a buscar'
        : 'Buscar letra';
  searchBtn.disabled = !state.track || state.lyrics_status === 'loading';
  $<HTMLButtonElement>('import').disabled = !state.track;
  $<HTMLButtonElement>('open-path-import').disabled = !state.track;
  if (state.error && state.error !== previousError) notice(state.error);
  const docSignature = JSON.stringify([state.generation, state.lyrics?.source, state.lyrics?.source_id, state.lyrics?.warnings]);
  if (docSignature !== documentSignature && state.lyrics?.warnings.length) {
    notice(`LRC importado con advertencias: ${state.lyrics.warnings[0]}`);
  }
  documentSignature = docSignature;
  if ($('settings-panel').hidden) {
    previewSettings = null;
    applyAppearance(state.settings);
  } else {
    syncMascot(effectiveSettings());
  }
}

function renderTimedLyrics(lyrics: Snapshot['lyrics']) {
  const view = lyricView({ ...state, lyrics });
  for (const key of ['previous', 'next'] as const) {
    if ($(key).textContent !== view[key]) $(key).textContent = view[key];
  }
  const words = canHighlightWords(state, lyrics, translated);
  const status = modeStatus(state, lyrics, translated);
  $('karaoke-status').textContent = status;
  $('settings-karaoke-status').textContent = status;
  $('karaoke-badge').title = status;
  const reading = readingStatus(state, lyrics);
  $('karaoke-badge').textContent = words ? 'Palabra' : reading ? 'Lectura' : 'Línea';
  $('current').classList.toggle('word-mode', words);
  if (words && lyrics && state.active_line != null) {
    const key = `${state.generation}:${state.lyrics_revision}:${state.active_line}:${translated}:${lyrics.lines[state.active_line].text}`;
    const reduced = effectiveSettings().reduced_motion || systemReducedMotion();
    wordRenderer.render($('current'), key, lyrics.lines[state.active_line], state.position_ms + effectiveSettings().offset_ms, reduced);
  } else {
    wordRenderer.clear();
    if ($('current').textContent !== view.current || $('current').children.length) $('current').textContent = view.current;
  }
}

function positionTick(tick: PositionTick) {
  if (!state || tick.generation !== state.generation || tick.lyrics_revision !== state.lyrics_revision || tick.sequence <= (state.position_sequence ?? 0)) return;
  state = {
    ...state,
    position_sequence: tick.sequence,
    position_ms: tick.position_ms,
    active_line: tick.active_line,
    playing: tick.playing,
    timing_reliable: tick.timing_reliable,
  };
  renderTimedLyrics(translated && state.translation ? state.translation : state.lyrics);
  syncMascot();
  const progress = $<HTMLProgressElement>('progress');
  progress.value = state.position_ms;
  const ratio = state.track?.duration_ms ? Math.min(1, state.position_ms / state.track.duration_ms) : 0;
  $('progress-fill').style.width = `${ratio * 100}%`;
  $('elapsed').textContent = time(state.position_ms);
  $('playback').textContent = !state.track ? 'Sin sesión' : state.playing ? 'Reproduciendo' : 'En pausa';
  $('playback').classList.toggle('playing', state.playing);
  renderTransport();
}

function populateSettings() {
  const s = state.settings;
  select('karaoke-mode').value = s.karaoke_mode ?? 'line';
  select('z-order').value = s.z_order;
  select('view-mode').value = s.view_mode;
  select('theme-id').value = normalizeThemeId(s.theme_id);
  select('lyrics-backdrop').value = normalizeLyricsBackdrop(s.lyrics_backdrop);
  input('settings-cat-animation').checked = s.cat_animation_enabled !== false;
  input('offset').value = String(s.offset_ms);
  $('offset-value').textContent = `${s.offset_ms > 0 ? '+' : ''}${s.offset_ms} ms`;
  input('font-size').value = String(s.font_size);
  input('opacity').value = String(s.opacity);
  select('alignment').value = s.alignment;
  input('reduced-motion').checked = s.reduced_motion;
  input('network-allowed').checked = s.network_allowed;
  $('theme-help').textContent = THEME_HELP[normalizeThemeId(s.theme_id)];
  $('form-error').hidden = true;
  previewSettings = null;
}

function settingsFromForm(): Settings {
  return {
    ...state.settings,
    karaoke_mode: select('karaoke-mode').value as Settings['karaoke_mode'],
    z_order: select('z-order').value as Settings['z_order'],
    view_mode: select('view-mode').value as Settings['view_mode'],
    theme_id: normalizeThemeId(select('theme-id').value),
    lyrics_backdrop: normalizeLyricsBackdrop(select('lyrics-backdrop').value) as LyricsBackdrop,
    cat_animation_enabled: input('settings-cat-animation').checked,
    offset_ms: Number(input('offset').value),
    font_size: Number(input('font-size').value),
    opacity: Number(input('opacity').value),
    alignment: select('alignment').value,
    reduced_motion: input('reduced-motion').checked,
    network_allowed: input('network-allowed').checked,
  };
}

function openSettings(scrollToAppearance = false) {
  hideAuxiliary();
  closeAllPopovers(false);
  populateSettings();
  $('settings-panel').hidden = false;
  $('candidates-panel').hidden = true;
  if (scrollToAppearance) {
    $('appearance-group').scrollIntoView({ block: 'start' });
    select('theme-id').focus();
  } else {
    $('close-settings').focus();
  }
}

function closeSettings() {
  $('settings-panel').hidden = true;
  previewSettings = null;
  applyAppearance(state.settings);
  $('open-settings').focus();
}

async function persistSettings(next: Settings, close = false) {
  await command('update_settings', { settings: next });
  render(await command<Snapshot>('get_state'));
  if (close) closeSettings();
}

$('open-settings').onclick = () => openSettings();
$('select-source').onclick = () => { openSettings(); $('sessions').focus(); };
$('close-settings').onclick = closeSettings;
$('settings-form').oninput = () => {
  const s = settingsFromForm();
  $('offset-value').textContent = `${s.offset_ms > 0 ? '+' : ''}${s.offset_ms} ms`;
  $('theme-help').textContent = THEME_HELP[normalizeThemeId(s.theme_id)];
  previewSettings = { ...s, view_mode: state.settings.view_mode };
  applyAppearance({ ...s, view_mode: state.settings.view_mode });
};
$('settings-form').onsubmit = async event => {
  event.preventDefault();
  if (saving) return;
  saving = true;
  try {
    await persistSettings(settingsFromForm(), true);
    notice('Ajustes guardados.');
  } catch (error) {
    $('form-error').textContent = String(error);
    $('form-error').hidden = false;
  } finally {
    saving = false;
  }
};
$('sessions').onchange = () => { void action('select_session', { id: select('sessions').value || null }); };
$('view-toggle').onclick = () => {
  void action('update_settings', {
    settings: { ...state.settings, view_mode: state.settings.view_mode === 'compact' ? 'lyrics_only' : 'compact' },
  });
};
$('hide').onclick = () => { void action('hide_window'); };

$('lyrics-menu-button').onclick = () => {
  if ($('lyrics-menu').hidden) openPopover('lyrics-menu', $('lyrics-menu-button'));
  else closePopover('lyrics-menu');
};
$('karaoke-badge').onclick = () => {
  if ($('karaoke-menu').hidden) openPopover('karaoke-menu', $('karaoke-badge'));
  else closePopover('karaoke-menu');
};
$('karaoke-menu').onclick = async event => {
  const target = (event.target as HTMLElement).closest('[data-karaoke]') as HTMLElement | null;
  if (!target) return;
  const mode = target.dataset.karaoke as 'line' | 'word';
  closePopover('karaoke-menu');
  await action('update_settings', { settings: { ...state.settings, karaoke_mode: mode } });
};

$('import').onclick = async () => {
  closePopover('lyrics-menu');
  const button = $<HTMLButtonElement>('import');
  button.disabled = true;
  await action('import_lrc');
  button.disabled = !state.track;
};
$('open-path-import').onclick = () => {
  closePopover('lyrics-menu');
  openSettings();
  const details = document.querySelector('.settings-advanced') as HTMLDetailsElement | null;
  if (details) details.open = true;
  input('local-lrc-path').focus();
};
$('local-lrc-submit').onclick = async () => {
  try {
    await command('import_lrc_path', { path: input('local-lrc-path').value.trim() });
    closeSettings();
    notice('LRC local cargado.');
  } catch (error) {
    notice(String(error));
  }
};
$('search').onclick = () => {
  closePopover('lyrics-menu');
  if (state.candidates.length) {
    $('candidates-panel').hidden = false;
    $('settings-panel').hidden = true;
    $('close-candidates').focus();
  } else if (!state.settings.network_allowed) {
    openSettings();
    input('network-allowed').focus();
  } else {
    void action('search_lyrics');
  }
};
$('close-candidates').onclick = () => {
  $('candidates-panel').hidden = true;
  $('lyrics-menu-button').focus();
};

$('correct-search').onclick = () => {
  closePopover('lyrics-menu');
  hideAuxiliary();
  $('settings-panel').hidden = true;
  correctionGeneration = state.generation;
  const track = state.search_override ?? state.track;
  input('search-title').value = track?.title ?? '';
  input('search-artist').value = track?.artist ?? '';
  $('correction-panel').hidden = false;
  input('search-title').focus();
};
$('close-correction').onclick = () => {
  $('correction-panel').hidden = true;
  $('lyrics-menu-button').focus();
};
$('correction-form').onsubmit = async event => {
  event.preventDefault();
  try {
    await command('correct_search', {
      title: input('search-title').value,
      artist: input('search-artist').value,
      generation: correctionGeneration,
      reset: false,
    });
    $('correction-panel').hidden = true;
  } catch (error) {
    notice(String(error));
  }
};
$('reset-search').onclick = async () => {
  try {
    await command('correct_search', { title: '', artist: '', generation: correctionGeneration, reset: true });
    $('correction-panel').hidden = true;
  } catch (error) {
    notice(String(error));
  }
};
$('translate').onclick = () => {
  closePopover('lyrics-menu');
  if (state.translation) {
    translated = !translated;
    render(state);
    return;
  }
  hideAuxiliary();
  translationGeneration = state.generation;
  translationRevision = state.lyrics_revision ?? 0;
  $('settings-panel').hidden = true;
  $('translation-panel').hidden = false;
  select('translation-language').value = state.language ?? '';
  select('translation-language').focus();
};
$('close-translation').onclick = () => {
  $('translation-panel').hidden = true;
  $('lyrics-menu-button').focus();
};
$('confirm-translation').onclick = async () => {
  try {
    await command('translate_lyrics', {
      source: select('translation-language').value || null,
      generation: translationGeneration,
      lyricsRevision: translationRevision,
    });
    translated = true;
    $('translation-panel').hidden = true;
    render(state);
  } catch (error) {
    notice(String(error));
  }
};
$('cancel-translation').onclick = () => {
  closePopover('lyrics-menu');
  void action('cancel_translation');
};
$('reset-position').onclick = () => { void action('reset_window'); };
$('quit').onclick = () => { void action('quit'); };

$('cat-options').onclick = () => {
  if ($('cat-menu').hidden) {
    input('cat-animation').checked = state.settings.cat_animation_enabled !== false;
    openPopover('cat-menu', $('cat-options'));
    mascot.triggerGreet();
  } else {
    closePopover('cat-menu');
  }
};
$('cat-options').onpointerenter = () => mascot.triggerGreet();
$('cat-options').onpointerleave = () => mascot.leaveHover();
$('cat-animation').onchange = async () => {
  const previous = state.settings.cat_animation_enabled;
  const next = { ...state.settings, cat_animation_enabled: input('cat-animation').checked };
  try {
    await persistSettings(next);
  } catch (error) {
    input('cat-animation').checked = previous !== false;
    notice(String(error));
  }
};
$('cat-open-appearance').onclick = () => {
  closePopover('cat-menu', false);
  openSettings(true);
};

$('drag-handle').onpointerdown = event => {
  if (native && event.button === 0) void getCurrentWindow().startDragging();
};

document.addEventListener('keydown', event => {
  if (event.key === 'Escape') {
    if (!$('cat-menu').hidden) { closePopover('cat-menu'); return; }
    if (!$('lyrics-menu').hidden) { closePopover('lyrics-menu'); return; }
    if (!$('karaoke-menu').hidden) { closePopover('karaoke-menu'); return; }
    if (!$('settings-panel').hidden) { closeSettings(); return; }
    hideAuxiliary();
  }
});

document.addEventListener('pointerdown', event => {
  const target = event.target as Node;
  for (const id of ['cat-menu', 'lyrics-menu', 'karaoke-menu'] as const) {
    const pop = $(id);
    if (pop.hidden) continue;
    const anchors = id === 'cat-menu' ? [$('cat-options')]
      : id === 'lyrics-menu' ? [$('lyrics-menu-button')]
        : [$('karaoke-badge')];
    if (!pop.contains(target) && !anchors.some(a => a.contains(target))) closePopover(id);
  }
});

document.addEventListener('visibilitychange', () => syncMascot());
window.addEventListener('focus', () => syncMascot());
document.body.addEventListener('pointerenter', () => {
  document.body.classList.add('controls-visible');
  syncMascot();
});
document.body.addEventListener('pointerleave', () => {
  document.body.classList.remove('controls-visible');
  syncMascot();
});
document.body.addEventListener('focusin', () => {
  document.body.classList.add('controls-visible');
  syncMascot();
});
document.body.addEventListener('focusout', () => {
  requestAnimationFrame(() => {
    if (!document.body.contains(document.activeElement)) document.body.classList.remove('controls-visible');
    syncMascot();
  });
});

async function start() {
  document.documentElement.dataset.theme = 'classic_cat';
  document.documentElement.dataset.backdrop = 'dark';
  if (native) {
    await listen<PositionTick>('proyecto-reze:position', event => positionTick(event.payload));
    await listen<Snapshot>('proyecto-reze:state', event => render(event.payload));
    await listen('proyecto-reze:settings', () => openSettings());
    render(await command<Snapshot>('get_state'));
  } else {
    render({
      revision: 0,
      generation: 0,
      sessions: [],
      selected_session: null,
      track: null,
      playing: false,
      position_ms: 0,
      timing_reliable: false,
      artwork: null,
      lyrics: null,
      active_line: null,
      lyrics_status: 'missing',
      candidates: [],
      error: null,
      settings: { ...DEFAULT_SETTINGS },
    });
    notice('Vista previa · abre la aplicación de escritorio para usar GSMTC.');
  }
}

void start().catch(error => notice(`No se pudo iniciar la interfaz: ${String(error)}`));

export { applyAppearance, settingsFromForm, populateSettings, mascot };
