use proyecto_reze_core::{
    clock::PlaybackClock,
    lrc,
    lyrics::{Lrclib, LyricsProvider, automatic_match},
    model::{LyricFormat, MediaSample, MediaSource, Snapshot},
    settings::Settings,
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, SyncSender},
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

pub type Shared = Arc<Mutex<AppState>>;
pub struct AppState {
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
}
impl AppState {
    pub fn update_position(&mut self, now: Instant) {
        self.snapshot.position_ms = self.clock.position(now);
        self.snapshot.timing_reliable = self.clock.reliable();
        let effective = (self.snapshot.position_ms.min(i64::MAX as u64) as i64)
            .saturating_add(self.snapshot.settings.offset_ms);
        self.snapshot.active_line = self
            .snapshot
            .lyrics
            .as_ref()
            .filter(|d| {
                d.format == LyricFormat::LineSynced
                    && self.clock.reliable()
                    && self.snapshot.sync_confirmed
            })
            .and_then(|d| lrc::active_line(&d.lines, effective));
        self.snapshot.position_sequence += 1;
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
        self.snapshot.translation_status = "idle".into();
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
        self.snapshot.lyrics_status = "ready".into();
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
        self.snapshot.candidates.clear();
        self.snapshot.artwork = None;
        self.snapshot.error = None;
        self.snapshot.lyrics_status = "missing".into();
        self.snapshot.playing = false;
        self.snapshot.position_ms = 0;
        self.snapshot.timing_reliable = false;
        self.local_override = false;
        self.clock = PlaybackClock::default();
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
        snapshot: Snapshot {
            position_sequence: 0,
            search_override: None,
            selected_candidate_id: None,
            lyrics_revision: 0,
            sync_confirmed: true,
            selection_notice: None,
            translation: None,
            translation_status: "idle".into(),
            language: None,
            revision: 0,
            generation: 0,
            sessions: vec![],
            selected_session: None,
            track: None,
            playing: false,
            position_ms: 0,
            timing_reliable: false,
            artwork: None,
            lyrics: None,
            active_line: None,
            lyrics_status: "missing".into(),
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
    }))
}
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
pub fn search(app: &tauri::AppHandle, shared: Shared, manual: bool) -> Result<(), String> {
    let mut state = shared.lock().unwrap();
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
    let request = state.request;
    let generation = state.snapshot.generation;
    state.snapshot.lyrics_status = "loading".into();
    state.snapshot.error = None;
    emit(app, &mut state);
    let app = app.clone();
    let task_shared = shared.clone();
    state.pending = Some(tauri::async_runtime::spawn(async move {
        let result = provider.search(&track).await;
        let mut state = task_shared.lock().unwrap();
        if state.snapshot.generation != generation
            || state.request != request
            || state.local_override
            || !state.snapshot.settings.network_allowed
        {
            return;
        }
        match result {
            Ok(candidates) => {
                let safe = automatic_match(&track, &candidates);
                let selected = safe
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
                        automatic_match(&track, std::slice::from_ref(candidate)).is_some();
                    state.install_lyrics(document, Some(candidate.id), confirmed);
                    state.snapshot.selection_notice = notice;
                    state.snapshot.candidates = candidates;
                } else {
                    state.cancel_translation();
                    state.snapshot.lyrics = None;
                    state.snapshot.active_line = None;
                    state.snapshot.candidates = candidates;
                    state.snapshot.selected_candidate_id = None;
                    state.snapshot.lyrics_status = "missing".into();
                }
            }
            Err(error) => {
                state.snapshot.error = Some(error);
                state.snapshot.lyrics_status = "error".into();
            }
        }
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
    let mut state = shared.lock().unwrap();
    if state.snapshot.selected_session != selection {
        return;
    }
    let changed = track_changed(state.snapshot.track.as_ref(), sample.track.as_ref());
    if changed {
        state.reset_track();
    }
    state.snapshot.track = sample.track;
    state.snapshot.sessions = sample.sessions;
    state.snapshot.artwork = sample.artwork;
    if let Some(error) = sample.error {
        state.snapshot.error = Some(error);
    }
    state.snapshot.playing = sample.timeline.playing;
    state.clock.update(sample.timeline, now);
    state.snapshot.timing_reliable = state.clock.reliable();
    state.snapshot.position_ms = state.clock.position(now);
    emit(app, &mut state);
    drop(state);
    if changed {
        let _ = search(app, shared.clone(), false);
    }
}

fn track_changed(
    previous: Option<&proyecto_reze_core::model::Track>,
    next: Option<&proyecto_reze_core::model::Track>,
) -> bool {
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
        loop {
            let event = rx.recv_timeout(Duration::from_millis(33));
            let now = Instant::now();
            let visible = app
                .get_webview_window("main")
                .is_some_and(|w| w.is_visible().unwrap_or(false));
            if event.is_ok() || now >= next_sample {
                if media.is_none() {
                    let tx = shared.lock().unwrap().wake.clone();
                    match crate::media::Gsmtc::new(tx) {
                        Ok(source) => media = Some(source),
                        Err(error) => {
                            let mut state = shared.lock().unwrap();
                            state.snapshot.error = Some(format!(
                                "GSMTC no está disponible ({}). Se reintentará.",
                                error.code()
                            ));
                            emit(&app, &mut state);
                        }
                    }
                }
                if let Some(source) = media.as_mut() {
                    let selection = shared.lock().unwrap().snapshot.selected_session.clone();
                    match source.sample(selection.as_deref()) {
                        Ok(sample) => {
                            apply_sample(&app, &shared, sample, selection, Instant::now())
                        }
                        Err(error) => {
                            let mut state = shared.lock().unwrap();
                            state.clock = PlaybackClock::default();
                            state.snapshot.timing_reliable = false;
                            state.snapshot.error = Some(error);
                            emit(&app, &mut state);
                        }
                    }
                }
                next_sample = Instant::now() + Duration::from_secs(if visible { 1 } else { 5 });
            }
            if visible {
                let mut state = shared.lock().unwrap();
                let previous_line = state.snapshot.active_line;
                state.update_position(Instant::now());
                let position = state.snapshot.position_ms;
                let changed = previous_line != state.snapshot.active_line
                    || position / 1000 != last_progress_second;
                last_progress_second = position / 1000;
                if changed {
                    emit(&app, &mut state);
                }
                let words = state.snapshot.settings.karaoke_mode
                    == proyecto_reze_core::settings::KaraokeMode::Word
                    && state.snapshot.sync_confirmed
                    && state.snapshot.timing_reliable
                    && state
                        .snapshot
                        .active_line
                        .and_then(|index| state.snapshot.lyrics.as_ref()?.lines.get(index))
                        .is_some_and(|line| !line.words.is_empty());
                let tick=words.then(||serde_json::json!({"generation":state.snapshot.generation,"lyrics_revision":state.snapshot.lyrics_revision,"sequence":state.snapshot.position_sequence,"position_ms":position,"active_line":state.snapshot.active_line,"playing":state.snapshot.playing,"timing_reliable":state.snapshot.timing_reliable}));
                drop(state);
                // Only a tiny clock payload at 30 Hz; never copy lyrics or query HTTP per frame.
                if let Some(tick) = tick {
                    let _ = app.emit("proyecto-reze:position", tick);
                }
            }
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
    #[test]
    fn late_duration_does_not_invalidate_lyrics_but_other_versions_do() {
        let old = proyecto_reze_core::model::Track {
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
        let (tx, _) = std::sync::mpsc::sync_channel(1);
        let shared = create(Settings::default(), PathBuf::from("unused"), None, tx);
        let mut state = shared.lock().unwrap();
        state.snapshot.lyrics = Some(lrc::parse(b"[00:01.00]Own lyrics", "test").unwrap());
        state.local_override = true;
        let request = state.request;
        let generation = state.snapshot.generation;
        state.reset_track();
        assert!(state.snapshot.lyrics.is_none());
        assert!(!state.local_override);
        assert!(state.request > request);
        assert!(state.snapshot.generation > generation);
    }
}
