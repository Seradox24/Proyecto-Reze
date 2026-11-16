use serde::{Deserialize, Serialize};
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

/// Visual skin of the interface. An identifier this build does not know (a newer or
/// hand-edited file) falls back to the classic look instead of discarding every setting.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemeId {
    MochiCat,
    #[default]
    #[serde(other)]
    Classic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KaraokeMode {
    Line,
    Word,
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
    pub theme_id: ThemeId,
    pub reduced_motion: bool,
    pub z_order: ZOrder,
    pub view_mode: ViewMode,
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
            theme_id: ThemeId::Classic,
            reduced_motion: false,
            z_order: ZOrder::Normal,
            view_mode: ViewMode::Compact,
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
        assert_eq!(old.theme_id, ThemeId::Classic);
        let unknown: Settings =
            serde_json::from_str(r#"{"offset_ms":500,"theme_id":"from_the_future"}"#).unwrap();
        assert_eq!(unknown.theme_id, ThemeId::Classic);
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
}
