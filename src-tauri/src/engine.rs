use proyecto_reze_core::{
    clock::PlaybackClock,
    lrc,
    lyrics::{Candidate, CandidateSummary, Lrclib, LyricsProvider, SearchCache, automatic_match},
    model::{
        LyricFormat, LyricsStatus, MediaSample, MediaSource, PlaybackAction, PlaybackControls,
        PositionTick, Snapshot, Track, TranslationStatus,
    },
    settings::{KaraokeMode, Settings},
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard,
        mpsc::{Receiver, SyncSender},
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

/// Automatic searches wait this long so skipping through tracks does not query each one.
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(400);
/// Window geometry is written once the window has stopped moving for this long.
const GEOMETRY_DEBOUNCE: Duration = Duration::from_millis(500);
/// Refresh interval while a line is highlighted word by word.
const WORD_FRAME: Duration = Duration::from_millis(33);

pub type Shared = Arc<Mutex<AppState>>;

pub trait SharedExt {
    fn guard(&self) -> MutexGuard<'_, AppState>;
}
impl SharedExt for Mutex<AppState> {
    /// A panic elsewhere must not make every later command fail on a poisoned lock.
    fn guard(&self) -> MutexGuard<'_, AppState> {
        self.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

struct TransportRequest {
    session_id: String,
    generation: u64,
    action: PlaybackAction,
    reply: tokio::sync::oneshot::Sender<Result<(), String>>,
}
pub struct AppState {
    transport_request: Option<TransportRequest>,
    transport_busy: bool,
    pub snapshot: Snapshot,
    pub settings_path: PathBuf,
    pub clock: PlaybackClock,
    pub provider: Option<Lrclib>,
    pub local_override: bool,
    pub request: u64,
    pub pending: Option<tauri::async_runtime::JoinHandle<()>>,
    pub translation_task: Option<tauri::async_runtime::JoinHandle<()>>,
    pub translation_request: u64,
    pub wake: SyncSender<()>,
    pub tray_available: bool,
    /// Data URL of the current artwork; published by revision, fetched on demand.
    pub artwork: Option<Arc<str>>,
    /// Full alternatives, lyrics included. The snapshot only lists their summaries.
    pub candidates: Vec<Candidate>,
    pub cache: SearchCache,
    geometry_dirty: bool,
    geometry_change: u64,
    geometry_task: bool,
}
impl AppState {
    fn validate_transport(
        &self,
        id: &str,
        generation: u64,
        action: PlaybackAction,
    ) -> Result<(), String> {
        if self.snapshot.selected_session.as_deref() != Some(id) || self.snapshot.track.is_none() {
            return Err("Selecciona una sesión con una pista disponible.".into());
        }
        if self.snapshot.generation != generation {
            return Err("La pista cambió. Vuelve a pulsar el control.".into());
        }
        if !self.snapshot.playback_controls.supports(action) {
            return Err("La sesión elegida no admite este control.".into());
        }
        Ok(())
    }
    pub fn update_position(&mut self, now: Instant) {
        self.snapshot.position_ms = self.clock.position(now);
        self.snapshot.timing_reliable = self.clock.reliable();
        self.snapshot.active_line = self
            .snapshot
            .lyrics
            .as_ref()
            .filter(|d| {
                d.format == LyricFormat::LineSynced
                    && self.clock.reliable()
                    && self.snapshot.sync_confirmed
            })
            .and_then(|d| lrc::active_line(&d.lines, self.effective_position()));
        self.snapshot.position_sequence += 1;
    }
    fn effective_position(&self) -> i64 {
        (self.snapshot.position_ms.min(i64::MAX as u64) as i64)
            .saturating_add(self.snapshot.settings.offset_ms)
    }
    pub fn tick(&self) -> PositionTick {
        PositionTick {
            generation: self.snapshot.generation,
            lyrics_revision: self.snapshot.lyrics_revision,
            sequence: self.snapshot.position_sequence,
            position_ms: self.snapshot.position_ms,
            active_line: self.snapshot.active_line,
            playing: self.snapshot.playing,
            timing_reliable: self.snapshot.timing_reliable,
        }
    }
    /// True while the active line must be repainted every frame.
    fn word_highlight(&self) -> bool {
        self.snapshot.settings.karaoke_mode == KaraokeMode::Word
            && self.snapshot.sync_confirmed
            && self.snapshot.timing_reliable
            && self
                .snapshot
                .active_line
                .and_then(|index| self.snapshot.lyrics.as_ref()?.lines.get(index))
                .is_some_and(|line| !line.words.is_empty())
    }
    /// Time until the next visible change: a word frame, the next line or the next second.
    /// `None` when nothing moves until Windows reports a change.
    fn next_position_wait(&self, now: Instant) -> Option<Duration> {
        if !self.clock.advancing() {
            return None;
        }
        if self.word_highlight() {
            return Some(WORD_FRAME);
        }
        let position = self.snapshot.position_ms;
        let next_second = (position / 1000 + 1) * 1000;
        let next_line = self
            .snapshot
            .lyrics
            .as_ref()
            .filter(|d| d.format == LyricFormat::LineSynced && self.snapshot.sync_confirmed)
            .and_then(|d| lrc::next_line_at(&d.lines, self.effective_position()))
            .map(|at| at.saturating_sub(self.snapshot.settings.offset_ms).max(0) as u64);
        let target = next_line.map_or(next_second, |line| line.min(next_second));
        self.clock
            .wait_until(target, now)
            .map(|wait| wait.clamp(Duration::from_millis(5), Duration::from_secs(1)))
    }
    pub fn cancel_search(&mut self) {
        self.request += 1;
        if let Some(task) = self.pending.take() {
            task.abort();
        }
    }
    pub fn cancel_translation(&mut self) {
        self.translation_request += 1;
        if let Some(task) = self.translation_task.take() {
            task.abort();
        }
        self.snapshot.translation = None;
        self.snapshot.translation_status = TranslationStatus::Idle;
    }
    pub fn set_candidates(&mut self, candidates: Vec<Candidate>) {
        self.snapshot.candidates = candidates.iter().map(CandidateSummary::from).collect();
        self.candidates = candidates;
    }
    fn set_artwork(&mut self, artwork: Option<Arc<str>>) {
        let same = match (&self.artwork, &artwork) {
            (None, None) => true,
            (Some(old), Some(new)) => Arc::ptr_eq(old, new) || old == new,
            _ => false,
        };
        if !same {
            self.snapshot.has_artwork = artwork.is_some();
            self.snapshot.artwork_revision += 1;
            self.artwork = artwork;
        }
    }
    pub fn install_lyrics(
        &mut self,
        document: proyecto_reze_core::model::LyricsDocument,
        id: Option<i64>,
        confirmed: bool,
    ) {
        self.cancel_translation();
        self.snapshot.lyrics_revision += 1;
        self.snapshot.language = proyecto_reze_core::translation::language(&document);
        self.snapshot.selected_candidate_id = id;
        self.snapshot.selection_notice = None;
        self.snapshot.active_line = None;
        self.snapshot.sync_confirmed = confirmed;
        self.snapshot.lyrics = Some(document);
        self.snapshot.lyrics_status = LyricsStatus::Ready;
        self.snapshot.error = None;
    }
    pub fn reset_track(&mut self) {
        self.cancel_translation();
        self.snapshot.lyrics_revision += 1;
        self.snapshot.search_override = None;
        self.snapshot.selected_candidate_id = None;
        self.snapshot.sync_confirmed = true;
        self.snapshot.selection_notice = None;
        self.snapshot.language = None;
        self.cancel_search();
        self.snapshot.generation += 1;
        self.snapshot.track = None;
        self.snapshot.lyrics = None;
        self.snapshot.active_line = None;
        self.set_candidates(vec![]);
        self.set_artwork(None);
        self.snapshot.error = None;
        self.snapshot.lyrics_status = LyricsStatus::Missing;
        self.snapshot.playing = false;
        self.snapshot.playback_controls = PlaybackControls::default();
        self.snapshot.position_ms = 0;
        self.snapshot.timing_reliable = false;
        self.local_override = false;
        self.clock = PlaybackClock::default();
    }
    /// Installs the best usable result of a search, or records why there is none.
    fn apply_search(&mut self, track: &Track, result: Result<Vec<Candidate>, String>) {
        match result {
            Ok(candidates) => {
                let selected = automatic_match(track, &candidates)
                    .into_iter()
                    .chain(0..candidates.len())
                    .find_map(|index| {
                        candidates[index]
                            .document()
                            .ok()
                            .map(|document| (index, document))
                    });
                if let Some((index, document)) = selected {
                    let candidate = &candidates[index];
                    let notice = if candidates.len() > 1 {
                        Some(format!(
                            "Encontramos {} letras. Seleccionada: {} — {}. Puedes elegir otra. {}",
                            candidates.len(),
                            candidate.track_name,
                            candidate.artist_name,
                            candidate.match_note.as_deref().unwrap_or("")
                        ))
                    } else {
                        candidate.match_note.clone()
                    };
                    let confirmed =
                        automatic_match(track, std::slice::from_ref(candidate)).is_some();
                    self.install_lyrics(document, Some(candidate.id), confirmed);
                    self.snapshot.selection_notice = notice;
                } else {
                    self.cancel_translation();
                    self.snapshot.lyrics = None;
                    self.snapshot.active_line = None;
                    self.snapshot.selected_candidate_id = None;
                    self.snapshot.lyrics_status = LyricsStatus::Missing;
                }
                self.set_candidates(candidates);
            }
            Err(error) => {
                self.snapshot.error = Some(error);
                self.snapshot.lyrics_status = LyricsStatus::Error;
            }
        }
    }
    /// Records the geometry reported by the window. Returns true when a save must be scheduled.
    pub fn geometry_changed(&mut self, x: i32, y: i32, width: f64, height: f64) -> bool {
        let settings = &mut self.snapshot.settings;
        if settings.x == Some(x)
            && settings.y == Some(y)
            && settings.width == width
            && settings.height == height
        {
            return false;
        }
        settings.x = Some(x);
        settings.y = Some(y);
        settings.width = width;
        settings.height = height;
        self.geometry_dirty = true;
        self.geometry_change += 1;
        !std::mem::replace(&mut self.geometry_task, true)
    }
    /// Call after any successful save of the whole settings file.
    pub fn geometry_saved(&mut self) {
        self.geometry_dirty = false;
    }
    /// Writes a pending geometry change now. Used by the debounce task and on exit.
    pub fn flush_geometry(&mut self) {
        if !self.geometry_dirty {
            return;
        }
        match proyecto_reze_core::settings::save(&self.settings_path, &self.snapshot.settings) {
            Ok(()) => self.geometry_dirty = false,
            Err(error) => tracing::warn!(%error, "No se pudo guardar geometría"),
        }
    }
}
pub fn create(
    settings: Settings,
    settings_path: PathBuf,
    warning: Option<String>,
    wake: SyncSender<()>,
) -> Shared {
    let provider = if settings.network_allowed {
        Lrclib::new().ok()
    } else {
        None
    };
    Arc::new(Mutex::new(AppState {
        transport_request: None,
        transport_busy: false,
        snapshot: Snapshot {
            playback_controls: PlaybackControls::default(),
            position_sequence: 0,
            search_override: None,
            selected_candidate_id: None,
            lyrics_revision: 0,
            sync_confirmed: true,
            selection_notice: None,
            translation: None,
            translation_status: TranslationStatus::Idle,
            language: None,
            revision: 0,
            generation: 0,
            sessions: vec![],
            selected_session: None,
            track: None,
            playing: false,
            position_ms: 0,
            timing_reliable: false,
            artwork_revision: 0,
            has_artwork: false,
            lyrics: None,
            active_line: None,
            lyrics_status: LyricsStatus::Missing,
            candidates: vec![],
            error: warning,
            settings,
        },
        settings_path,
        clock: PlaybackClock::default(),
        provider,
        local_override: false,
        request: 0,
        pending: None,
        translation_task: None,
        translation_request: 0,
        wake,
        tray_available: false,
        artwork: None,
        candidates: vec![],
        cache: SearchCache::default(),
        geometry_dirty: false,
        geometry_change: 0,
        geometry_task: false,
    }))
}
/// Publishes the whole snapshot. Call only when something other than the position changed;
/// the playback position travels in `proyecto-reze:position` ticks.
pub fn emit(app: &tauri::AppHandle, state: &mut AppState) {
    state.update_position(Instant::now());
    state.snapshot.revision += 1;
    // WebView dispatch can wait for the UI thread. Never retain the state mutex there.
    let snapshot = state.snapshot.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = app.emit("proyecto-reze:state", snapshot) {
            tracing::warn!(%error, "No se pudo publicar el estado");
        }
    });
}
/// Saves the window geometry once it has stopped changing, off the UI thread.
pub fn schedule_geometry_save(shared: Shared) {
    tauri::async_runtime::spawn(async move {
        loop {
            let seen = shared.guard().geometry_change;
            tokio::time::sleep(GEOMETRY_DEBOUNCE).await;
            let mut state = shared.guard();
            if state.geometry_change == seen {
                state.flush_geometry();
                state.geometry_task = false;
                break;
            }
        }
    });
}
pub fn search(app: &tauri::AppHandle, shared: Shared, manual: bool) -> Result<(), String> {
    let mut state = shared.guard();
    if state.local_override {
        return if manual {
            Err("El LRC local tiene prioridad hasta cambiar de pista.".into())
        } else {
            Ok(())
        };
    }
    let Some(provider) = state.provider.clone() else {
        return if manual {
            Err("Activa la búsqueda automática en ajustes.".into())
        } else {
            Ok(())
        };
    };
    let Some(track) = state
        .snapshot
        .search_override
        .clone()
        .or_else(|| state.snapshot.track.clone())
    else {
        return if manual {
            Err("Selecciona una sesión con una pista.".into())
        } else {
            Ok(())
        };
    };
    state.cancel_search();
    // A manual search always asks the provider again; an automatic one reuses this run's result.
    if !manual && let Some(candidates) = state.cache.get(&track) {
        state.apply_search(&track, Ok(candidates));
        emit(app, &mut state);
        return Ok(());
    }
    let request = state.request;
    let generation = state.snapshot.generation;
    state.snapshot.lyrics_status = LyricsStatus::Loading;
    state.snapshot.error = None;
    emit(app, &mut state);
    let app = app.clone();
    let task_shared = shared.clone();
    state.pending = Some(tauri::async_runtime::spawn(async move {
        if !manual {
            // Aborted by `cancel_search` when the track changes before the wait ends.
            tokio::time::sleep(SEARCH_DEBOUNCE).await;
        }
        let result = provider.search(&track).await;
        let mut state = task_shared.guard();
        if state.snapshot.generation != generation
            || state.request != request
            || state.local_override
            || !state.snapshot.settings.network_allowed
        {
            return;
        }
        if let Ok(candidates) = &result {
            state.cache.insert(&track, candidates);
        }
        state.apply_search(&track, result);
        emit(&app, &mut state);
    }));
    Ok(())
}

