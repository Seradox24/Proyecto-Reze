use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackAction {
    Play,
    Pause,
    Previous,
    Next,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaybackControls {
    pub play: bool,
    pub pause: bool,
    pub previous: bool,
    pub next: bool,
}

impl PlaybackControls {
    pub fn supports(self, action: PlaybackAction) -> bool {
        match action {
            PlaybackAction::Play => self.play,
            PlaybackAction::Pause => self.pause,
            PlaybackAction::Previous => self.previous,
            PlaybackAction::Next => self.next,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Timeline {
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub playing: bool,
    pub rate: f64,
    pub reliable: bool,
    /// Source UTC timestamp, in Windows 100 ns ticks. Repeated reads keep this value.
    pub source_updated: i64,
    pub age_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MediaSample {
    pub playback_controls: PlaybackControls,
    pub sessions: Vec<Session>,
    pub track: Option<Track>,
    pub timeline: Timeline,
    pub artwork: Option<String>,
    pub error: Option<String>,
}

/// Implementations own their subscriptions and release them when dropped.
pub trait MediaSource {
    fn sample(&mut self, selected: Option<&str>) -> Result<MediaSample, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LyricWord {
    pub text: String,
    pub start_ms: i64,
    pub end_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LyricLine {
    pub at_ms: i64,
    pub text: String,
    #[serde(default)]
    pub end_ms: Option<i64>,
    #[serde(default)]
    pub words: Vec<LyricWord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LyricFormat {
    Plain,
    LineSynced,
    Instrumental,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsDocument {
    pub source: String,
    pub source_id: Option<i64>,
    pub format: LyricFormat,
    pub lines: Vec<LyricLine>,
    pub plain_text: String,
    pub warnings: Vec<String>,
    pub rights_metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub playback_controls: PlaybackControls,
    pub position_sequence: u64,
    pub search_override: Option<Track>,
    pub selected_candidate_id: Option<i64>,
    pub lyrics_revision: u64,
    pub sync_confirmed: bool,
    pub selection_notice: Option<String>,
    pub translation: Option<LyricsDocument>,
    pub translation_status: String,
    pub language: Option<String>,
    pub revision: u64,
    pub generation: u64,
    pub sessions: Vec<Session>,
    pub selected_session: Option<String>,
    pub track: Option<Track>,
    pub playing: bool,
    pub position_ms: u64,
    pub timing_reliable: bool,
    pub artwork: Option<String>,
    pub lyrics: Option<LyricsDocument>,
    pub active_line: Option<usize>,
    pub lyrics_status: String,
    pub candidates: Vec<crate::lyrics::Candidate>,
    pub error: Option<String>,
    pub settings: crate::settings::Settings,
}
