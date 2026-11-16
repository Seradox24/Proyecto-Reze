export const THEME_IDS = [
  'classic_cat',
  'window_cat',
  'cat_notebook',
  'cat_constellation',
  'mochi_cat',
] as const;
export type ThemeId = (typeof THEME_IDS)[number];
export type LyricsBackdrop = 'dark' | 'light';
export type PlaybackAction = 'play' | 'pause' | 'previous' | 'next';
export interface PlaybackControls {
  play: boolean;
  pause: boolean;
  previous: boolean;
  next: boolean;
}
export interface Settings {
  karaoke_mode?: 'line' | 'word';
  schema_version: number;
  network_allowed: boolean;
  offset_ms: number;
  font_size: number;
  opacity: number;
  alignment: 'left' | 'center' | 'right';
  reduced_motion: boolean;
  z_order: 'top' | 'normal' | 'bottom';
  view_mode: 'compact' | 'lyrics_only';
  theme_id: ThemeId;
  cat_animation_enabled: boolean;
  lyrics_backdrop: LyricsBackdrop;
  x: number | null;
  y: number | null;
  width: number;
  height: number;
}
export interface Track {
  title: string;
  artist: string;
  album: string;
  duration_ms: number | null;
}
export interface Candidate {
  id: number;
  trackName: string;
  artistName: string;
  albumName: string;
  duration: number;
  instrumental?: boolean;
  matchNote?: string | null;
}
export type LyricsStatus = 'missing' | 'loading' | 'ready' | 'error';
export type TranslationStatus = 'idle' | 'loading' | 'ready' | 'error';
/** Everything except the artwork bytes and the lyrics of each alternative. */
export interface Snapshot {
  playback_controls?: PlaybackControls;
  position_sequence?: number;
  search_override?: Track | null;
  selected_candidate_id?: number | null;
  lyrics_revision?: number;
  sync_confirmed?: boolean;
  selection_notice?: string | null;
  translation?: Snapshot['lyrics'];
  translation_status?: TranslationStatus;
  language?: string | null;
  revision: number;
  generation: number;
  sessions: { id: string; name: string }[];
  selected_session: string | null;
  track: Track | null;
  playing: boolean;
  position_ms: number;
  timing_reliable: boolean;
  /** Changes whenever the artwork does; the image is fetched with `get_artwork`. */
  artwork_revision?: number;
  has_artwork?: boolean;
  lyrics: {
    source: string;
    source_id: number | null;
    format: 'plain' | 'line_synced' | 'instrumental';
    lines: LyricLine[];
    plain_text: string;
    warnings: string[];
  } | null;
  active_line: number | null;
  lyrics_status: LyricsStatus;
  candidates: Candidate[];
  error: string | null;
  settings: Settings;
}
export interface LyricWord {
  text: string;
  start_ms: number;
  end_ms?: number | null;
}
export interface LyricLine {
  at_ms: number;
  text: string;
  end_ms?: number | null;
  words?: LyricWord[];
}
export interface PositionTick {
  generation: number;
  lyrics_revision: number;
  sequence: number;
  position_ms: number;
  active_line: number | null;
  playing: boolean;
  timing_reliable: boolean;
}

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