fn apply_sample(
    app: &tauri::AppHandle,
    shared: &Shared,
    sample: MediaSample,
    selection: Option<String>,
    now: Instant,
) {
    let mut state = shared.guard();
    if state.snapshot.selected_session != selection {
        return;
    }
    let changed = track_changed(state.snapshot.track.as_ref(), sample.track.as_ref());
    if changed {
        state.reset_track();
    }
    // Only a real change publishes the snapshot; an unchanged sample just moves the clock.
    let mut dirty = changed;
    let artwork_revision = state.snapshot.artwork_revision;
    state.set_artwork(sample.artwork);
    dirty |= state.snapshot.artwork_revision != artwork_revision;
    if state.snapshot.track != sample.track {
        state.snapshot.track = sample.track;
        dirty = true;
    }
    if state.snapshot.sessions != sample.sessions {
        state.snapshot.sessions = sample.sessions;
        dirty = true;
    }
    if let Some(error) = sample.error
        && state.snapshot.error.as_ref() != Some(&error)
    {
        state.snapshot.error = Some(error);
        dirty = true;
    }
    dirty |= state.snapshot.playing != sample.timeline.playing;
    state.snapshot.playing = sample.timeline.playing;
    dirty |= state.snapshot.playback_controls != sample.playback_controls;
    state.snapshot.playback_controls = sample.playback_controls;
    state.clock.update(sample.timeline, now);
    dirty |= state.snapshot.timing_reliable != state.clock.reliable();
    if dirty {
        emit(app, &mut state);
    } else {
        state.update_position(now);
        let tick = state.tick();
        drop(state);
        let _ = app.emit("proyecto-reze:position", tick);
        return;
    }
    drop(state);
    if changed {
        let _ = search(app, shared.clone(), false);
    }
}

