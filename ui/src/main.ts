import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Settings, Snapshot, PositionTick } from './types';
import { lyricView, time, readingStatus } from './view';
import {canHighlightWords, modeStatus, WordRenderer} from './karaoke';
import './style.css';

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const input = (id: string) => $<HTMLInputElement>(id);
const select = (id: string) => $<HTMLSelectElement>(id);
const native = '__TAURI_INTERNALS__' in window;
let state: Snapshot;
const wordRenderer=new WordRenderer();
let sessionSignature = '';
let candidateSignature = '';
let documentSignature = '';
let saving = false;
let translated = false;
let displayedRevision = -1;
let correctionGeneration = 0;
let translationGeneration=0;
let translationRevision=0;
let noticeTimer: ReturnType<typeof setTimeout>;
function notice(message: string) { $('notice').textContent = message; $('notice').hidden = false; clearTimeout(noticeTimer); noticeTimer = setTimeout(() => { $('notice').hidden = true; }, 7000); }
async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  if (native) return invoke<T>(name, args);
  throw new Error('Esta acción necesita la aplicación de escritorio.');
}
async function action(name: string, args?: Record<string, unknown>) {
  try { return await command(name, args); } catch (error) { notice(String(error)); }
}
function applyAppearance(settings: Settings) {
  document.documentElement.style.setProperty('--lyric-size', `${settings.font_size}px`);
  document.documentElement.style.setProperty('--opacity', String(settings.opacity));
  document.documentElement.style.setProperty('--alignment', settings.alignment);
  document.body.classList.toggle('lyrics-only', settings.view_mode === 'lyrics_only');
  document.body.classList.toggle('reduced-motion', settings.reduced_motion);
  $('view-toggle').setAttribute('aria-label', settings.view_mode === 'compact' ? 'Cambiar a solo letras' : 'Cambiar a vista compacta');
}
function render(next: Snapshot) {
  if (state && next.revision < state.revision) return;
  if(state && next.generation===state.generation && next.lyrics_revision===state.lyrics_revision
    && (next.position_sequence??0)<(state.position_sequence??0)){
    next={...next,position_sequence:state.position_sequence,position_ms:state.position_ms,active_line:state.active_line,playing:state.playing,timing_reliable:state.timing_reliable};
  }
  const previousError = state?.error;
  state = next;
  if(displayedRevision !== (state.lyrics_revision ?? 0)){translated=false;displayedRevision=state.lyrics_revision ?? 0;}
  const lyrics=translated && state.translation ? state.translation : state.lyrics;
  $('selection-alert').textContent=state.selection_notice ?? '';
  $('selection-alert').hidden=!state.selection_notice;
  $<HTMLButtonElement>('correct-search').disabled=!state.track;
  $('translate').hidden=!state.lyrics || state.lyrics.format==='instrumental' || (state.language==='es' && !state.translation);
  $('translate').textContent=state.translation_status==='loading' ? 'Traduciendo…' : state.translation ? (translated ? 'Ver original' : 'Ver español') : 'Traducir al español';
  $<HTMLButtonElement>('translate').disabled=state.translation_status==='loading';
  $('cancel-translation').hidden=state.translation_status!=='loading';
  $('track-title').textContent = state.track?.title || (state.track ? 'Título no disponible' : 'Tu música, a la vista.');
  $('track-artist').textContent = state.track ? state.track.artist || 'Artista no disponible' : 'Selecciona una sesión para empezar';
  $('track-title').title = state.track?.title ?? '';
  $('playback').textContent = !state.track ? 'Sin sesión' : state.playing ? 'Reproduciendo' : 'En pausa';
  $('playback').classList.toggle('playing', state.playing);
  $('source-info').textContent = state.lyrics ? `Fuente: ${state.lyrics.source}${state.lyrics.source_id !== null ? ` · ID ${state.lyrics.source_id}` : ''}. Derechos de exhibición según la fuente o el archivo aportado.` : 'Sin fuente de letras seleccionada.';
  const art = $<HTMLImageElement>('artwork');
  art.hidden = !state.artwork;
  if (state.artwork && art.getAttribute('src') !== state.artwork) art.src = state.artwork;
  const reading=readingStatus(state,lyrics);
  $('reading').hidden=!reading; $('lyrics').hidden=!!reading;
  document.body.classList.toggle('reading-mode',!!reading);
  if(reading){$('reading-label').textContent=reading.title;$('reading-detail').textContent=reading.detail;}
  if (reading && lyrics) {const text=lyrics.plain_text || lyrics.lines.map(line=>line.text).join('\n');if($('plain-text').textContent!==text)$('plain-text').textContent=text;}
  renderTimedLyrics(lyrics);
  $('elapsed').textContent = time(state.position_ms);
  const progress = $<HTMLProgressElement>('progress'); progress.max = state.track?.duration_ms || 1; progress.value = state.position_ms;
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
      const button = document.createElement('button'); button.className = 'candidate';
      const title = document.createElement('strong'); title.textContent = `${candidate.id === state.selected_candidate_id ? 'Seleccionada · ' : ''}${candidate.trackName}`;
      const detail = document.createElement('span'); detail.textContent = `${candidate.artistName} · ${candidate.albumName || 'Sin álbum'} · ${time(candidate.duration * 1000)}${candidate.instrumental ? ' · Instrumental' : ''}`;
      button.append(title, detail);
      if (candidate.matchNote) { const note = document.createElement('span'); note.className = 'match-note'; note.textContent = candidate.matchNote; button.append(note); }
      const generation = state.generation;
      button.onclick = async () => { button.disabled = true; try { await command('choose_candidate', {id: candidate.id, generation}); $('candidates-panel').hidden = true; } catch (error) { notice(String(error)); } finally { button.disabled = false; } };
      return button;
    })); candidateSignature = candidates;
  }
  $('search').textContent = state.candidates.length ? `Versiones (${state.candidates.length})` : state.lyrics_status === 'loading' ? 'Buscando…' : 'Buscar letra';
  $<HTMLButtonElement>('import').disabled = !state.track;
  $<HTMLButtonElement>('search').disabled = !state.track || state.lyrics_status === 'loading';
  if (state.error && state.error !== previousError) notice(state.error);
  const docSignature = JSON.stringify([state.generation,state.lyrics?.source,state.lyrics?.source_id,state.lyrics?.warnings]);
  if (docSignature !== documentSignature && state.lyrics?.warnings.length) notice(`LRC importado con advertencias: ${state.lyrics.warnings[0]}`);
  documentSignature = docSignature;
  if ($('settings-panel').hidden) applyAppearance(state.settings);
}
function renderTimedLyrics(lyrics:Snapshot['lyrics']){
  const view=lyricView({...state,lyrics});
  for(const key of ['previous','next'] as const)if($(key).textContent!==view[key])$(key).textContent=view[key];
  const words=canHighlightWords(state,lyrics,translated);
  const status=modeStatus(state,lyrics,translated);
  $('karaoke-status').textContent=status;$('karaoke-badge').title=status;
  $('karaoke-badge').textContent=words?'Palabra':readingStatus(state,lyrics)?'Lectura':'Línea';
  $('current').classList.toggle('word-mode',words);
  if(words && lyrics && state.active_line!=null){
    const key=`${state.generation}:${state.lyrics_revision}:${state.active_line}:${translated}:${lyrics.lines[state.active_line].text}`;
    const reduced=state.settings.reduced_motion || matchMedia('(prefers-reduced-motion: reduce)').matches;
    wordRenderer.render($('current'),key,lyrics.lines[state.active_line],state.position_ms+state.settings.offset_ms,reduced);
  }else{wordRenderer.clear();if($('current').textContent!==view.current || $('current').children.length)$('current').textContent=view.current;}
}
function positionTick(tick:PositionTick){
  if(!state || tick.generation!==state.generation || tick.lyrics_revision!==state.lyrics_revision || tick.sequence<=(state.position_sequence??0))return;
  state={...state,position_sequence:tick.sequence,position_ms:tick.position_ms,active_line:tick.active_line,playing:tick.playing,timing_reliable:tick.timing_reliable};
  renderTimedLyrics(translated && state.translation ? state.translation : state.lyrics);
}
function populateSettings() {
  const s = state.settings;
  select('karaoke-mode').value=s.karaoke_mode??'line';
  select('z-order').value = s.z_order; select('view-mode').value = s.view_mode;
  input('offset').value = String(s.offset_ms); $('offset-value').textContent = `${s.offset_ms > 0 ? '+' : ''}${s.offset_ms} ms`;
  input('font-size').value = String(s.font_size); input('opacity').value = String(s.opacity);
  select('alignment').value = s.alignment; input('reduced-motion').checked = s.reduced_motion;
  input('network-allowed').checked = s.network_allowed;
  $('form-error').hidden = true;
}
function settingsFromForm(): Settings {
  return {...state.settings, karaoke_mode:select('karaoke-mode').value as Settings['karaoke_mode'], z_order: select('z-order').value as Settings['z_order'], view_mode: select('view-mode').value as Settings['view_mode'], offset_ms: Number(input('offset').value), font_size: Number(input('font-size').value), opacity: Number(input('opacity').value), alignment: select('alignment').value, reduced_motion: input('reduced-motion').checked, network_allowed: input('network-allowed').checked};
}
function openSettings() { hideAuxiliary(); populateSettings(); $('settings-panel').hidden = false; $('candidates-panel').hidden = true; $('close-settings').focus(); }
function closeSettings() { $('settings-panel').hidden = true; applyAppearance(state.settings); $('open-settings').focus(); }
$('open-settings').onclick = openSettings; $('select-source').onclick = () => { openSettings(); $('sessions').focus(); };
$('close-settings').onclick = closeSettings;
$('settings-form').oninput = () => { const s = settingsFromForm(); $('offset-value').textContent = `${s.offset_ms > 0 ? '+' : ''}${s.offset_ms} ms`; applyAppearance({...s, view_mode: state.settings.view_mode}); };
$('settings-form').onsubmit = async event => {
  event.preventDefault(); if (saving) return; saving = true;
  try { await command('update_settings', {settings: settingsFromForm()}); render(await command<Snapshot>('get_state')); closeSettings(); notice('Ajustes guardados.'); }
  catch (error) { $('form-error').textContent = String(error); $('form-error').hidden = false; }
  finally { saving = false; }
};
$('sessions').onchange = () => { void action('select_session', {id: select('sessions').value || null}); };
$('view-toggle').onclick = () => { void action('update_settings', {settings: {...state.settings, view_mode: state.settings.view_mode === 'compact' ? 'lyrics_only' : 'compact'}}); };
$('hide').onclick = () => { void action('hide_window'); };
$('import').onclick = async () => { const button = $<HTMLButtonElement>('import'); button.disabled = true; await action('import_lrc'); button.disabled = !state.track; };
$('local-lrc-form').onsubmit = async event => {
  event.preventDefault();
  try { await command('import_lrc_path', {path: input('local-lrc-path').value.trim()}); closeSettings(); notice('LRC local cargado.'); }
  catch (error) { notice(String(error)); }
};
$('search').onclick = () => {
  if (state.candidates.length) { $('candidates-panel').hidden = false; $('settings-panel').hidden = true; $('close-candidates').focus(); }
  else if (!state.settings.network_allowed) { openSettings(); $('network-allowed').focus(); }
  else void action('search_lyrics');
};
$('close-candidates').onclick = () => { $('candidates-panel').hidden = true; $('search').focus(); };
function hideAuxiliary(){for(const id of ['correction-panel','translation-panel','candidates-panel'])$(id).hidden=true;}
$('correct-search').onclick=()=>{hideAuxiliary();$('settings-panel').hidden=true;correctionGeneration=state.generation;
  const track=state.search_override ?? state.track; input('search-title').value=track?.title ?? '';input('search-artist').value=track?.artist ?? '';
  $('correction-panel').hidden=false;input('search-title').focus();};
