#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod media;
mod window;

use engine::{Shared, SharedExt, emit, search};
use proyecto_reze_core::{
    lrc,
    model::{LyricsStatus, TranslationStatus},
    settings::Settings,
};
use tauri::{Manager, State};

#[tauri::command]
fn get_state(state: State<'_, Shared>) -> proyecto_reze_core::model::Snapshot {
    // The worker only refreshes the position when something visible changes, so bring it
    // up to date here: a caller always gets the position of this instant.
    let mut guard = state.guard();
    guard.update_position(std::time::Instant::now());
    guard.snapshot.clone()
}

/// The artwork travels apart from the snapshot: the interface asks for it only when
/// `artwork_revision` changes, instead of receiving the image with every update.
#[tauri::command]
fn get_artwork(state: State<'_, Shared>, revision: u64) -> Option<String> {
    let guard = state.guard();
    (guard.snapshot.artwork_revision == revision)
        .then(|| guard.artwork.as_deref().map(str::to_owned))
        .flatten()
}

#[tauri::command]
fn select_session(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    id: Option<String>,
) -> Result<(), String> {
    let mut guard = state.guard();
    if id
        .as_ref()
        .is_some_and(|id| !guard.snapshot.sessions.iter().any(|s| &s.id == id))
    {
        return Err("La sesión ya no está disponible.".into());
    }
    guard.reset_track();
    guard.snapshot.selected_session = id;
    let _ = guard.wake.try_send(());
    emit(&app, &mut guard);
    Ok(())
}

#[tauri::command]
async fn import_lrc(app: tauri::AppHandle, state: State<'_, Shared>) -> Result<(), String> {
    let shared = state.inner().clone();
    let generation = {
        let guard = shared.guard();
        if guard.snapshot.track.is_none() {
            return Err("Selecciona primero una sesión con una pista.".into());
        }
        guard.snapshot.generation
    };
    let parent = app
        .get_webview_window("main")
        .ok_or("No hay ventana para importar el LRC.")?;
    let file = rfd::AsyncFileDialog::new()
        .set_parent(&parent)
        .set_title("Importar letra LRC")
        .add_filter("Letra LRC o Lyricsfile", &["lrc", "yaml", "yml"])
        .pick_file()
        .await;
    let Some(file) = file else {
        return Ok(());
    };
    let path = file.path().to_owned();
    load_local_lrc(app, shared, generation, path).await
}

#[tauri::command]
async fn import_lrc_path(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    path: std::path::PathBuf,
) -> Result<(), String> {
    if !path.is_absolute()
        || !path.extension().is_some_and(|ext| {
            ["lrc", "yaml", "yml"]
                .iter()
                .any(|allowed| ext.eq_ignore_ascii_case(allowed))
        })
    {
        return Err("Indica la ruta completa de un archivo .lrc o Lyricsfile YAML local.".into());
    }
    let shared = state.inner().clone();
    let generation = {
        let guard = shared.guard();
        if guard.snapshot.track.is_none() {
            return Err("Selecciona primero una sesión con una pista.".into());
        }
        guard.snapshot.generation
    };
    load_local_lrc(app, shared, generation, path).await
}