fn track_changed(previous: Option<&Track>, next: Option<&Track>) -> bool {
    match (previous, next) {
        (None, None) => false,
        (Some(old), Some(new)) => {
            old.title != new.title
                || old.artist != new.artist
                || old.album != new.album
                || old
                    .duration_ms
                    .zip(new.duration_ms)
                    .is_some_and(|(old, new)| old.abs_diff(new) > 2000)
        }
        _ => true,
    }
}

pub async fn control_playback(
    shared: Shared,
    session_id: String,
    generation: u64,
    action: PlaybackAction,
) -> Result<(), String> {
    let receive = {
        let mut state = shared.guard();
        state.validate_transport(&session_id, generation, action)?;
        if state.transport_busy {
            return Err("Espera a que el reproductor responda a la orden anterior.".into());
        }
        let (reply, receive) = tokio::sync::oneshot::channel();
        state.transport_busy = true;
        state.transport_request = Some(TransportRequest {
            session_id,
            generation,
            action,
            reply,
        });
        let _ = state.wake.try_send(());
        receive
    };
    tokio::time::timeout(Duration::from_secs(6), receive)
        .await
        .map_err(|_| {
            "El reproductor tarda en responder. Comprueba su estado antes de reintentar."
                .to_string()
        })?
        .map_err(|_| "No se pudo completar la orden del reproductor.".to_string())?
}

