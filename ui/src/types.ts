export interface Settings {
  karaoke_mode?: 'line' | 'word';
  schema_version: number;
  network_allowed: boolean;
  offset_ms: number;
  font_size: number;
  opacity: number;
  alignment: string;
  reduced_motion: boolean;
  z_order: 'top' | 'normal' | 'bottom';
  view_mode: 'compact' | 'lyrics_only';
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
  syncedLyrics?: string;
  instrumental?: boolean;
  matchNote?: string | null;
}
export interface Snapshot {
  position_sequence?: number;
  search_override?: Track | null;
  selected_candidate_id?: number | null;
  lyrics_revision?: number;
  sync_confirmed?: boolean;
  selection_notice?: string | null;
  translation?: Snapshot['lyrics'];
  translation_status?: string;
  language?: string | null;
  revision: number;
  generation: number;
  sessions: { id: string; name: string }[];
  selected_session: string | null;
  track: Track | null;
  playing: boolean;
  position_ms: number;
  timing_reliable: boolean;
  artwork: string | null;
  lyrics: {
    source: string;
    source_id: number | null;
    format: 'plain' | 'line_synced' | 'instrumental';
    lines: LyricLine[];
    plain_text: string;
    warnings: string[];
  } | null;
  active_line: number | null;
  lyrics_status: string;
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
