//! Conservative Lyricsfile 1.0 reader. Missing word end times are never invented.
use crate::model::{LyricFormat, LyricLine, LyricWord, LyricsDocument};
use serde::Deserialize;
const MAX_BYTES: usize = 512 * 1024;
const MAX_TIME: i64 = 24 * 60 * 60 * 1000;

#[derive(Deserialize)]
struct File {
    version: String,
    metadata: Metadata,
    #[serde(default)]
    lines: Vec<Line>,
    #[serde(default)]
    plain: String,
}
#[derive(Deserialize)]
struct Metadata {
    title: String,
    artist: String,
    #[serde(default)]
    offset_ms: i64,
    #[serde(default)]
    instrumental: bool,
}
#[derive(Deserialize)]
struct Line {
    text: String,
    start_ms: i64,
    #[serde(default)]
    end_ms: Option<i64>,
    #[serde(default)]
    words: Vec<LyricWord>,
}

pub fn parse(raw: &str) -> Result<LyricsDocument, String> {
    if raw.len() > MAX_BYTES {
        return Err("Lyricsfile supera el límite de tamaño.".into());
    }
    let file: File =
        serde_yaml_ng::from_str(raw).map_err(|_| "Lyricsfile no tiene una estructura válida.")?;
    if file.version != "1.0" {
        return Err("Versión Lyricsfile no admitida.".into());
    }
    if file.metadata.offset_ms != 0 {
        return Err("El desfase interno de Lyricsfile no tiene una convención estable; se usará el formato alternativo.".into());
    }
    // These fields are required by the format but never override Windows metadata.
    let _metadata = (&file.metadata.title, &file.metadata.artist);
    if file.lines.len() > 5000 || file.lines.iter().map(|l| l.words.len()).sum::<usize>() > 40000 {
        return Err("Lyricsfile contiene demasiados segmentos.".into());
    }
    if file.metadata.instrumental && (!file.lines.is_empty() || !file.plain.trim().is_empty()) {
        return Err("Lyricsfile instrumental contiene texto contradictorio.".into());
    }
    let mut warnings = Vec::new();
    let mut lines = Vec::with_capacity(file.lines.len());
    for mut line in file.lines {
        if !(0..=MAX_TIME).contains(&line.start_ms)
            || line
                .end_ms
                .is_some_and(|end| end < line.start_ms || end > MAX_TIME)
        {
            return Err("Lyricsfile contiene tiempos de línea inválidos.".into());
        }
        let joined: String = line.words.iter().map(|w| w.text.as_str()).collect();
        let valid = !line.text.trim().is_empty()
            && joined == line.text
            && line.words.len() <= 1000
            && line.words.iter().all(|word| {
                word.start_ms >= line.start_ms
                    && word.start_ms <= MAX_TIME
                    && !word.text.is_empty()
                    && line.end_ms.is_none_or(|end| word.start_ms <= end)
                    && word.end_ms.is_none_or(|end| {
                        end >= word.start_ms
                            && end <= MAX_TIME
                            && line.end_ms.is_none_or(|line_end| end <= line_end)
                    })
            })
            && line
                .words
                .windows(2)
                .all(|pair| pair[0].start_ms <= pair[1].start_ms);
        if !line.words.is_empty() && !valid {
            line.words.clear();
            warnings.push("Una línea tiene segmentos incompatibles; se mostrará por línea.".into());
        }
        lines.push(LyricLine {
            at_ms: line.start_ms,
            end_ms: line.end_ms,
            text: line.text,
            words: line.words,
        });
    }
    lines.sort_by_key(|line| line.at_ms);
    let format = if file.metadata.instrumental {
        LyricFormat::Instrumental
    } else if !lines.is_empty() {
        LyricFormat::LineSynced
    } else if !file.plain.trim().is_empty() {
        LyricFormat::Plain
    } else {
        return Err("Lyricsfile no contiene letras utilizables.".into());
    };
    Ok(LyricsDocument {
        source: "LRCLIB".into(),
        source_id: None,
        format,
        lines,
        plain_text: file.plain,
        warnings,
        rights_metadata: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const FILE: &str = include_str!("../../../fixtures/karaoke-demo.lyricsfile.yaml");
    #[test]
    fn reads_absolute_word_times_spaces_and_missing_end_without_guessing() {
        let doc = parse(FILE).unwrap();
        assert_eq!(doc.lines[0].text, "Hola mundo");
        assert_eq!(doc.lines[0].words[0].text, "Hola ");
        assert_eq!(doc.lines[0].words[1].start_ms, 2000);
        assert_eq!(doc.lines[1].words[1].end_ms, None);
    }
    #[test]
    fn invalid_words_fall_back_per_line_and_keep_line_text() {
        for invalid in [
            FILE.replace("start_ms: 2000", "start_ms: -1"),
            FILE.replace("text: 'Hola '", "text: 'Otra '"),
            FILE.replace("end_ms: 3000", "end_ms: 1500"),
        ] {
            let doc = parse(&invalid).unwrap();
            assert!(doc.lines[0].words.is_empty());
            assert_eq!(doc.lines[0].text, "Hola mundo");
            assert!(!doc.warnings.is_empty());
        }
    }
    #[test]
    fn rejects_unknown_version_duplicate_fields_oversized_and_ambiguous_offset() {
        assert!(parse(&FILE.replace("'1.0'", "'2.0'")).is_err());
        assert!(parse(&format!("version: '1.0'\n{FILE}")).is_err());
        assert!(parse(&"x".repeat(MAX_BYTES + 1)).is_err());
        assert!(parse(&FILE.replace("  artist:", "  offset_ms: 300\n  artist:")).is_err());
    }
}
