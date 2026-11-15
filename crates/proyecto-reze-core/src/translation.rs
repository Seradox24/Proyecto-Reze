use crate::{lyrics::CLIENT_IDENTIFICATION, model::LyricsDocument};
use std::{collections::HashMap, time::Duration};

pub fn language(document: &LyricsDocument) -> Option<String> {
    use whatlang::Lang;
    let text = if document.lines.is_empty() {
        document.plain_text.clone()
    } else {
        document
            .lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let text: String = text.chars().take(16000).collect();
    let info = whatlang::detect(&text)?;
    if !info.is_reliable() {
        return None;
    }
    Some(
        match info.lang() {
            Lang::Eng => "en",
            Lang::Spa => "es",
            Lang::Jpn => "ja",
            Lang::Fra => "fr",
            Lang::Deu => "de",
            Lang::Ita => "it",
            Lang::Por => "pt",
            Lang::Kor => "ko",
            Lang::Rus => "ru",
            Lang::Cmn => "zh",
            _ => return None,
        }
        .into(),
    )
}

#[derive(Clone)]
pub struct Translator {
    client: reqwest::Client,
    base: String,
}
impl Translator {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent(CLIENT_IDENTIFICATION)
                .timeout(Duration::from_secs(12))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| "No se pudo preparar la traducción.")?,
            base: "https://api.mymemory.translated.net/get".into(),
        })
    }
    pub async fn translate(
        &self,
        original: &LyricsDocument,
        source: &str,
    ) -> Result<LyricsDocument, String> {
        if !["en", "ja", "fr", "de", "it", "pt", "ko", "ru", "zh"].contains(&source) {
            return Err(
                "Elige el idioma original; si ya está en español no necesita traducción.".into(),
            );
        }
        let segments: Vec<String> = if original.lines.is_empty() {
            original.plain_text.lines().map(str::to_owned).collect()
        } else {
            original.lines.iter().map(|l| l.text.clone()).collect()
        };
        let mut unique = Vec::new();
        for text in &segments {
            if !text.trim().is_empty() && !unique.contains(text) {
                unique.push(text.clone());
            }
        }
        if unique.iter().any(|s| s.len() > 500) {
            return Err("Una línea supera los 500 bytes admitidos por el traductor. La letra original se conserva.".into());
        }
        if unique.iter().map(|s| s.chars().count()).sum::<usize>() > 5000 || unique.len() > 100 {
            return Err("La letra supera el límite de esta traducción gratuita (5000 caracteres o 100 líneas distintas).".into());
        }
        let mut cache = HashMap::new();
        for (index, text) in unique.into_iter().enumerate() {
            if index > 0 {
                tokio::time::sleep(Duration::from_millis(400)).await;
            }
            let response = self
                .client
                .get(&self.base)
                .query(&[
                    ("q", text.as_str()),
                    ("langpair", &format!("{source}|es")),
                    ("mt", "1"),
                ])
                .send()
                .await
                .map_err(|_| "No se pudo contactar al traductor. La letra original se conserva.")?;
            if !response.status().is_success() {
                return Err("El servicio de traducción no está disponible o alcanzó su límite. Reintenta más tarde.".into());
            }
            if response
                .content_length()
                .is_some_and(|size| size > 128 * 1024)
            {
                return Err("Respuesta de traducción demasiado grande.".into());
            }
            let mut response = response;
            let mut body = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| "Traducción interrumpida.")?
            {
                if body.len() + chunk.len() > 128 * 1024 {
                    return Err("Respuesta de traducción demasiado grande.".into());
                }
                body.extend_from_slice(&chunk);
            }
            let value: serde_json::Value =
                serde_json::from_slice(&body).map_err(|_| "Respuesta de traducción inválida.")?;
            if value["responseStatus"].as_u64() != Some(200) {
                return Err("El traductor rechazó la consulta o agotó su cuota. No se modificó la letra original.".into());
            }
            let translated = value["responseData"]["translatedText"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("El traductor devolvió una línea vacía.")?;
            cache.insert(text, translated.to_owned());
        }
        let mut document = original.clone();
        document.source = format!("{} · traducción automática MyMemory", original.source);
        document.warnings.push("Traducción automática al español; puede contener errores. Tiempos conservados del original.".into());
        if document.lines.is_empty() {
            document.plain_text = segments
                .iter()
                .map(|s| cache.get(s).unwrap_or(s).as_str())
                .collect::<Vec<_>>()
                .join("\n");
        } else {
            for line in &mut document.lines {
                if let Some(translated) = cache.get(&line.text) {
                    line.text = translated.clone();
                }
                line.words.clear();
            }
        }
        Ok(document)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{path, query_param},
    };
    #[tokio::test]
    async fn translation_preserves_timestamps_and_deduplicates_repeated_lines() {
        let server = MockServer::start().await;
        Mock::given(path("/get")).and(query_param("langpair","en|es")).and(query_param("q","Hello world"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"responseStatus":200,"responseData":{"translatedText":"Hola mundo"}}))).expect(1).mount(&server).await;
        let mut translator = Translator::new().unwrap();
        translator.base = format!("{}/get", server.uri());
        let mut original = crate::lrc::parse(b"[00:01.00][00:03.00]Hello world", "Test").unwrap();
        original.lines[0].words.push(crate::model::LyricWord {
            text: "Hello world".into(),
            start_ms: 1000,
            end_ms: Some(2000),
        });
        let translated = translator.translate(&original, "en").await.unwrap();
        assert_eq!(translated.lines[0].at_ms, 1000);
        assert_eq!(translated.lines[1].at_ms, 3000);
        assert_eq!(translated.lines[0].text, "Hola mundo");
        assert_eq!(original.lines[0].text, "Hello world");
        assert!(translated.lines[0].words.is_empty());
        assert_eq!(original.lines[0].words.len(), 1);
    }
    #[tokio::test]
    async fn rejects_quota_error_and_oversized_lines_without_replacing_original() {
        let server = MockServer::start().await;
        Mock::given(path("/get"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"responseStatus":403})),
            )
            .mount(&server)
            .await;
        let mut translator = Translator::new().unwrap();
        translator.base = format!("{}/get", server.uri());
        let original = crate::lrc::parse(b"[00:00.00]Test", "Test").unwrap();
        assert!(translator.translate(&original, "en").await.is_err());
        assert!(translator.translate(&original, "es").await.is_err());
        let large =
            crate::lrc::parse(format!("[00:00.00]{}", "x".repeat(501)).as_bytes(), "Test").unwrap();
        assert!(translator.translate(&large, "en").await.is_err());
    }
}
