use base64::Engine;
use proyecto_reze_core::model::{
    MediaSample, MediaSource, PlaybackAction, PlaybackControls, Session, Timeline, Track,
};
use std::sync::{Arc, mpsc::SyncSender};
use windows::{Foundation::TypedEventHandler, Media::Control::*, Storage::Streams::DataReader};

pub struct Gsmtc {
    manager: GlobalSystemMediaTransportControlsSessionManager,
    manager_tokens: Vec<(bool, i64)>,
    sessions: Vec<(String, GlobalSystemMediaTransportControlsSession)>,
    selected: Option<GlobalSystemMediaTransportControlsSession>,
    tokens: Vec<(u8, i64)>,
    wake: SyncSender<()>,
    next_id: u64,
    last_track: Option<Track>,
    artwork: Option<Arc<str>>,
}
impl Gsmtc {
    /// Start the request on the WinRT owner thread; await it outside the sampler
    /// so an unresponsive player cannot stop the lyrics clock or block the UI.
    pub fn control(
        &self,
        id: &str,
        action: PlaybackAction,
    ) -> Result<impl std::future::Future<Output = Result<(), String>> + Send + 'static, String>
    {
        let session = self
            .sessions
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, session)| session)
            .ok_or("La sesión elegida ya no está disponible.")?;
        let attempt = (|| -> windows::core::Result<_> {
            if !self
                .manager
                .GetSessions()?
                .into_iter()
                .any(|current| &current == session)
            {
                return Err(windows::core::Error::from(windows::core::HRESULT(
                    0x8000000e_u32 as i32,
                )));
            }
            let info = session.GetPlaybackInfo()?;
            let controls = info.Controls()?;
            let toggle = controls.IsPlayPauseToggleEnabled().unwrap_or(false);
            let playing = info.PlaybackStatus()?
                == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
            let operation = match action {
                PlaybackAction::Play if controls.IsPlayEnabled().unwrap_or(false) || toggle => {
                    if playing {
                        None
                    } else if controls.IsPlayEnabled().unwrap_or(false) {
                        Some(session.TryPlayAsync()?)
                    } else {
                        Some(session.TryTogglePlayPauseAsync()?)
                    }
                }
                PlaybackAction::Pause if controls.IsPauseEnabled().unwrap_or(false) || toggle => {
                    if !playing {
                        None
                    } else if controls.IsPauseEnabled().unwrap_or(false) {
                        Some(session.TryPauseAsync()?)
                    } else {
                        Some(session.TryTogglePlayPauseAsync()?)
                    }
                }
                PlaybackAction::Previous if controls.IsPreviousEnabled().unwrap_or(false) => {
                    Some(session.TrySkipPreviousAsync()?)
                }
                PlaybackAction::Next if controls.IsNextEnabled().unwrap_or(false) => {
                    Some(session.TrySkipNextAsync()?)
                }
                _ => {
                    return Err(windows::core::Error::from(windows::core::HRESULT(
                        0x80004001_u32 as i32,
                    )));
                }
            };
            Ok(operation)
        })();
        let operation = attempt
            .map_err(|_| "La sesión ya no está disponible o no admite este control.".to_string())?;
        Ok(async move {
            if let Some(operation) = operation {
                let accepted = operation
                    .await
                    .map_err(|_| "Windows no pudo enviar la orden al reproductor.".to_string())?;
                if !accepted {
                    return Err("El reproductor no aceptó la orden. Comprueba su estado e inténtalo de nuevo.".into());
                }
            }
            Ok(())
        })
    }
    pub fn new(wake: SyncSender<()>) -> windows::core::Result<Self> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.join()?;
        let tx = wake.clone();
        let sessions = manager.SessionsChanged(&TypedEventHandler::new(move |_, _| {
            let _ = tx.try_send(());
            Ok(())
        }))?;
        let tx = wake.clone();
        let current = manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
            let _ = tx.try_send(());
            Ok(())
        }))?;
        Ok(Self {
            manager,
            manager_tokens: vec![(false, sessions), (true, current)],
            sessions: vec![],
            selected: None,
            tokens: vec![],
            wake,
            next_id: 1,
            last_track: None,
            artwork: None,
        })
    }
    fn unsubscribe(&mut self) {
        if let Some(session) = &self.selected {
            for (kind, token) in self.tokens.drain(..) {
                let _ = match kind {
                    0 => session.RemoveMediaPropertiesChanged(token),
                    1 => session.RemovePlaybackInfoChanged(token),
                    _ => session.RemoveTimelinePropertiesChanged(token),
                };
            }
        }
        self.selected = None;
    }
    fn subscribe(&mut self, session: GlobalSystemMediaTransportControlsSession) {
        self.unsubscribe();
        let tx = self.wake.clone();
        if let Ok(token) = session.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
            let _ = tx.try_send(());
            Ok(())
        })) {
            self.tokens.push((0, token));
        }
        let tx = self.wake.clone();
        if let Ok(token) = session.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
            let _ = tx.try_send(());
            Ok(())
        })) {
            self.tokens.push((1, token));
        }
        let tx = self.wake.clone();
        if let Ok(token) =
            session.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
                let _ = tx.try_send(());
                Ok(())
            }))
        {
            self.tokens.push((2, token));
        }
        self.selected = Some(session);
        self.last_track = None;
        self.artwork = None;
    }
    fn read(&mut self, selected: Option<&str>) -> windows::core::Result<MediaSample> {
        let sessions = self.manager.GetSessions()?;
        let mut next = Vec::new();
        for session in sessions {
            let id = if let Some((id, _)) = self.sessions.iter().find(|(_, old)| old == &session) {
                id.clone()
            } else {
                let id = format!("session-{}", self.next_id);
                self.next_id += 1;
                id
            };
            next.push((id, session));
        }
        self.sessions = next;
        let listing = self
            .sessions
            .iter()
            .map(|(id, session)| Session {
                id: id.clone(),
                name: session
                    .SourceAppUserModelId()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| "Reproductor".into()),
            })
            .collect();
        let target = self
            .sessions
            .iter()
            .find(|(id, _)| Some(id.as_str()) == selected)
            .map(|(_, s)| s.clone());
        if self.selected != target {
            if let Some(target) = target {
                self.subscribe(target);
            } else {
                self.unsubscribe();
                self.last_track = None;
                self.artwork = None;
            }
        }
        let Some(session) = &self.selected else {
            return Ok(MediaSample {
                sessions: listing,
                ..Default::default()
            });
        };
        let properties = session.TryGetMediaPropertiesAsync()?.join()?;
        let playback = session.GetPlaybackInfo()?;
        let playback_controls = playback
            .Controls()
            .ok()
            .map(|controls| {
                let toggle = controls.IsPlayPauseToggleEnabled().unwrap_or(false);
                PlaybackControls {
                    play: controls.IsPlayEnabled().unwrap_or(false) || toggle,
                    pause: controls.IsPauseEnabled().unwrap_or(false) || toggle,
                    previous: controls.IsPreviousEnabled().unwrap_or(false),
                    next: controls.IsNextEnabled().unwrap_or(false),
                }
            })
            .unwrap_or_default();
        let playing = playback.PlaybackStatus()?
            == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
        let rate = playback
            .PlaybackRate()
            .and_then(|r| r.Value())
            .unwrap_or(1.0);
        let rate = if rate.is_finite() && rate > 0.0 {
            rate
        } else {
            1.0
        };
        let timeline = session
            .GetTimelineProperties()
            .ok()
            .map(|timeline| {
                let start = timeline.StartTime().map(|t| t.Duration).unwrap_or(0);
                let end = timeline.EndTime().map(|t| t.Duration).unwrap_or(0);
                let position = timeline.Position().map(|t| t.Duration).unwrap_or(0);
                let updated = timeline
                    .LastUpdatedTime()
                    .map(|t| t.UniversalTime)
                    .unwrap_or(0);
                let now = windows_utc_ticks();
                let age_ms = now.saturating_sub(updated).max(0) as u64 / 10_000;
                Timeline {
                    position_ms: position.saturating_sub(start).max(0) as u64 / 10_000,
                    duration_ms: (end > start)
                        .then_some(end.saturating_sub(start).max(0) as u64 / 10_000),
                    playing,
                    rate,
                    reliable: updated > 0
                        && updated <= now + 10_000_000
                        && age_ms < 300_000
                        && end > start
                        && position >= start
                        && position <= end,
                    source_updated: updated,
                    age_ms,
                }
            })
            .unwrap_or(Timeline {
                playing,
                rate,
                ..Default::default()
            });
        let track = Track {
            title: properties
                .Title()
                .map(|s| s.to_string())
                .unwrap_or_default(),
            artist: properties
                .Artist()
                .map(|s| s.to_string())
                .unwrap_or_default(),
            album: properties
                .AlbumTitle()
                .map(|s| s.to_string())
                .unwrap_or_default(),
            duration_ms: timeline.duration_ms,
        };
        if self.last_track.as_ref() != Some(&track) {
            self.artwork = properties
                .Thumbnail()
                .ok()
                .and_then(|thumbnail| thumbnail.OpenReadAsync().ok()?.join().ok())
                .and_then(|stream| {
                    let size = stream.Size().ok()?;
                    if size == 0 || size > 2 * 1024 * 1024 {
                        return None;
                    }
                    let mime = stream.ContentType().ok()?.to_string();
                    if !["image/jpeg", "image/png", "image/webp"].contains(&mime.as_str()) {
                        return None;
                    }
                    let reader = DataReader::CreateDataReader(&stream).ok()?;
                    let count = reader.LoadAsync(size as u32).ok()?.join().ok()?;
                    let mut bytes = vec![0; count as usize];
                    reader.ReadBytes(&mut bytes).ok()?;
                    Some(Arc::from(format!(
                        "data:{mime};base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    )))
                });
            self.last_track = Some(track.clone());
        }
        Ok(MediaSample {
            playback_controls,
            sessions: listing,
            track: Some(track),
            timeline,
            artwork: self.artwork.clone(),
            error: None,
        })
    }
}
fn windows_utc_ticks() -> i64 {
    const EPOCH: i64 = 116_444_736_000_000_000;
    EPOCH
        + (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            / 100) as i64
}
impl MediaSource for Gsmtc {
    fn sample(&mut self, selected: Option<&str>) -> Result<MediaSample, String> {
        self.read(selected)
            .map_err(|e| format!("Windows no pudo leer la sesión multimedia ({}).", e.code()))
    }
}
impl Drop for Gsmtc {
    fn drop(&mut self) {
        self.unsubscribe();
        for (current, token) in self.manager_tokens.drain(..) {
            let _ = if current {
                self.manager.RemoveCurrentSessionChanged(token)
            } else {
                self.manager.RemoveSessionsChanged(token)
            };
        }
    }
}
