use proyecto_reze_core::settings::{Settings, ViewMode, ZOrder};
use tauri::{
    Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

pub fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
pub fn apply(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Falta la ventana principal.")?;
    window.set_always_on_top(false).map_err(|e| e.to_string())?;
    window
        .set_always_on_bottom(false)
        .map_err(|e| e.to_string())?;
    match settings.z_order {
        ZOrder::Top => window.set_always_on_top(true),
        ZOrder::Bottom => window.set_always_on_bottom(true),
        ZOrder::Normal => Ok(()),
    }
    .map_err(|e| e.to_string())?;
    window
        .set_shadow(settings.view_mode != ViewMode::LyricsOnly)
        .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn restore(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Falta ventana principal.")?;
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let target = monitors
        .iter()
        .find(|monitor| {
            let area = monitor.work_area();
            settings.x.zip(settings.y).is_some_and(|(x, y)| {
                x >= area.position.x
                    && y >= area.position.y
                    && x < area.position.x + area.size.width as i32
                    && y < area.position.y + area.size.height as i32
            })
        })
        .or_else(|| monitors.first());
    if let Some(monitor) = target {
        let area = monitor.work_area();
        let scale = monitor.scale_factor();
        let width = (settings.width * scale).min(area.size.width as f64) as u32;
        let height = (settings.height * scale).min(area.size.height as f64) as u32;
        window
            .set_size(tauri::PhysicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        let x = settings
            .x
            .unwrap_or(area.position.x + (area.size.width.saturating_sub(width) / 2) as i32)
            .clamp(
                area.position.x,
                area.position.x + area.size.width.saturating_sub(width) as i32,
            );
        let y = settings
            .y
            .unwrap_or(area.position.y + (area.size.height.saturating_sub(height) / 2) as i32)
            .clamp(
                area.position.y,
                area.position.y + area.size.height.saturating_sub(height) as i32,
            );
        window
            .set_position(tauri::PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Mostrar / ocultar", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Ajustes", true, None::<&str>)?;
    let reset = MenuItem::with_id(app, "reset", "Recuperar ventana", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &settings, &reset, &quit])?;
    let mut pixels = vec![0u8; 32 * 32 * 4];
    for y in 4..28 {
        for x in 4..28 {
            let index = (y * 32 + x) * 4;
            let mark = ((10..=12).contains(&x) && (8..=24).contains(&y))
                || ((12..=19).contains(&x) && ((8..=10).contains(&y) || (15..=17).contains(&y)))
                || ((19..=21).contains(&x) && (10..=15).contains(&y))
                || ((17..=24).contains(&y) && (x as i32 - y as i32 + 3).abs() <= 1);
            pixels[index..index + 4].copy_from_slice(if mark {
                &[193, 232, 179, 255]
            } else {
                &[27, 32, 27, 255]
            });
        }
    }
    TrayIconBuilder::with_id("proyecto-reze")
        .icon(tauri::image::Image::new_owned(pixels, 32, 32))
        .tooltip("Proyecto Reze — letras sincronizadas")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => {
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        show(app);
                    }
                }
            }
            "settings" => {
                show(app);
                let _ = app.emit("proyecto-reze:settings", ());
            }
            "reset" => {
                let state = app.state::<crate::engine::Shared>();
                let settings = state.lock().unwrap().snapshot.settings.clone();
                let _ = restore(
                    app,
                    &Settings {
                        x: None,
                        y: None,
                        ..settings
                    },
                );
                show(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
pub fn event(window: &tauri::Window, event: &tauri::WindowEvent) {
    let Some(state) = window.app_handle().try_state::<crate::engine::Shared>() else {
        return;
    };
    match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            if state.lock().unwrap().tray_available {
                api.prevent_close();
                let _ = window.hide();
            }
        }
        tauri::WindowEvent::Moved(_)
        | tauri::WindowEvent::Resized(_)
        | tauri::WindowEvent::ScaleFactorChanged { .. } => {
            if !window.is_visible().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
                return;
            }
            let (Ok(position), Ok(size), Ok(scale)) = (
                window.outer_position(),
                window.inner_size(),
                window.scale_factor(),
            ) else {
                return;
            };
            let mut state = state.lock().unwrap();
            state.snapshot.settings.x = Some(position.x);
            state.snapshot.settings.y = Some(position.y);
            state.snapshot.settings.width = size.width as f64 / scale;
            state.snapshot.settings.height = size.height as f64 / scale;
            if let Err(error) =
                proyecto_reze_core::settings::save(&state.settings_path, &state.snapshot.settings)
            {
                tracing::warn!(%error, "No se pudo guardar geometría");
            }
        }
        _ => {}
    }
}
