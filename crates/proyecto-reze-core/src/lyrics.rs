use crate::{
    lrc,
    model::{LyricFormat, LyricsDocument, Track},
};
use serde::{Deserialize, Serialize};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::{Mutex, Semaphore};

/// Honest identity for this personal build; no user contact or invented project URL.
pub const CLIENT_IDENTIFICATION: &str = concat!(
    "ProyectoReze/",
    env!("CARGO_PKG_VERSION"),
    " (Windows desktop; app.proyectoreze.widget)"
);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: i64,
    #[serde(default)]
    pub track_name: String,
    #[serde(default)]
    pub artist_name: String,
    #[serde(default)]
    pub album_name: String,
    #[serde(default)]
    pub duration: f64,
    #[serde(default)]
    pub instrumental: bool,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    #[serde(default)]
    pub lyricsfile: Option<String>,
    #[serde(default)]
    pub has_word_sync: bool,
    #[serde(default)]
    pub match_note: Option<String>,
}
/// What the interface needs to list an alternative, without its lyrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CandidateSummary {
    pub id: i64,
    pub track_name: String,
    pub artist_name: String,
    pub album_name: String,
    pub duration: f64,
    pub instrumental: bool,
    pub match_note: Option<String>,
}
impl From<&Candidate> for CandidateSummary {
    fn from(candidate: &Candidate) -> Self {
        Self {
            id: candidate.id,
            track_name: candidate.track_name.clone(),
            artist_name: candidate.artist_name.clone(),
            album_name: candidate.album_name.clone(),
            duration: candidate.duration,
            instrumental: candidate.instrumental,
            match_note: candidate.match_note.clone(),
        }
    }
}
impl Candidate {
    pub fn document(&self) -> Result<LyricsDocument, String> {
        let mut file_warning = None;
        if !self.instrumental
            && let Some(raw) = self.lyricsfile.as_deref().filter(|s| !s.trim().is_empty())
        {
            match crate::lyricsfile::parse(raw) {
                Ok(mut document) => {
                    document.source_id = Some(self.id);
                    return Ok(document);
                }
                Err(error) => {
                    file_warning =
                        Some(format!("{error} Se usará la letra alternativa disponible."))
                }
            }
        }
        let mut doc = if self.instrumental {
            LyricsDocument {
                source: "LRCLIB".into(),
                source_id: Some(self.id),
                format: LyricFormat::Instrumental,
                lines: vec![],
                plain_text: String::new(),
                warnings: vec![],
                rights_metadata: None,
            }
        } else if let Some(synced) = self.synced_lyrics.as_ref().filter(|s| !s.trim().is_empty()) {
            lrc::parse(synced.as_bytes(), "LRCLIB")?
        } else if let Some(plain) = self.plain_lyrics.as_ref().filter(|s| !s.trim().is_empty()) {
            LyricsDocument {
                source: "LRCLIB".into(),
                source_id: Some(self.id),
                format: LyricFormat::Plain,
                lines: vec![],
                plain_text: plain.clone(),
                warnings: vec![],
                rights_metadata: None,
            }
        } else {
            return Err("El resultado no contiene letras.".into());
        };
        doc.source_id = Some(self.id);
        if let Some(warning) = file_warning {
            doc.warnings.push(warning);
        }
        Ok(doc)
    }
}
/// Results kept in memory for this run only, so returning to a recent track does not
/// repeat its queries. Nothing is written to disk.
#[derive(Debug, Default)]
pub struct SearchCache {
    entries: std::collections::VecDeque<(Track, Vec<Candidate>)>,
}
impl SearchCache {
    pub const CAPACITY: usize = 16;
    pub fn get(&self, track: &Track) -> Option<Vec<Candidate>> {
        self.entries
            .iter()
            .find(|(key, _)| key == track)
            .map(|(_, candidates)| candidates.clone())
    }
    /// Empty results are not remembered: a later search may find newly published lyrics.
    pub fn insert(&mut self, track: &Track, candidates: &[Candidate]) {
        if candidates.is_empty() {
            return;
        }
        self.entries.retain(|(key, _)| key != track);
        if self.entries.len() == Self::CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back((track.clone(), candidates.to_vec()));
    }
}
pub use crate::matching::automatic_match;
pub trait LyricsProvider: Send + Sync {
    fn search(&self, track: &Track) -> impl Future<Output = Result<Vec<Candidate>, String>> + Send;
    fn fetch(&self, id: i64) -> impl Future<Output = Result<LyricsDocument, String>> + Send;
}

