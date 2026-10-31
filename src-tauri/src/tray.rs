use crate::i18n::t;
use crate::icon::{self, ICON_PX};
use crate::model::Light;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

pub const TRAY_ID: &str = "main";

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(PopoverPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
    })
}

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

pub fn set_icon(app: &AppHandle, light: Light, progress: Option<f32>, idle: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let image = if idle {
            Image::new_owned(icon::render_idle(), ICON_PX, ICON_PX)
        } else {
            icon_image(light, progress)
        };
        let _ = tray.set_icon(Some(image));
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

/// While settings is open the app behaves like a regular one (Dock icon, Cmd+Tab), so
/// the window can be found again after switching apps; otherwise it lives in the tray.
fn set_regular_app(app: &AppHandle, regular: bool) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(if regular {
        tauri::ActivationPolicy::Regular
    } else {
        tauri::ActivationPolicy::Accessory
    });
    #[cfg(not(target_os = "macos"))]
    let _ = (app, regular);
}

pub fn open_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        set_regular_app(app, true);
        let _ = w.set_title(&t("settings.window_title"));
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
    // Showing the panel without activating the app keeps it over fullscreen apps.
    #[cfg(target_os = "macos")]
    if let Ok(panel) = tauri_nspanel::ManagerExt::get_webview_panel(app, "popover") {
        panel.show_and_make_key();
        return;
    }
    let _ = w.show();
    let _ = w.set_focus();
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let refresh = MenuItem::with_id(app, "refresh", t("tray.menu.refresh"), true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", t("tray.menu.settings"), true, None::<&str>)?;
    let updates = MenuItem::with_id(app, "updates", t("tray.menu.updates"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", t("tray.menu.quit"), true, None::<&str>)?;
    Menu::with_items(app, &[&refresh, &settings, &updates, &quit])
}

/// After the language changes in Settings: menu and window title in the new language.
pub fn relabel(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.set_title(&t("settings.window_title"));
    }
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = menu(app)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::new_owned(icon::render_idle(), ICON_PX, ICON_PX))
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
            "updates" => {
                // The settings window shows the version and runs the check itself.
                open_settings(app);
                let _ = app.emit_to("settings", "show-updates", ());
            }
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
        // A plain NSWindow never joins another app's fullscreen Space, whatever its
        // collection behavior; a non-activating panel does (like other menu-bar apps).
        #[cfg(target_os = "macos")]
        {
            use tauri_nspanel::objc2_app_kit::{NSWindowCollectionBehavior, NSWindowStyleMask};
            use tauri_nspanel::{PanelLevel, WebviewWindowExt};
            let panel = popover.to_panel::<PopoverPanel>()?;
            panel.set_level(PanelLevel::Floating.value());
            let _ = panel.add_style_mask(NSWindowStyleMask::NonactivatingPanel);
            panel.set_collection_behavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary,
            );
            panel.set_hides_on_deactivate(false);
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
        let handle = app.clone();
        // Closing settings only hides it; the app lives in the tray.
        settings.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = w.hide();
                set_regular_app(&handle, false);
            }
        });
    }
    Ok(())
}