$('close-correction').onclick=()=>{$('correction-panel').hidden=true;$('correct-search').focus();};
$('correction-form').onsubmit=async event=>{event.preventDefault();try{await command('correct_search',{title:input('search-title').value,artist:input('search-artist').value,generation:correctionGeneration,reset:false});$('correction-panel').hidden=true;}catch(error){notice(String(error));}};
$('reset-search').onclick=async()=>{try{await command('correct_search',{title:'',artist:'',generation:correctionGeneration,reset:true});$('correction-panel').hidden=true;}catch(error){notice(String(error));}};
$('translate').onclick=()=>{if(state.translation){translated=!translated;render(state);return;}hideAuxiliary();translationGeneration=state.generation;translationRevision=state.lyrics_revision ?? 0;$('settings-panel').hidden=true;$('translation-panel').hidden=false;select('translation-language').value=state.language ?? '';select('translation-language').focus();};
$('close-translation').onclick=()=>{$('translation-panel').hidden=true;$('translate').focus();};
$('confirm-translation').onclick=async()=>{try{await command('translate_lyrics',{source:select('translation-language').value || null,generation:translationGeneration,lyricsRevision:translationRevision});translated=true;$('translation-panel').hidden=true;render(state);}catch(error){notice(String(error));}};
$('cancel-translation').onclick=()=>{void action('cancel_translation');};
$('reset-position').onclick = () => { void action('reset_window'); };
$('quit').onclick = () => { void action('quit'); };
$('drag-handle').onpointerdown = event => { if (native && event.button === 0) void getCurrentWindow().startDragging(); };
document.addEventListener('keydown', event => { if (event.key === 'Escape') { closeSettings(); hideAuxiliary(); } });

async function start() {
  if (native) {
    await listen<PositionTick>('proyecto-reze:position',event=>positionTick(event.payload));
    await listen<Snapshot>('proyecto-reze:state', event => render(event.payload));
    await listen('proyecto-reze:settings', openSettings);
    render(await command<Snapshot>('get_state'));
  } else {
    const defaults: Settings = {schema_version: 1, network_allowed: true, offset_ms: 0, font_size: 23, opacity: 1, alignment: 'center', reduced_motion: false, z_order: 'normal', view_mode: 'compact', x: null,y: null,width: 420,height: 260};
    render({revision: 0,generation: 0,sessions: [],selected_session: null,track: null,playing: false,position_ms: 0,timing_reliable: false,artwork: null,lyrics: null,active_line: null,lyrics_status: 'missing',candidates: [],error: null,settings: defaults});
    notice('Vista previa · abre la aplicación de escritorio para usar GSMTC.');
  }
}
void start().catch(error => notice(`No se pudo iniciar la interfaz: ${String(error)}`));
