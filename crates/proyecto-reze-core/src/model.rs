use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    /// Data URL. Shared so repeated samples of the same track never copy the image.
    pub artwork: Option<Arc<str>>,
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LyricsStatus {
    #[default]
    Missing,
    Loading,
    Ready,
    Error,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranslationStatus {
    #[default]
    Idle,
    Loading,
    Ready,
    Error,
}

/// Small clock payload published between full snapshots.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PositionTick {
    pub generation: u64,
    pub lyrics_revision: u64,
    pub sequence: u64,
    pub position_ms: u64,
    pub active_line: Option<usize>,
    pub playing: bool,
    pub timing_reliable: bool,
}

/// State published to the interface. It deliberately excludes the artwork bytes and the
/// lyrics of every alternative: the interface asks for the artwork when its revision changes.
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
    pub translation_status: TranslationStatus,
    pub language: Option<String>,
    pub revision: u64,
    pub generation: u64,
    pub sessions: Vec<Session>,
    pub selected_session: Option<String>,
    pub track: Option<Track>,
    pub playing: bool,
    pub position_ms: u64,
    pub timing_reliable: bool,
    pub artwork_revision: u64,
    pub has_artwork: bool,
    pub lyrics: Option<LyricsDocument>,
    pub active_line: Option<usize>,
    pub lyrics_status: LyricsStatus,
    pub candidates: Vec<crate::lyrics::CandidateSummary>,
    pub error: Option<String>,
    pub settings: crate::settings::Settings,
}
