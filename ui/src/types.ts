export type ThemeId = 'classic_cat' | 'window_cat' | 'cat_notebook' | 'cat_constellation';
export type LyricsBackdrop = 'dark' | 'light';

export interface Settings {
  karaoke_mode?: 'line' | 'word';
  schema_version: number; network_allowed: boolean;
  offset_ms: number; font_size: number; opacity: number; alignment: string;
  reduced_motion: boolean; z_order: 'top' | 'normal' | 'bottom'; view_mode: 'compact' | 'lyrics_only';
  theme_id: ThemeId;
  cat_animation_enabled: boolean;
  lyrics_backdrop: LyricsBackdrop;
  x: number | null; y: number | null; width: number; height: number;
}
export interface Track { title: string; artist: string; album: string; duration_ms: number | null }
export interface Candidate { id: number; trackName: string; artistName: string; albumName: string; duration: number; syncedLyrics?: string; instrumental?: boolean; matchNote?: string | null }
export interface Snapshot {
  position_sequence?: number;
  search_override?: Track | null; selected_candidate_id?: number | null; lyrics_revision?: number;
  sync_confirmed?: boolean; selection_notice?: string | null;
  translation?: Snapshot['lyrics']; translation_status?: string; language?: string | null;
  revision: number; generation: number; sessions: {id: string; name: string}[]; selected_session: string | null;
  track: Track | null; playing: boolean; position_ms: number; timing_reliable: boolean; artwork: string | null;
  lyrics: {source: string; source_id: number | null; format: 'plain' | 'line_synced' | 'instrumental'; lines: LyricLine[]; plain_text: string; warnings: string[]} | null;
  active_line: number | null; lyrics_status: string; candidates: Candidate[]; error: string | null; settings: Settings;
}
export interface LyricWord {text:string;start_ms:number;end_ms?:number|null}
export interface LyricLine {at_ms:number;text:string;end_ms?:number|null;words?:LyricWord[]}
export interface PositionTick {generation:number;lyrics_revision:number;sequence:number;position_ms:number;active_line:number|null;playing:boolean;timing_reliable:boolean}

export const THEME_IDS: ThemeId[] = ['classic_cat', 'window_cat', 'cat_notebook', 'cat_constellation'];
export const DEFAULT_SETTINGS: Settings = {
  schema_version: 1,
  network_allowed: true,
  offset_ms: 0,
  font_size: 23,
  opacity: 1,
  alignment: 'center',
  reduced_motion: false,
  z_order: 'normal',
  view_mode: 'compact',
  theme_id: 'classic_cat',
  cat_animation_enabled: true,
  lyrics_backdrop: 'dark',
  x: null,
  y: null,
  width: 420,
  height: 260,
};

export function normalizeThemeId(value: unknown): ThemeId {
  return THEME_IDS.includes(value as ThemeId) ? (value as ThemeId) : 'classic_cat';
}

export function normalizeLyricsBackdrop(value: unknown): LyricsBackdrop {
  return value === 'light' || value === 'dark' ? value : 'dark';
}
