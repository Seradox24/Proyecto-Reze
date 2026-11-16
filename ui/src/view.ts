import type { Snapshot } from './types';
export function readingText(lyrics: Snapshot['lyrics']): string {
  if (!lyrics) return '';
  // A timed document's plain field can still contain raw LRC timestamps.
  // Parsed lines are safe to display when Windows timing becomes unavailable.
  if (lyrics.format === 'line_synced') return lyrics.lines.map((line) => line.text).join('\n');
  return lyrics.plain_text;
}
export function readingStatus(
  state: Snapshot,
  lyrics: Snapshot['lyrics'],
): { title: string; detail: string } | null {
  if (lyrics?.format === 'plain')
    return {
      title: 'Modo lectura · sin tiempos',
      detail: 'Esta letra no tiene marcas de tiempo. Se muestra sin seguimiento ni resaltado.',
    };
  if (lyrics?.format !== 'line_synced') return null;
  if (state.sync_confirmed === false)
    return {
      title: 'Modo lectura · versión por confirmar',
      detail: 'Confirma esta versión en Versiones para activar el seguimiento de la letra.',
    };
  if (!state.timing_reliable)
    return {
      title: 'Modo lectura · sin posición fiable',
      detail:
        'Windows no proporciona una posición utilizable. La letra se muestra sin seguimiento ni resaltado.',
    };
  return null;
}
export function time(ms: number): string {
  const seconds = Math.floor(Math.max(0, ms) / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}
export function lyricView(state: Snapshot): { previous: string; current: string; next: string } {
  if (!state.track)
    return {
      previous: '',
      current: state.selected_session ? 'Sin sesión disponible' : 'Abre tu reproductor',
      next: state.selected_session
        ? 'La sesión elegida se cerró o no publica una pista.'
        : 'Elige una sesión de Windows para empezar.',
    };
  if (state.lyrics?.format === 'instrumental')
    return { previous: '', current: 'Pista instrumental', next: 'Esta versión no tiene letra.' };
  if (state.lyrics?.format === 'line_synced') {
    const lines = state.lyrics.lines;
    const index = state.active_line;
    if (state.sync_confirmed === false)
      return {
        previous: '',
        current: 'Versión seleccionada para lectura',
        next: 'Confirma su versión para activar los tiempos.',
      };
    if (!state.timing_reliable)
      return {
        previous: '',
        current: 'Sincronización no disponible',
        next: 'Windows no publica una posición utilizable. Puedes leer la letra.',
      };
    if (index === null)
      return {
        previous: '',
        current: 'La letra empieza en un momento',
        next: lines[0]?.text ?? '',
      };
    return {
      previous: lines[index - 1]?.text ?? '',
      current: lines[index]?.text || '♪',
      next: lines[index + 1]?.text ?? '',
    };
  }
  const messages: Record<Snapshot['lyrics_status'] | 'candidates', [string, string]> = {
    loading: ['Buscando la letra…', 'Puedes seguir escuchando.'],
    candidates:
      state.candidates?.length === 1
        ? [
            'Encontramos una posible letra',
            'Confirma la versión y su duración antes de sincronizar.',
          ]
        : ['Hay varias versiones', 'Elige la letra que corresponde a tu canción.'],
    error: ['No se pudo obtener la letra', 'Vuelve a buscar o importa un archivo LRC.'],
    missing: ['Aún no hay letra', 'Importa un LRC o permite LRCLIB en ajustes.'],
    ready: ['Aún no hay letra', 'Importa un LRC o permite LRCLIB en ajustes.'],
  };
  // Alternatives without a usable default: the listener has to pick one.
  const key =
    state.lyrics_status === 'missing' && state.candidates?.length
      ? 'candidates'
      : state.lyrics_status;
  const [current, next] = messages[key] ?? messages.missing;
  return { previous: '', current, next };
}
