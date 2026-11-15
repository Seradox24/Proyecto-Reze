import type { Snapshot, LyricLine, LyricWord } from './types';
import { readingStatus } from './view';

export function wordProgress(word: LyricWord, position: number, reduced = false): number {
  if (position < word.start_ms) return 0;
  if (reduced || word.end_ms == null || word.end_ms <= word.start_ms) return 1;
  return Math.max(0, Math.min(1, (position - word.start_ms) / (word.end_ms - word.start_ms)));
}
export function canHighlightWords(
  state: Snapshot,
  lyrics: Snapshot['lyrics'],
  translation = false,
): boolean {
  return (
    state.settings.karaoke_mode === 'word' &&
    !translation &&
    state.sync_confirmed !== false &&
    state.timing_reliable &&
    state.active_line != null &&
    !!lyrics?.lines[state.active_line]?.words?.length
  );
}
export function modeStatus(
  state: Snapshot,
  lyrics: Snapshot['lyrics'],
  translation = false,
): string {
  const reading = readingStatus(state, lyrics);
  if (reading) return `${reading.title}. ${reading.detail}`;
  if (state.settings.karaoke_mode !== 'word') return 'Resaltado por línea.';
  if (translation)
    return 'La traducción se muestra por línea; el karaoke por palabra corresponde al original.';
  if (state.sync_confirmed === false)
    return 'Modo lectura: confirma la versión en Versiones para usar sus tiempos.';
  if (!lyrics?.lines.some((line) => line.words?.length))
    return 'Esta letra no tiene tiempos por palabra válidos. Se mantiene el modo por línea.';
  if (!state.timing_reliable)
    return 'Windows no publica una posición fiable. Se mantiene el modo lectura.';
  if (state.active_line != null && !lyrics.lines[state.active_line]?.words?.length)
    return 'Este fragmento no tiene tiempos por palabra. Se resalta por línea.';
  return 'Karaoke por palabra con los tiempos originales de la letra.';
}

export class WordRenderer {
  private key = '';
  private spans: HTMLSpanElement[] = [];
  clear() {
    this.key = '';
    this.spans = [];
  }
  render(element: HTMLElement, key: string, line: LyricLine, position: number, reduced: boolean) {
    if (key !== this.key) {
      this.key = key;
      this.spans = (line.words ?? []).map((word) => {
        const span = document.createElement('span');
        span.className = 'karaoke-word';
        span.dir = 'auto';
        span.textContent = word.text;
        return span;
      });
      element.replaceChildren(...this.spans);
    }
    this.spans.forEach((span, index) => {
      const progress = wordProgress(line.words![index], position, reduced);
      span.style.setProperty('--sung', `${progress * 100}%`);
      span.dataset.progress = String(progress);
    });
  }
}
