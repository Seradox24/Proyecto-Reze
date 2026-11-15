use crate::model::{LyricFormat, LyricLine, LyricsDocument};

pub const MAX_LRC_BYTES: usize = 2 * 1024 * 1024;

fn timestamp(value: &str) -> Option<i64> {
    let (minutes, seconds) = value.split_once(':')?;
    if minutes.is_empty() || !minutes.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let (whole, frac) = seconds.split_once('.').unwrap_or((seconds, ""));
    if whole.len() != 2
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || frac.len() > 3
        || !frac.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let sec: i64 = whole.parse().ok()?;
    if sec >= 60 {
        return None;
    }
    let ms = if frac.is_empty() {
        0
    } else {
        frac.parse::<i64>().ok()? * 10_i64.pow(3 - frac.len() as u32)
    };
    minutes
        .parse::<i64>()
        .ok()?
        .checked_mul(60_000)?
        .checked_add(sec * 1000 + ms)
}

pub fn parse(bytes: &[u8], source: &str) -> Result<LyricsDocument, String> {
    if bytes.len() > MAX_LRC_BYTES {
        return Err("El LRC supera el límite de 2 MiB.".into());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "Guarda el LRC como UTF-8.")?
        .trim_start_matches('\u{feff}');
    let mut lines = Vec::new();
    let mut warnings = Vec::new();
    let mut offset: i64 = 0;
    for (index, raw) in text.lines().enumerate() {
        let mut rest = raw.trim();
        let mut times = Vec::new();
        let mut valid_metadata = false;
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else {
                break;
            };
            let tag = &stripped[..end];
            if let Some(value) = timestamp(tag) {
                times.push(value);
            } else if let Some(value) = tag.strip_prefix("offset:") {
                match value.trim().parse::<i64>() {
                    Ok(value) if value.abs_diff(0) <= 600_000 => offset = value,
                    _ => warnings.push(format!("Desfase inválido en línea {}.", index + 1)),
                }
                valid_metadata = true;
            } else if ["ar:", "ti:", "al:", "by:", "length:", "re:", "ve:"]
                .iter()
                .any(|prefix| tag.starts_with(prefix))
            {
                valid_metadata = true;
            } else {
                warnings.push(format!("Etiqueta inválida en línea {}.", index + 1));
            }
            rest = &stripped[end + 1..];
        }
        if times.is_empty() && !valid_metadata && !raw.trim().is_empty() {
            warnings.push(format!("Sin tiempo válido en línea {}.", index + 1));
        }
        for at_ms in times {
            lines.push(LyricLine {
                at_ms,
                text: rest.trim().to_owned(),
                end_ms: None,
                words: vec![],
            });
        }
    }
    if lines.is_empty() {
        return Err("El archivo no contiene líneas LRC temporizadas válidas.".into());
    }
    // Positive LRC offset advances the lyrics, consistent with the manual offset.
    for line in &mut lines {
        line.at_ms = line.at_ms.saturating_sub(offset);
    }
    lines.sort_by_key(|line| line.at_ms);
    Ok(LyricsDocument {
        source: source.into(),
        source_id: None,
        format: LyricFormat::LineSynced,
        lines,
        plain_text: String::new(),
        warnings,
        rights_metadata: None,
    })
}

pub fn active_line(lines: &[LyricLine], position_ms: i64) -> Option<usize> {
    lines
        .partition_point(|line| line.at_ms <= position_ms)
        .checked_sub(1)
}

/// Start of the first line after `position_ms`, used to sleep exactly until it becomes active.
pub fn next_line_at(lines: &[LyricLine], position_ms: i64) -> Option<i64> {
    lines
        .get(lines.partition_point(|line| line.at_ms <= position_ms))
        .map(|line| line.at_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn next_line_at_finds_the_following_start_and_ends_after_the_last_line() {
        let doc = parse(b"[00:01.00]a\n[00:01.00]b\n[00:03.00]c", "test").unwrap();
        assert_eq!(next_line_at(&doc.lines, -5), Some(1000));
        assert_eq!(next_line_at(&doc.lines, 1000), Some(3000));
        assert_eq!(next_line_at(&doc.lines, 3000), None);
    }
    #[test]
    fn parses_bom_repeated_tags_offsets_and_stable_duplicates() {
        let doc = parse("\u{feff}[ar:Propio]\n[offset:100]\n[00:02.5][00:01.00]Hola\n[00:01.000]Otra\n[00:90.00]Inválida".as_bytes(), "test").unwrap();
        assert_eq!(
            doc.lines.iter().map(|l| l.at_ms).collect::<Vec<_>>(),
            vec![900, 900, 2400]
        );
        assert_eq!(doc.lines[0].text, "Hola");
        assert_eq!(active_line(&doc.lines, 899), None);
        assert_eq!(active_line(&doc.lines, 900), Some(1));
        assert!(!doc.warnings.is_empty());
    }
    #[test]
    fn rejects_invalid_empty_and_oversize() {
        for text in ["", "texto", "[00:70.00]x", "[00:01.x]x"] {
            assert!(parse(text.as_bytes(), "x").is_err());
        }
        assert!(parse(&[0xff], "x").is_err());
        assert!(parse(&vec![0; MAX_LRC_BYTES + 1], "x").is_err());
    }
}
