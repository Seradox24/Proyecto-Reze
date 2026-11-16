use serde::{Deserialize, Deserializer, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ZOrder {
    Top,
    #[default]
    Normal,
    Bottom,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    #[default]
    Compact,
    LyricsOnly,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KaraokeMode {
    Line,
    Word,
}

/// Visual theme of the interface. An identifier this build does not know (a newer or
/// hand-edited file, or the `classic` id written by 0.2.0) falls back to the default
/// theme instead of discarding every setting.
#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemeId {
    #[default]
    ClassicCat,
    WindowCat,
    CatNotebook,
    CatConstellation,
    MochiCat,
}

impl ThemeId {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "classic_cat" => Some(Self::ClassicCat),
            "window_cat" => Some(Self::WindowCat),
            "cat_notebook" => Some(Self::CatNotebook),
            "cat_constellation" => Some(Self::CatConstellation),
            "mochi_cat" => Some(Self::MochiCat),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClassicCat => "classic_cat",
            Self::WindowCat => "window_cat",
            Self::CatNotebook => "cat_notebook",
            Self::CatConstellation => "cat_constellation",
            Self::MochiCat => "mochi_cat",
        }
    }
}

impl<'de> Deserialize<'de> for ThemeId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Option::<serde_json::Value>::deserialize(deserializer)?;
        Ok(match value.as_ref().and_then(|v| v.as_str()) {
            Some(raw) => ThemeId::parse(raw).unwrap_or_default(),
            _ => ThemeId::default(),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LyricsBackdrop {
    #[default]
    Dark,
    Light,
}

impl LyricsBackdrop {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

impl<'de> Deserialize<'de> for LyricsBackdrop {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Option::<serde_json::Value>::deserialize(deserializer)?;
        Ok(match value.as_ref().and_then(|v| v.as_str()) {
            Some(raw) => LyricsBackdrop::parse(raw).unwrap_or_default(),
            _ => LyricsBackdrop::default(),
        })
    }
}

fn default_cat_animation() -> bool {
    true
}

fn deserialize_cat_animation<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::Bool(flag)) => flag,
        Some(serde_json::Value::Null) | None => true,
        _ => true,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub karaoke_mode: KaraokeMode,
    pub schema_version: u32,
    pub network_allowed: bool,
    pub offset_ms: i64,
    pub font_size: u32,
    pub opacity: f64,
    pub alignment: Alignment,
    pub reduced_motion: bool,
    pub z_order: ZOrder,
    pub view_mode: ViewMode,
    pub theme_id: ThemeId,
    #[serde(
        default = "default_cat_animation",
        deserialize_with = "deserialize_cat_animation"
    )]
    pub cat_animation_enabled: bool,
    pub lyrics_backdrop: LyricsBackdrop,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: f64,
    pub height: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            karaoke_mode: KaraokeMode::Line,
            schema_version: 1,
            network_allowed: true,
            offset_ms: 0,
            font_size: 23,
            opacity: 1.0,
            alignment: Alignment::Center,
            reduced_motion: false,
            z_order: ZOrder::Normal,
            view_mode: ViewMode::Compact,
            theme_id: ThemeId::ClassicCat,
            cat_animation_enabled: true,
            lyrics_backdrop: LyricsBackdrop::Dark,
            x: None,
            y: None,
            width: 420.0,
            height: 260.0,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Versión de configuración no admitida.".into());
        }
        if !(-10_000..=10_000).contains(&self.offset_ms)
            || !(14..=64).contains(&self.font_size)
            || !self.opacity.is_finite()
            || !(0.2..=1.0).contains(&self.opacity)
            || !self.width.is_finite()
            || !self.height.is_finite()
            || !(300.0..=2000.0).contains(&self.width)
            || !(180.0..=2000.0).contains(&self.height)
        {
            return Err("Ajustes visuales fuera del rango permitido.".into());
        }
        Ok(())
    }
}
pub fn load(path: &Path) -> (Settings, Option<String>) {
    if !path.exists() {
        return (Settings::default(), None);
    }
    let loaded = std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|bytes| serde_json::from_slice::<Settings>(&bytes).map_err(|e| e.to_string()))
        .and_then(|s| {
            s.validate()?;
            Ok(s)
        });
    match loaded {
        Ok(settings) => (settings, None),
        Err(_) => {
            let backup = path.with_extension(format!(
                "invalid-{}.json",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ));
            let result = std::fs::copy(path, backup);
            (Settings::default(), Some(if result.is_ok() { "Se restauraron los ajustes; el archivo inválido tiene una copia de respaldo." } else { "Se restauraron los ajustes; no se pudo respaldar el archivo inválido." }.into()))
        }
    }
}
/// Import preferences from the former application identity only on first launch.
/// A current configuration always takes priority; the original remains intact.
pub fn load_with_legacy(path: &Path, legacy: &Path) -> (Settings, Option<String>) {
    if path.exists() || !legacy.exists() {
        return load(path);
    }
    let (settings, warning) = load(legacy);
    if warning.is_some() {
        return (settings, warning);
    }
    let warning = save(path, &settings).err().map(|_| {
        "Se cargaron los ajustes anteriores, pero no se pudo guardar su migración. Se reintentará al iniciar.".into()
    });
    (settings, warning)
}

pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    use std::io::Write;
    settings.validate()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    // Windows rename replaces an existing file through MoveFileExW in std.
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_preserves_legacy_and_never_overwrites_current_settings() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("previous/settings.json");
        let current = dir.path().join("current/settings.json");
        let previous = Settings {
            offset_ms: 500,
            karaoke_mode: KaraokeMode::Word,
            network_allowed: false,
            ..Default::default()
        };
        save(&legacy, &previous).unwrap();
        let original = std::fs::read(&legacy).unwrap();
        assert_eq!(
            load_with_legacy(&current, &legacy),
            (previous.clone(), None)
        );
        assert_eq!(load(&current).0, previous);
        assert_eq!(std::fs::read(&legacy).unwrap(), original);
        let updated = Settings {
            offset_ms: -300,
            ..previous
        };
        save(&current, &updated).unwrap();
        assert_eq!(load_with_legacy(&current, &legacy), (updated, None));
    }
    #[test]
    fn invalid_legacy_is_backed_up_without_creating_migrated_settings() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("previous.json");
        let current = dir.path().join("current/settings.json");
        std::fs::write(&legacy, "invalid json").unwrap();
        assert!(load_with_legacy(&current, &legacy).1.is_some());
        assert!(!current.exists());
        assert_eq!(std::fs::read_to_string(&legacy).unwrap(), "invalid json");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }
    #[test]
    fn invalid_config_is_backed_up_and_roundtrip_preserves_preferences() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "not json").unwrap();
        assert!(load(&path).1.is_some());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
        let settings = Settings {
            offset_ms: 500,
            ..Default::default()
        };
        save(&path, &settings).unwrap();
        save(&path, &settings).unwrap();
        assert_eq!(load(&path).0, settings);
    }
    #[test]
    fn theme_defaults_to_classic_and_unknown_ids_keep_the_other_settings() {
        let old: Settings = serde_json::from_str(r#"{"offset_ms":500}"#).unwrap();
        assert_eq!(old.theme_id, ThemeId::ClassicCat);
        // 0.2.0 wrote `classic` for the default look.
        let unknown: Settings =
            serde_json::from_str(r#"{"offset_ms":500,"theme_id":"classic"}"#).unwrap();
        assert_eq!(unknown.theme_id, ThemeId::ClassicCat);
        assert_eq!(unknown.offset_ms, 500);
        assert!(unknown.validate().is_ok());
        let cat = Settings {
            theme_id: ThemeId::MochiCat,
            ..Default::default()
        };
        let json = serde_json::to_string(&cat).unwrap();
        assert!(json.contains(r#""theme_id":"mochi_cat""#));
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), cat);
    }
    #[test]
    fn old_contact_is_ignored_and_never_saved() {
        let settings: Settings = serde_json::from_str(
            r#"{"network_allowed":true,"client_contact":"old@example.org","offset_ms":500}"#,
        )
        .unwrap();
        assert!(settings.validate().is_ok());
        assert_eq!(settings.offset_ms, 500);
        assert!(
            !serde_json::to_string(&settings)
                .unwrap()
                .contains("client_contact")
        );
        assert!(Settings::default().network_allowed);
        assert_eq!(settings.karaoke_mode, KaraokeMode::Line);
        let mut settings = settings;
        settings.karaoke_mode = KaraokeMode::Word;
        let restored: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.karaoke_mode, KaraokeMode::Word);
    }

    #[test]
    fn missing_theme_fields_receive_defaults_without_losing_others() {
        let settings: Settings = serde_json::from_str(
            r#"{"network_allowed":false,"offset_ms":250,"font_size":28,"karaoke_mode":"word"}"#,
        )
        .unwrap();
        assert_eq!(settings.theme_id, ThemeId::ClassicCat);
        assert!(settings.cat_animation_enabled);
        assert_eq!(settings.lyrics_backdrop, LyricsBackdrop::Dark);
        assert!(!settings.network_allowed);
        assert_eq!(settings.offset_ms, 250);
        assert_eq!(settings.font_size, 28);
        assert_eq!(settings.karaoke_mode, KaraokeMode::Word);
    }

    #[test]
    fn unknown_theme_or_backdrop_recovers_only_that_preference() {
        let settings: Settings = serde_json::from_str(
            r#"{
                "offset_ms": 700,
                "theme_id": "neon_dog",
                "lyrics_backdrop": "neon",
                "cat_animation_enabled": "maybe",
                "network_allowed": false,
                "view_mode": "lyrics_only"
            }"#,
        )
        .unwrap();
        assert_eq!(settings.theme_id, ThemeId::ClassicCat);
        assert_eq!(settings.lyrics_backdrop, LyricsBackdrop::Dark);
        assert!(settings.cat_animation_enabled);
        assert_eq!(settings.offset_ms, 700);
        assert!(!settings.network_allowed);
        assert_eq!(settings.view_mode, ViewMode::LyricsOnly);
    }

    #[test]
    fn theme_preferences_roundtrip() {
        let settings = Settings {
            theme_id: ThemeId::CatConstellation,
            cat_animation_enabled: false,
            lyrics_backdrop: LyricsBackdrop::Light,
            offset_ms: 100,
            ..Default::default()
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        save(&path, &settings).unwrap();
        assert_eq!(load(&path).0, settings);
        let encoded = serde_json::to_string(&settings).unwrap();
        assert!(encoded.contains("\"theme_id\":\"cat_constellation\""));
        assert!(encoded.contains("\"lyrics_backdrop\":\"light\""));
        assert!(encoded.contains("\"cat_animation_enabled\":false"));
    }
}
