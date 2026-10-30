mod app_state;
mod auth;
mod commands;
mod config;
mod git_remote;
mod github;
mod icon;
mod interval;
mod model;
mod notify_rules;
mod oauth;
mod poller;
mod progress;
mod status;
mod tray;
mod tray_text;

use app_state::AppCtx;
use auth::KeyringStore;
use github::{GITHUB_API, HttpClient};
use model::DashboardState;
use std::sync::{Arc, RwLock};
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_notification::{NotificationExt, PermissionState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard,
            commands::refresh_now,
            commands::get_config,
            commands::save_config,
            commands::list_repos,
            commands::get_repo,
            commands::list_workflows,
            commands::auth_status,
            commands::set_token,
            commands::clear_token,
            commands::open_url,
            commands::repo_from_folder,
            commands::oauth_start,
            commands::oauth_logout,
            commands::open_settings,
            commands::quit,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let config_dir = app.path().app_config_dir()?;
            let cfg = config::load(&config_dir);
            let first_run = cfg.watched.is_empty();
            let autostart = cfg.autostart;

            app.manage(AppCtx {
                cfg: RwLock::new(cfg),
                api: Arc::new(HttpClient::new(GITHUB_API, None)),
                manual: KeyringStore {
                    account: auth::MANUAL_ACCOUNT,
                },
                oauth: KeyringStore {
                    account: auth::OAUTH_ACCOUNT,
                },
                dashboard: RwLock::new(DashboardState::default()),
                auth: RwLock::new(model::AuthStatus::none()),
                wake: Arc::new(tokio::sync::Notify::new()),
                config_dir,
            });

            let handle = app.handle().clone();
            tray::build(&handle)?;
            commands::apply_autostart(&handle, autostart);
            if first_run {
                tray::open_settings(&handle);
            }
            if app.notification().permission_state().ok() != Some(PermissionState::Granted) {
                let _ = app.notification().request_permission();
            }

            tauri::async_runtime::spawn(async move {
                handle.state::<AppCtx>().refresh_auth().await;
                poller::start(handle);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