async fn load_local_lrc(
    app: tauri::AppHandle,
    shared: Shared,
    generation: u64,
    path: std::path::PathBuf,
) -> Result<(), String> {
    let document = tauri::async_runtime::spawn_blocking(move || {
        use std::io::Read;
        let yaml = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"));
        let file = std::fs::File::open(path).map_err(|_| "No se pudo abrir el LRC.")?;
        let mut bytes = Vec::new();
        file.take((lrc::MAX_LRC_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "No se pudo leer el LRC.")?;
        if yaml {
            let text = std::str::from_utf8(&bytes).map_err(|_| "Guarda Lyricsfile como UTF-8.")?;
            let mut document = proyecto_reze_core::lyricsfile::parse(text)?;
            document.source = "Archivo local".into();
            Ok(document)
        } else {
            lrc::parse(&bytes, "Archivo local")
        }
    })
    .await
    .map_err(|e| e.to_string())??;
    let mut guard = shared.guard();
    if generation != guard.snapshot.generation {
        return Err("La pista cambió durante la importación. Vuelve a cargar el LRC.".into());
    }
    guard.cancel_search();
    guard.local_override = true;
    guard.install_lyrics(document, None, true);
    guard.set_candidates(vec![]);
    emit(&app, &mut guard);
    Ok(())
}

#[tauri::command]
fn search_lyrics(app: tauri::AppHandle, state: State<'_, Shared>) -> Result<(), String> {
    search(&app, state.inner().clone(), true)
}

#[tauri::command]
fn choose_candidate(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    id: i64,
    generation: u64,
) -> Result<(), String> {
    let mut guard = state.guard();
    if guard.snapshot.generation != generation || guard.local_override {
        return Err("El resultado pertenece a otra pista o hay un LRC local activo.".into());
    }
    // Search responses already contain the lyrics. Reuse them without another request.
    let document = guard
        .candidates
        .iter()
        .find(|c| c.id == id)
        .ok_or("El resultado ya no está disponible.")?
        .document()?;
    guard.cancel_search();
    guard.install_lyrics(document, Some(id), true);
    guard.snapshot.selection_notice =
        Some("Versión confirmada. Puedes elegir otra si la letra no corresponde.".into());
    emit(&app, &mut guard);
    Ok(())
}

#[tauri::command]
fn update_settings(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    mut settings: Settings,
) -> Result<(), String> {
    settings.validate()?;
    let old = state.guard().snapshot.settings.clone();
    // Geometry is owned by the native window event handler, not stale UI forms.
    settings.x = old.x;
    settings.y = old.y;
    settings.width = old.width;
    settings.height = old.height;
    let network_changed = settings.network_allowed != old.network_allowed;
    let provider = if settings.network_allowed {
        Some(proyecto_reze_core::lyrics::Lrclib::new()?)
    } else {
        None
    };
    window::apply(&app, &settings)?;
    let mut guard = state.guard();
    settings.x = guard.snapshot.settings.x;
    settings.y = guard.snapshot.settings.y;
    settings.width = guard.snapshot.settings.width;
    settings.height = guard.snapshot.settings.height;
    if let Err(error) = proyecto_reze_core::settings::save(&guard.settings_path, &settings) {
        drop(guard);
        let _ = window::apply(&app, &old);
        return Err(error);
    }
    guard.snapshot.settings = settings;
    guard.geometry_saved();
    if network_changed {
        guard.cancel_search();
        guard.cancel_translation();
        guard.provider = provider;
        if guard.snapshot.lyrics_status == LyricsStatus::Loading {
            guard.snapshot.lyrics_status = LyricsStatus::Missing;
        }
    }
    emit(&app, &mut guard);
    drop(guard);
    if network_changed {
        let _ = search(&app, state.inner().clone(), false);
    }
    Ok(())
}

#[tauri::command]
fn correct_search(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    title: String,
    artist: String,
    generation: u64,
    reset: bool,
) -> Result<(), String> {
    let mut guard = state.guard();
    if !guard.snapshot.settings.network_allowed {
        return Err("Activa la búsqueda en Internet antes de corregir la consulta.".into());
    }
    if guard.snapshot.generation != generation || guard.snapshot.track.is_none() {
        return Err("La pista cambió. Vuelve a abrir la corrección de búsqueda.".into());
    }
    if !reset
        && (title.trim().is_empty() || title.chars().count() > 512 || artist.chars().count() > 512)
    {
        return Err("Escribe un título y usa como máximo 512 caracteres por campo.".into());
    }
    guard.cancel_search();
    guard.cancel_translation();
    guard.local_override = false;
    guard.snapshot.search_override = if reset {
        None
    } else {
        Some(proyecto_reze_core::model::Track {
            title: title.trim().into(),
            artist: artist.trim().into(),
            album: String::new(),
            duration_ms: guard.snapshot.track.as_ref().and_then(|t| t.duration_ms),
        })
    };
    guard.snapshot.lyrics = None;
    guard.set_candidates(vec![]);
    guard.snapshot.active_line = None;
    guard.snapshot.selection_notice = None;
    guard.snapshot.selected_candidate_id = None;
    guard.snapshot.lyrics_revision += 1;
    drop(guard);
    search(&app, state.inner().clone(), true)
}

#[tauri::command]
fn translate_lyrics(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    source: Option<String>,
    generation: u64,
    lyrics_revision: u64,
) -> Result<(), String> {
    let shared = state.inner().clone();
    let mut guard = shared.guard();
    if !guard.snapshot.settings.network_allowed {
        return Err("Activa el acceso a Internet en ajustes para traducir.".into());
    }
    if guard.snapshot.generation != generation || guard.snapshot.lyrics_revision != lyrics_revision
    {
        return Err("La letra cambió; vuelve a solicitar la traducción.".into());
    }
    let original = guard
        .snapshot
        .lyrics
        .clone()
        .ok_or("Primero carga una letra.")?;
    let source = source
        .filter(|s| !s.is_empty())
        .or_else(|| guard.snapshot.language.clone())
        .ok_or("Elige el idioma original de la letra.")?;
    if source == "es" {
        return Err("Esta letra ya está en español.".into());
    }
    let translator = proyecto_reze_core::translation::Translator::new()?;
    guard.cancel_translation();
    let request = guard.translation_request;
    guard.snapshot.translation_status = TranslationStatus::Loading;
    guard.snapshot.error = None;
    emit(&app, &mut guard);
    let task_shared = shared.clone();
    guard.translation_task = Some(tauri::async_runtime::spawn(async move {
        let result = translator.translate(&original, &source).await;
        let mut guard = task_shared.guard();
        if guard.translation_request != request
            || guard.snapshot.generation != generation
            || guard.snapshot.lyrics_revision != lyrics_revision
            || !guard.snapshot.settings.network_allowed
        {
            return;
        }
        match result {
            Ok(document) => {
                guard.snapshot.translation = Some(document);
                guard.snapshot.translation_status = TranslationStatus::Ready;
            }
            Err(error) => {
                guard.snapshot.translation_status = TranslationStatus::Error;
                guard.snapshot.error = Some(error);
            }
        }
        emit(&app, &mut guard);
    }));
    Ok(())
}

#[tauri::command]
fn cancel_translation(app: tauri::AppHandle, state: State<'_, Shared>) {
    let mut guard = state.guard();
    guard.cancel_translation();
    emit(&app, &mut guard);
}

#[tauri::command]
fn reset_window(app: tauri::AppHandle, state: State<'_, Shared>) -> Result<(), String> {
    let settings = Settings {
        x: None,
        y: None,
        ..state.guard().snapshot.settings.clone()
    };
    window::restore(&app, &settings)?;
    let mut guard = state.guard();
    guard.snapshot.settings.x = None;
    guard.snapshot.settings.y = None;
    proyecto_reze_core::settings::save(&guard.settings_path, &guard.snapshot.settings)?;
    guard.geometry_saved();
    Ok(())
}
#[tauri::command]
fn hide_window(app: tauri::AppHandle, state: State<'_, Shared>) -> Result<(), String> {
    if !state.guard().tray_available {
        return Err("La bandeja no está disponible; usa Cerrar para salir.".into());
    }
    app.get_webview_window("main")
        .ok_or("Falta la ventana principal.")?
        .hide()
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            window::show(app);
        }))
        .invoke_handler(tauri::generate_handler![
            get_state,
            get_artwork,
            select_session,
            import_lrc,
            import_lrc_path,
            search_lyrics,
            correct_search,
            translate_lyrics,
            cancel_translation,
            choose_candidate,
            update_settings,
            reset_window,
            hide_window,
            quit
        ])
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            let legacy = app
                .path()
                .config_dir()?
                .join("app.danae.widget/settings.json");
            let (settings, warning) =
                proyecto_reze_core::settings::load_with_legacy(&path, &legacy);
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            let state = engine::create(settings.clone(), path, warning, tx);
            app.manage(state.clone());
            window::restore(app.handle(), &settings).map_err(std::io::Error::other)?;
            window::apply(app.handle(), &settings).map_err(std::io::Error::other)?;
            let tray = window::tray(app.handle());
            state.guard().tray_available = tray.is_ok();
            if let Err(error) = tray {
                tracing::warn!(%error, "Bandeja no disponible");
            }
            engine::start(app.handle().clone(), state, rx);
            window::show(app.handle());
            Ok(())
        })
        .on_window_event(window::event)
        .build(tauri::generate_context!())
        .expect("No se pudo iniciar Proyecto Reze")
        .run(|app, event| {
            // A move made just before quitting is still waiting for its debounced save.
            if let tauri::RunEvent::Exit = event
                && let Some(state) = app.try_state::<Shared>()
            {
                state.guard().flush_geometry();
            }
        });
}