fn dispatch_transport(shared: &Shared, media: Option<&crate::media::Gsmtc>) {
    let request = shared.guard().transport_request.take();
    let Some(request) = request else {
        return;
    };
    if request.reply.is_closed() {
        shared.guard().transport_busy = false;
        return;
    }
    let validated =
        shared
            .guard()
            .validate_transport(&request.session_id, request.generation, request.action);
    let result = validated
        .and_then(|()| media.ok_or_else(|| "Windows GSMTC no está disponible.".to_string()))
        .and_then(|source| source.control(&request.session_id, request.action));
    let shared = shared.clone();
    tauri::async_runtime::spawn(async move {
        let result = match result {
            Ok(operation) => tokio::time::timeout(Duration::from_secs(4), operation)
                .await
                .unwrap_or_else(|_| {
                    Err(
                        "El reproductor no respondió. Comprueba su estado antes de reintentar."
                            .into(),
                    )
                }),
            Err(error) => Err(error),
        };
        let mut state = shared.guard();
        state.transport_busy = false;
        let _ = state.wake.try_send(());
        drop(state);
        let _ = request.reply.send(result);
    });
}

pub fn start(app: tauri::AppHandle, shared: Shared, rx: Receiver<()>) {
    std::thread::spawn(move || {
        // WinRT session objects and subscriptions stay on this worker; the UI never blocks on COM.
        let initialized = unsafe {
            windows::Win32::System::WinRT::RoInitialize(
                windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
            )
        }
        .is_ok();
        let mut media = None;
        let mut next_sample = Instant::now();
        let mut last_progress_second = 0;
        // What the interface was last told, so a line change is reported exactly once
        // whichever path noticed it first.
        let mut last_line = None;
        let mut timeout = Duration::ZERO;
        loop {
            // Sleeps until Windows reports a change, the next sample is due or the next line
            // starts. A hidden or paused widget wakes only to sample.
            let event = rx.recv_timeout(timeout);
            let now = Instant::now();
            let visible = app
                .get_webview_window("main")
                .is_some_and(|w| w.is_visible().unwrap_or(false));
            if event.is_ok() || now >= next_sample {
                if media.is_none() {
                    let tx = shared.guard().wake.clone();
                    match crate::media::Gsmtc::new(tx) {
                        Ok(source) => media = Some(source),
                        Err(error) => {
                            let message = format!(
                                "GSMTC no está disponible ({}). Se reintentará.",
                                error.code()
                            );
                            let mut state = shared.guard();
                            if state.snapshot.error.as_ref() != Some(&message) {
                                state.snapshot.error = Some(message);
                                emit(&app, &mut state);
                            }
                        }
                    }
                }
                dispatch_transport(&shared, media.as_ref());
                if let Some(source) = media.as_mut() {
                    let selection = shared.guard().snapshot.selected_session.clone();
                    match source.sample(selection.as_deref()) {
                        Ok(sample) => {
                            apply_sample(&app, &shared, sample, selection, Instant::now())
                        }
                        Err(error) => {
                            let mut state = shared.guard();
                            let was_reliable = state.clock.reliable();
                            state.clock = PlaybackClock::default();
                            state.snapshot.timing_reliable = false;
                            let controls = PlaybackControls::default();
                            if was_reliable
                                || state.snapshot.error.as_ref() != Some(&error)
                                || state.snapshot.playback_controls != controls
                            {
                                state.snapshot.playback_controls = controls;
                                state.snapshot.error = Some(error);
                                emit(&app, &mut state);
                            }
                        }
                    }
                }
                next_sample = Instant::now() + Duration::from_secs(if visible { 1 } else { 5 });
            }
            let mut wait = None;
            if visible {
                let mut state = shared.guard();
                let now = Instant::now();
                state.update_position(now);
                let position = state.snapshot.position_ms;
                let changed = last_line != state.snapshot.active_line
                    || position / 1000 != last_progress_second;
                last_line = state.snapshot.active_line;
                last_progress_second = position / 1000;
                // Only a tiny clock payload; never copy lyrics or query HTTP per frame.
                let tick = (changed || state.word_highlight()).then(|| state.tick());
                wait = state.next_position_wait(now);
                drop(state);
                if let Some(tick) = tick {
                    let _ = app.emit("proyecto-reze:position", tick);
                }
            }
            let until_sample = next_sample.saturating_duration_since(Instant::now());
            timeout = wait.map_or(until_sample, |wait| wait.min(until_sample));
            if Arc::strong_count(&shared) == 1 {
                break;
            }
        }
        drop(media);
        if initialized {
            unsafe { windows::Win32::System::WinRT::RoUninitialize() };
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> Shared {
        let (tx, _) = std::sync::mpsc::sync_channel(1);
        create(Settings::default(), PathBuf::from("unused"), None, tx)
    }
    #[test]
    fn transport_requires_the_selected_session_current_generation_and_supported_action() {
        let (tx, _) = std::sync::mpsc::sync_channel(1);
        let shared = create(Settings::default(), PathBuf::from("unused"), None, tx);
        let mut state = shared.guard();
        state.snapshot.selected_session = Some("selected".into());
        state.snapshot.track = Some(proyecto_reze_core::model::Track::default());
        state.snapshot.playback_controls = PlaybackControls {
            play: true,
            pause: true,
            previous: false,
            next: true,
        };
        let generation = state.snapshot.generation;
        assert!(
            state
                .validate_transport("selected", generation, PlaybackAction::Pause)
                .is_ok()
        );
        assert!(
            state
                .validate_transport("other", generation, PlaybackAction::Next)
                .is_err()
        );
        assert!(
            state
                .validate_transport("selected", generation + 1, PlaybackAction::Next)
                .is_err()
        );
        assert!(
            state
                .validate_transport("selected", generation, PlaybackAction::Previous)
                .is_err()
        );
        state.reset_track();
        assert_eq!(
            state.snapshot.playback_controls,
            PlaybackControls::default()
        );
        assert!(
            state
                .validate_transport("selected", generation, PlaybackAction::Play)
                .is_err()
        );
    }

    #[tokio::test]
    async fn transport_refuses_a_second_pending_command() {
        let (tx, _) = std::sync::mpsc::sync_channel(1);
        let shared = create(Settings::default(), PathBuf::from("unused"), None, tx);
        {
            let mut state = shared.guard();
            state.snapshot.selected_session = Some("selected".into());
            state.snapshot.track = Some(proyecto_reze_core::model::Track::default());
            state.snapshot.playback_controls.next = true;
            state.transport_busy = true;
        }
        assert!(
            control_playback(shared.clone(), "selected".into(), 0, PlaybackAction::Next)
                .await
                .is_err()
        );
        assert!(shared.guard().transport_request.is_none());
    }
    #[test]
    fn late_duration_does_not_invalidate_lyrics_but_other_versions_do() {
        let old = Track {
            title: "Own song".into(),
            artist: "Proyecto Reze".into(),
            ..Default::default()
        };
        let mut updated = old.clone();
        updated.duration_ms = Some(90_000);
        assert!(!track_changed(Some(&old), Some(&updated)));
        let known = updated.clone();
        updated.duration_ms = Some(90_050);
        assert!(!track_changed(Some(&known), Some(&updated)));
        updated.duration_ms = Some(120_000);
        assert!(track_changed(Some(&known), Some(&updated)));
        updated = known.clone();
        updated.title = "Another song".into();
        assert!(track_changed(Some(&known), Some(&updated)));
        assert!(track_changed(Some(&old), None));
    }
    #[test]
    fn track_reset_clears_all_previous_content_and_invalidates_requests() {
        let shared = state();
        let mut state = shared.guard();
        state.snapshot.lyrics = Some(lrc::parse(b"[00:01.00]Own lyrics", "test").unwrap());
        state.local_override = true;
        state.set_artwork(Some("data:image/png;base64,AA==".into()));
        let request = state.request;
        let generation = state.snapshot.generation;
        let artwork = state.snapshot.artwork_revision;
        state.reset_track();
        assert!(state.snapshot.lyrics.is_none());
        assert!(!state.local_override);
        assert!(state.request > request);
        assert!(state.snapshot.generation > generation);
        assert!(state.artwork.is_none() && !state.snapshot.has_artwork);
        assert!(state.snapshot.artwork_revision > artwork);
    }
    #[test]
    fn artwork_revision_changes_only_with_its_content() {
        let shared = state();
        let mut state = shared.guard();
        let image: Arc<str> = "data:image/png;base64,AA==".into();
        state.set_artwork(Some(image.clone()));
        let revision = state.snapshot.artwork_revision;
        state.set_artwork(Some(image));
        state.set_artwork(Some("data:image/png;base64,AA==".into()));
        assert_eq!(state.snapshot.artwork_revision, revision);
        state.set_artwork(Some("data:image/png;base64,BB==".into()));
        assert_eq!(state.snapshot.artwork_revision, revision + 1);
        assert!(state.snapshot.has_artwork);
    }
    #[test]
    fn snapshot_lists_alternatives_without_their_lyrics() {
        let shared = state();
        let mut state = shared.guard();
        let candidate: Candidate = serde_json::from_value(serde_json::json!({
            "id": 7, "trackName": "Own song", "artistName": "Proyecto Reze", "duration": 90,
            "syncedLyrics": "[00:01.00]Own secret line"
        }))
        .unwrap();
        let track = Track {
            title: "Own song".into(),
            artist: "Proyecto Reze".into(),
            duration_ms: Some(90_000),
            ..Default::default()
        };
        state.apply_search(&track, Ok(vec![candidate]));
        assert_eq!(state.snapshot.lyrics_status, LyricsStatus::Ready);
        assert_eq!(state.snapshot.candidates[0].id, 7);
        assert_eq!(state.candidates[0].id, 7);
        let published = serde_json::to_string(&state.snapshot.candidates).unwrap();
        assert!(!published.contains("secret"));
        assert!(published.contains("trackName"));
    }
    #[test]
    fn geometry_is_saved_once_after_changes_and_flushed_on_demand() {
        let dir = std::env::temp_dir().join(format!("reze-geometry-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("settings.json");
        let (tx, _) = std::sync::mpsc::sync_channel(1);
        let shared = create(Settings::default(), path.clone(), None, tx);
        let mut state = shared.guard();
        // The first change asks for a task; later ones reuse it and write nothing.
        assert!(state.geometry_changed(10, 20, 420.0, 260.0));
        assert!(!state.geometry_changed(11, 20, 420.0, 260.0));
        assert!(!state.geometry_changed(11, 20, 420.0, 260.0));
        assert!(!path.exists());
        state.flush_geometry();
        let saved = proyecto_reze_core::settings::load(&path).0;
        assert_eq!((saved.x, saved.y), (Some(11), Some(20)));
        std::fs::remove_file(&path).unwrap();
        state.flush_geometry();
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn worker_sleeps_until_the_next_line_and_not_at_all_when_paused() {
        use proyecto_reze_core::model::Timeline;
        let shared = state();
        let mut state = shared.guard();
        let now = Instant::now();
        assert_eq!(state.next_position_wait(now), None);
        state.snapshot.lyrics =
            Some(lrc::parse(b"[00:01.00]Own\n[00:01.30]lyrics", "test").unwrap());
        let mut timeline = Timeline {
            position_ms: 1000,
            duration_ms: Some(60_000),
            playing: true,
            rate: 1.0,
            reliable: true,
            source_updated: 1,
            ..Default::default()
        };
        state.clock.update(timeline.clone(), now);
        state.update_position(now);
        assert_eq!(
            state.next_position_wait(now),
            Some(Duration::from_millis(300))
        );
        // A positive offset advances the lyrics, so the next line arrives sooner.
        state.snapshot.settings.offset_ms = 200;
        state.update_position(now);
        assert_eq!(
            state.next_position_wait(now),
            Some(Duration::from_millis(100))
        );
        timeline.playing = false;
        timeline.source_updated = 2;
        state.clock.update(timeline, now);
        assert_eq!(state.next_position_wait(now), None);
    }
}
