use crate::icon::{self, ICON_PX};
use crate::model::Light;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

pub const TRAY_ID: &str = "main";

/// When the popover was last hidden by losing focus. Clicking the tray icon while the
/// popover is open first blurs it; without this the same click would reopen it.
static LAST_BLUR_HIDE_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn icon_image(light: Light, progress: Option<f32>) -> Image<'static> {
    Image::new_owned(icon::render(light, progress), ICON_PX, ICON_PX)
}

pub fn set_icon(app: &AppHandle, light: Light, progress: Option<f32>) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(icon_image(light, progress)));
        let _ = tray.set_icon_as_template(false);
    }
}

/// Text beside the light (macOS only; ignored elsewhere) and the hover tooltip.
pub fn set_text(app: &AppHandle, title: Option<&str>, tooltip: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(Some(title.unwrap_or("")));
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

pub fn open_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn toggle_popover(app: &AppHandle) {
    let Some(w) = app.get_webview_window("popover") else {
        return;
    };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    if now_ms().saturating_sub(LAST_BLUR_HIDE_MS.load(Ordering::Relaxed)) < 300 {
        return;
    }
    let _ = w.move_window(Position::TrayCenter);
    let _ = w.show();
    let _ = w.set_focus();
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let refresh = MenuItem::with_id(app, "refresh", "Actualizar ahora", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Repos y workflows…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&refresh, &settings, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon_image(Light::Gray, None))
        .icon_as_template(false)
        .tooltip("Actions Lights")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "refresh" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let ctx = app.state::<crate::app_state::AppCtx>();
                    ctx.refresh_auth().await;
                    ctx.wake.notify_one();
                });
            }
            "settings" => open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_popover(tray.app_handle());
            }
        })
        .build(app)?;

    if let Some(popover) = app.get_webview_window("popover") {
        // Without FullScreenAuxiliary macOS keeps the popover off a fullscreen app's Space.
        #[cfg(target_os = "macos")]
        if let Ok(ptr) = popover.ns_window() {
            use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
            let ns_window = unsafe { &*(ptr as *const NSWindow) };
            ns_window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary,
            );
        }
        let w = popover.clone();
        popover.on_window_event(move |event| {
            if let WindowEvent::Focused(false) = event {
                LAST_BLUR_HIDE_MS.store(now_ms(), Ordering::Relaxed);
                let _ = w.hide();
            }
        });
    }
    if let Some(settings) = app.get_webview_window("settings") {
        let w = settings.clone();
        // Closing settings only hides it; the app lives in the tray.
        settings.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = w.hide();
            }
        });
    }
    Ok(())
}
