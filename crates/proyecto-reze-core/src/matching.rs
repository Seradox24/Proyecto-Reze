//! Search identities are derived hints; the original Windows metadata is preserved.
use crate::{lyrics::Candidate, model::Track};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

pub fn normalized(value: &str) -> String {
    value
        .nfkd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn artist_name(value: &str) -> String {
    let mut name = value.trim();
    for suffix in ["officialvevo", "vevo", " - topic"] {
        if name.to_lowercase().ends_with(suffix) {
            let prefix = &name[..name.len() - suffix.len()];
            if !prefix.trim().is_empty() {
                name = prefix.trim();
            }
        }
    }
    name.to_owned()
}
fn decoration(value: &str) -> bool {
    matches!(
        normalized(value).as_str(),
        "official video"
            | "official music video"
            | "official audio"
            | "music video"
            | "video oficial"
            | "audio oficial"
            | "official lyric video"
            | "lyric video"
            | "lyrics video"
            | "lyrics"
            | "lyric"
            | "letra"
            | "with lyrics"
            | "hd"
            | "hq"
            | "4k"
            | "visualizer"
    )
}
fn strip_decorations(value: &str) -> String {
    let mut text = value.trim().to_owned();
    for (open, close) in [('(', ')'), ('[', ']')] {
        let mut start = 0;
        while let Some(relative) = text[start..].find(open) {
            let left = start + relative;
            let Some(end) = text[left..].find(close) else {
                break;
            };
            let right = left + end;
            if decoration(&text[left + open.len_utf8()..right]) {
                text.replace_range(left..right + close.len_utf8(), " ");
                start = left;
            } else {
                start = right + close.len_utf8();
            }
        }
    }
    for separator in [" | ", " - "] {
        if let Some((left, right)) = text.rsplit_once(separator)
            && decoration(right)
        {
            text = left.to_owned();
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn sensitive(value: &str) -> bool {
    let words = normalized(value);
    [
        "cover",
        "live",
        "remix",
        "acoustic",
        "instrumental",
        "karaoke",
        "nightcore",
        "slowed",
        "sped",
        "radio edit",
        "espanol",
        "spanish",
        "english",
        "en vivo",
    ]
    .iter()
    .any(|marker| format!(" {words} ").contains(&format!(" {marker} ")))
        || ["[es]", "[en]", "(es)", "(en)"]
            .iter()
            .any(|tag| value.to_lowercase().contains(tag))
}
#[derive(Debug, Clone)]
pub struct Identity {
    pub title: String,
    pub artist: String,
    /// Author named in a cover video's title; never silently replaces its performer.
    pub alternate_artist: Option<String>,
    pub needs_confirmation: bool,
}
pub fn identity(track: &Track) -> Identity {
    let artist = artist_name(&track.artist);
    let mut title = strip_decorations(&track.title);
    let mut quoted = false;
    for (open, close) in [('『', '』'), ('「', '」'), ('“', '”')] {
        if let Some(left) = title.find(open)
            && let Some(end) = title[left + open.len_utf8()..].find(close)
        {
            title = title[left + open.len_utf8()..left + open.len_utf8() + end]
                .trim()
                .to_owned();
            quoted = true;
            break;
        }
    }
    let is_sensitive = sensitive(&track.title);
    for separator in [" - ", " – ", " — "] {
        if let Some((left, right)) = title.split_once(separator) {
            let left = left.trim();
            let right = right.trim();
            if left.is_empty() || right.is_empty() {
                continue;
            }
            if normalized(&artist_name(left)) == normalized(&artist) && !artist.is_empty() {
                return Identity {
                    title: strip_decorations(right),
                    artist,
                    alternate_artist: None,
                    needs_confirmation: is_sensitive,
                };
            }
            if normalized(&artist_name(right)) == normalized(&artist) && !artist.is_empty() {
                return Identity {
                    title: strip_decorations(left),
                    artist,
                    alternate_artist: None,
                    needs_confirmation: is_sensitive,
                };
            }
            if quoted || is_sensitive || artist.is_empty() {
                return Identity {
                    title: strip_decorations(right),
                    artist: if artist.is_empty() {
                        artist_name(left)
                    } else {
                        artist
                    },
                    alternate_artist: Some(artist_name(left)),
                    needs_confirmation: true,
                };
            }
        }
    }
    Identity {
        title,
        artist,
        alternate_artist: None,
        needs_confirmation: is_sensitive || quoted,
    }
}
pub fn same_title(identity: &Identity, candidate: &Candidate) -> bool {
    let target = normalized(&strip_decorations(&candidate.track_name));
    normalized(&identity.title) == target || title_variants(identity).contains(&target)
}
pub fn title_variants(identity: &Identity) -> Vec<String> {
    if !identity.needs_confirmation {
        return vec![];
    }
    let original = normalized(&identity.title);
    let mut clean = original.as_str();
    for suffix in [" intro cover", " cover espanol", " cover", " intro"] {
        if let Some(prefix) = clean.strip_suffix(suffix) {
            clean = prefix.trim();
        }
    }
    let mut variants = Vec::new();
    if !clean.is_empty() && clean != original {
        variants.push(clean.to_owned());
    }
    if let Some((short, _)) = clean.split_once(" y los ")
        && short.split_whitespace().count() >= 2
    {
        variants.push(short.to_owned());
    }
    variants
}
pub fn same_performer(identity: &Identity, candidate: &Candidate) -> bool {
    !identity.artist.is_empty()
        && normalized(&identity.artist) == normalized(&artist_name(&candidate.artist_name))
}
pub fn automatic_match(track: &Track, candidates: &[Candidate]) -> Option<usize> {
    let duration = track.duration_ms? as f64 / 1000.0;
    let derived = identity(track);
    let matches: Vec<_> = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| {
            let raw_exact = normalized(&track.title) == normalized(&candidate.track_name)
                && normalized(&track.artist) == normalized(&candidate.artist_name);
            same_title(&derived, candidate)
                && same_performer(&derived, candidate)
                && (!derived.needs_confirmation || raw_exact)
                && (raw_exact || !sensitive(&candidate.track_name))
                && (duration - candidate.duration).abs() <= 2.0
                && (track.album.is_empty()
                    || normalized(&track.album) == normalized(&candidate.album_name))
        })
        .map(|(index, _)| index)
        .collect();
    if matches.len() == 1 {
        Some(matches[0])
    } else {
        None
    }
}
pub fn rank(track: &Track, candidates: &mut Vec<Candidate>) {
    let derived = identity(track);
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.id));
    // Broad queries must not turn "He-Man" into unrelated "He Wasn't Man Enough".
    candidates.retain(|candidate| same_title(&derived, candidate));
    let score = |candidate: &Candidate| {
        let mut score = if same_title(&derived, candidate) {
            100
        } else {
            0
        };
        if same_performer(&derived, candidate) {
            score += 100;
        }
        if let Some(duration) = track.duration_ms {
            score -= ((duration as f64 / 1000.0 - candidate.duration)
                .abs()
                .min(60.0)) as i32;
        }
        if candidate
            .synced_lyrics
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
        {
            score += 5;
        }
        score
    };
    candidates.sort_by_key(|candidate| std::cmp::Reverse(score(candidate)));
    for candidate in candidates.iter_mut() {
        let mut notes = Vec::new();
        if !same_performer(&derived, candidate) {
            notes.push(
                "Coincidencia solo de título: otro artista; puede ser otra canción.".to_owned(),
            );
        }
        if derived.needs_confirmation {
            notes.push("El video indica una versión o idioma: confirma la letra.".to_owned());
        }
        if let Some(duration) = track.duration_ms {
            let delta = (duration as f64 / 1000.0 - candidate.duration).abs();
            if delta > 2.0 {
                notes.push(format!(
                    "Duración distinta por {delta:.0} s; la sincronización puede necesitar ajuste."
                ));
            }
        }
        candidate.match_note = (!notes.is_empty()).then(|| notes.join(" "));
    }
    candidates.truncate(20);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn track(title: &str, artist: &str) -> Track {
        Track {
            title: title.into(),
            artist: artist.into(),
            duration_ms: Some(196_000),
            ..Default::default()
        }
    }
    fn candidate(title: &str, artist: &str) -> Candidate {
        serde_json::from_value(serde_json::json!({"id":1,"trackName":title,"artistName":artist,"duration":196,"syncedLyrics":"[00:00.00]Test"})).unwrap()
    }
    #[test]
    fn youtube_cover_keeps_actual_performer_and_requires_confirmation() {
        let track = track(
            "Black Clover Opening 5 『Miyuna - Gamushara』Cover [ES]",
            "The Covers Duo",
        );
        let derived = identity(&track);
        assert_eq!(derived.title, "Gamushara");
        assert_eq!(derived.artist, "The Covers Duo");
        assert_eq!(derived.alternate_artist.as_deref(), Some("Miyuna"));
        assert_eq!(
            automatic_match(&track, &[candidate("Gamushara", "The Covers Duo")]),
            None
        );
        let mut results = vec![
            candidate("Gamushara", "Miyuna"),
            candidate("Gamushara", "The Covers Duo"),
        ];
        results[0].id = 2;
        rank(&track, &mut results);
        assert_eq!(results[0].artist_name, "The Covers Duo");
        assert!(results[0].match_note.as_ref().unwrap().contains("idioma"));
    }
    #[test]
    fn video_decoration_topic_and_vevo_are_search_noise() {
        for (title, artist, expected) in [
            (
                "FLOW - GO!!! (Official Music Video)",
                "FLOWofficialVEVO",
                "GO!!!",
            ),
            ("GO!!! - FLOW", "FLOW - Topic", "GO!!!"),
            ("GO!!! [HD]", "FLOW", "GO!!!"),
        ] {
            let track = track(title, artist);
            let derived = identity(&track);
            assert_eq!(derived.title, expected);
            assert_eq!(derived.artist, "FLOW");
            assert_eq!(
                automatic_match(&track, &[candidate("GO!!!", "FLOW")]),
                Some(0)
            );
        }
    }
    #[test]
    fn versions_missing_artists_and_duration_mismatch_never_auto_select() {
        for (title, artist) in [
            ("Gamushara (Live)", "Miyuna"),
            ("Gamushara [ES]", "Miyuna"),
            ("Gamushara - Remix", "Miyuna"),
            ("Gamushara", ""),
        ] {
            assert_eq!(
                automatic_match(&track(title, artist), &[candidate("Gamushara", "Miyuna")]),
                None
            );
        }
        let mut t = track("Gamushara", "The Covers Duo");
        t.duration_ms = Some(216_121);
        let mut results = vec![candidate("Gamushara", "The Covers Duo")];
        rank(&t, &mut results);
        assert_eq!(automatic_match(&t, &results), None);
        assert!(results[0].match_note.as_ref().unwrap().contains("20 s"));
        assert_eq!(
            identity(&track("Song - Part II", "Author")).title,
            "Song - Part II"
        );
    }
    #[test]
    fn unicode_and_accents_match_without_changing_display_text() {
        assert_eq!(normalized("Canción — Ａ"), normalized("Cancion - A"));
    }
    #[test]
    fn he_man_video_uses_short_title_but_filters_unrelated_search_hits() {
        let source = track(
            "HE-MAN Y Los Amos Del Universo Intro Cover!",
            "The Covers Duo",
        );
        let derived = identity(&source);
        assert!(title_variants(&derived).contains(&"he man".into()));
        let mut results = vec![
            candidate("He-Man", "Enorion"),
            candidate("He Wasn't Man Enough", "Toni Braxton"),
        ];
        results[1].id = 2;
        rank(&source, &mut results);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].track_name, "He-Man");
        assert!(
            results[0]
                .match_note
                .as_ref()
                .unwrap()
                .contains("otra canción")
        );
        assert_eq!(automatic_match(&source, &results), None);
    }
}