#[derive(Clone)]
pub struct Lrclib {
    client: reqwest::Client,
    base: String,
    gate: Arc<Semaphore>,
    cooldown: Arc<Mutex<tokio::time::Instant>>,
}
impl Lrclib {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .user_agent(CLIENT_IDENTIFICATION)
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            client,
            base: "https://lrclib.net".into(),
            gate: Arc::new(Semaphore::new(2)),
            cooldown: Arc::new(Mutex::new(tokio::time::Instant::now())),
        })
    }
    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<Option<T>, String> {
        let _permit = self
            .gate
            .acquire()
            .await
            .map_err(|_| "Proveedor cerrado.")?;
        for attempt in 0..=2 {
            let wait_until = *self.cooldown.lock().await;
            tokio::time::sleep_until(wait_until).await;
            let response = self
                .client
                .get(format!("{}{path}", self.base))
                .query(query)
                .send()
                .await;
            match response {
                Ok(mut response) => {
                    let status = response.status();
                    if status == reqwest::StatusCode::NOT_FOUND {
                        return Ok(None);
                    }
                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        let seconds = response
                            .headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(60);
                        let delay = Duration::from_secs_f64(
                            seconds as f64 * (1.0 + rand::random::<f64>() * 0.3),
                        );
                        let mut cooldown = self.cooldown.lock().await;
                        *cooldown = (*cooldown).max(tokio::time::Instant::now() + delay);
                        if attempt == 2 {
                            return Err(
                                "LRCLIB limitó las consultas. Espera antes de reintentar.".into()
                            );
                        }
                        continue;
                    }
                    if status.is_server_error() && attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                        continue;
                    }
                    if !status.is_success() {
                        return Err(format!("LRCLIB respondió HTTP {}.", status.as_u16()));
                    }
                    let mut body = Vec::new();
                    while let Some(chunk) = response
                        .chunk()
                        .await
                        .map_err(|_| "No se pudo leer la respuesta de LRCLIB.")?
                    {
                        if body.len() + chunk.len() > 8 * 1024 * 1024 {
                            return Err("Respuesta de letras demasiado grande.".into());
                        }
                        body.extend_from_slice(&chunk);
                    }
                    return serde_json::from_slice(&body).map(Some).map_err(|_| {
                        "LRCLIB devolvió una respuesta incompleta o inválida.".into()
                    });
                }
                Err(_) if attempt < 2 => {
                    tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await
                }
                Err(_) => {
                    return Err(
                        "No se pudo consultar LRCLIB. Revisa tu conexión o importa un LRC.".into(),
                    );
                }
            }
        }
        unreachable!()
    }
}
impl LyricsProvider for Lrclib {
    async fn search(&self, track: &Track) -> Result<Vec<Candidate>, String> {
        use crate::matching::{identity, rank, same_performer, same_title};
        if track.title.trim().is_empty() {
            return Err("Windows no entrega un título. Importa un LRC local.".into());
        }
        let derived = identity(track);
        let mut results = Vec::new();
        let primary_found = |results: &[Candidate]| {
            results.iter().any(|candidate| {
                same_title(&derived, candidate) && same_performer(&derived, candidate)
            })
        };
        let mut query = vec![("track_name".into(), track.title.clone())];
        if !track.artist.trim().is_empty() {
            query.push(("artist_name".into(), track.artist.clone()));
        }
        if !track.album.is_empty() {
            query.push(("album_name".into(), track.album.clone()));
        }
        if let Some(duration) = track.duration_ms.filter(|duration| {
            (1_000..=3_600_000).contains(duration) && !track.artist.trim().is_empty()
        }) {
            query.push(("duration".into(), (duration as f64 / 1000.0).to_string()));
            if let Some(candidate) = self.get::<Candidate>("/api/get", &query).await? {
                results.push(candidate);
                if automatic_match(track, &results).is_some() {
                    rank(track, &mut results);
                    return Ok(results);
                }
            }
            query.pop();
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        let mut plans = vec![query];
        let mut clean = vec![("track_name".into(), derived.title.clone())];
        if !derived.artist.is_empty() {
            clean.push(("artist_name".into(), derived.artist.clone()));
        }
        if plans.last() != Some(&clean) {
            plans.push(clean);
        }
        if !derived.artist.is_empty() {
            plans.push(vec![(
                "q".into(),
                format!("{} {}", derived.title, derived.artist),
            )]);
        }
        if let Some(alternate) = derived
            .alternate_artist
            .as_ref()
            .filter(|artist| **artist != derived.artist)
        {
            plans.push(vec![
                ("track_name".into(), derived.title.clone()),
                ("artist_name".into(), alternate.clone()),
            ]);
        }
        let broad = vec![("track_name".into(), derived.title.clone())];
        if !plans.contains(&broad) {
            plans.push(broad);
        }
        for title in crate::matching::title_variants(&derived) {
            if !derived.artist.is_empty() {
                plans.push(vec![
                    ("track_name".into(), title.clone()),
                    ("artist_name".into(), derived.artist.clone()),
                ]);
            }
            plans.push(vec![("q".into(), title)]);
        }
        // Bounded sequential fallbacks; stop as soon as the performer/title match.
        for (index, query) in plans.iter().take(9).enumerate() {
            if index > 0 {
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            results.extend(
                self.get::<Vec<Candidate>>("/api/search", query)
                    .await?
                    .unwrap_or_default(),
            );
            if primary_found(&results) {
                break;
            }
        }
        rank(track, &mut results);
        Ok(results)
    }
    async fn fetch(&self, id: i64) -> Result<LyricsDocument, String> {
        self.get::<Candidate>(&format!("/api/get/{id}"), &[])
            .await?
            .ok_or("No se encontró esa letra.")?
            .document()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};
    fn client(base: String) -> Lrclib {
        let mut client = Lrclib::new().unwrap();
        client.base = base;
        client
    }
    fn track() -> Track {
        Track {
            title: "Propia".into(),
            artist: "Autor".into(),
            duration_ms: Some(10_000),
            ..Default::default()
        }
    }
    fn record() -> serde_json::Value {
        serde_json::json!({"id":1,"trackName":"Propia","artistName":"Autor","duration":10,"plainLyrics":"Texto propio"})
    }
    #[test]
    fn search_cache_is_bounded_keyed_by_track_and_skips_empty_results() {
        let candidate: Candidate = serde_json::from_value(record()).unwrap();
        let mut cache = SearchCache::default();
        cache.insert(&track(), &[]);
        assert!(cache.get(&track()).is_none());
        cache.insert(&track(), std::slice::from_ref(&candidate));
        assert_eq!(cache.get(&track()).unwrap()[0].id, 1);
        let mut other = track();
        other.duration_ms = Some(20_000);
        assert!(cache.get(&other).is_none());
        for index in 0..SearchCache::CAPACITY {
            other.title = format!("Otra {index}");
            cache.insert(&other, std::slice::from_ref(&candidate));
        }
        assert!(cache.get(&track()).is_none());
        assert!(cache.get(&other).is_some());
    }
    #[tokio::test]
    async fn lrclib_word_payload_is_used_even_when_legacy_fields_are_absent() {
        let server = MockServer::start().await;
        let mut value = record();
        value["plainLyrics"] = serde_json::Value::Null;
        value["hasWordSync"] = true.into();
        value["lyricsfile"] = include_str!("../../../fixtures/karaoke-demo.lyricsfile.yaml").into();
        Mock::given(path("/api/get"))
            .respond_with(ResponseTemplate::new(200).set_body_json(value))
            .mount(&server)
            .await;
        let results = client(server.uri()).search(&track()).await.unwrap();
        let document = results[0].document().unwrap();
        assert_eq!(document.source_id, Some(1));
        assert_eq!(document.lines[0].words[0].start_ms, 1000);
        assert_eq!(document.lines[0].words[0].end_ms, Some(2000));
    }
    #[test]
    fn malformed_lyricsfile_uses_legacy_line_sync_and_plain_fallback() {
        let mut value = record();
        value["lyricsfile"] = "version: '99'".into();
        value["hasWordSync"] = true.into();
        value["syncedLyrics"] = "[00:01.00]Test line".into();
        let candidate: Candidate = serde_json::from_value(value.clone()).unwrap();
        let document = candidate.document().unwrap();
        assert_eq!(document.format, LyricFormat::LineSynced);
        assert!(document.lines[0].words.is_empty());
        assert!(!document.warnings.is_empty());
        value["syncedLyrics"] = serde_json::Value::Null;
        let document = serde_json::from_value::<Candidate>(value)
            .unwrap()
            .document()
            .unwrap();
        assert_eq!(document.format, LyricFormat::Plain);
    }
    #[tokio::test]
    async fn decorated_cover_falls_back_to_clean_title_and_preserves_performer() {
        use wiremock::matchers::query_param;
        let server = MockServer::start().await;
        let title = "Black Clover Opening 5 『Miyuna - Gamushara』Cover [ES]";
        Mock::given(path("/api/get"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/api/search"))
            .and(query_param("track_name", title))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/api/search")).and(query_param("track_name","Gamushara")).and(query_param("artist_name","The Covers Duo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([{"id":12756272,"trackName":"Gamushara","artistName":"The Covers Duo","duration":196,"syncedLyrics":"[00:00.00]Own test text"}]))).expect(1).mount(&server).await;
        let track = Track {
            title: title.into(),
            artist: "The Covers Duo".into(),
            duration_ms: Some(216_121),
            ..Default::default()
        };
        let results = client(server.uri()).search(&track).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].artist_name, "The Covers Duo");
        assert!(results[0].match_note.as_ref().unwrap().contains("20 s"));
        assert_eq!(automatic_match(&track, &results), None);
        assert_eq!(
            results[0].document().unwrap().format,
            LyricFormat::LineSynced
        );
    }
    #[tokio::test]
    async fn missing_artist_can_search_by_title_without_auto_selection() {
        let server = MockServer::start().await;
        Mock::given(path("/api/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([record()])))
            .expect(1)
            .mount(&server)
            .await;
        let mut track = track();
        track.artist.clear();
        let results = client(server.uri()).search(&track).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(automatic_match(&track, &results), None);
    }
    #[tokio::test]
    async fn exact_match_plain_and_ambiguous() {
        let server = MockServer::start().await;
        Mock::given(path("/api/get"))
            .and(wiremock::matchers::header(
                "user-agent",
                CLIENT_IDENTIFICATION,
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(record()))
            .expect(1)
            .mount(&server)
            .await;
        let results = client(server.uri()).search(&track()).await.unwrap();
        assert_eq!(automatic_match(&track(), &results), Some(0));
        assert_eq!(results[0].document().unwrap().format, LyricFormat::Plain);
        assert_eq!(
            automatic_match(&track(), &[results[0].clone(), results[0].clone()]),
            None
        );
        let mut wrong = track();
        wrong.title.push_str(" (Live)");
        assert_eq!(automatic_match(&wrong, &results), None);
    }
    #[tokio::test]
    async fn fallback_empty_and_malformed() {
        let server = MockServer::start().await;
        Mock::given(path("/api/get"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(path("/api/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(Vec::<Candidate>::new()))
            .mount(&server)
            .await;
        assert!(
            client(server.uri())
                .search(&track())
                .await
                .unwrap()
                .is_empty()
        );
        let server = MockServer::start().await;
        Mock::given(path("/api/get"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{"))
            .mount(&server)
            .await;
        assert!(client(server.uri()).search(&track()).await.is_err());
    }
    #[tokio::test]
    async fn rate_limit_is_retried_and_instrumental_supported() {
        let server = MockServer::start().await;
        Mock::given(path("/api/get"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        let mut value = record();
        value["instrumental"] = true.into();
        Mock::given(path("/api/get"))
            .respond_with(ResponseTemplate::new(200).set_body_json(value))
            .with_priority(2)
            .mount(&server)
            .await;
        let records = client(server.uri()).search(&track()).await.unwrap();
        assert_eq!(
            records[0].document().unwrap().format,
            LyricFormat::Instrumental
        );
    }
    #[tokio::test]
    async fn unavailable_service_returns_error() {
        let mut client = client("http://127.0.0.1:1".into());
        client.client = reqwest::Client::builder()
            .timeout(Duration::from_millis(20))
            .build()
            .unwrap();
        assert!(client.search(&track()).await.is_err());
    }
}
