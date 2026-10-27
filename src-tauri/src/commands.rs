use crate::app_state::AppCtx;
use crate::config;
use crate::github::{GithubApi, HttpClient, Repo, Workflow, GITHUB_API};
use crate::auth::SecretStore;
use crate::model::{AuthStatus, Config, DashboardState};
use crate::tray;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

pub fn apply_autostart(app: &AppHandle, enabled: bool) {
    // Dev builds would register target/debug as a login item.
    if cfg!(debug_assertions) {
        return;
    }
    let launcher = app.autolaunch();
    let _ = if enabled { launcher.enable() } else { launcher.disable() };
}

#[tauri::command]
pub fn get_dashboard(ctx: State<'_, AppCtx>) -> DashboardState {
    ctx.dashboard.read().unwrap().clone()
}

#[tauri::command]
pub async fn refresh_now(ctx: State<'_, AppCtx>) -> Result<(), String> {
    ctx.refresh_auth().await;
    ctx.wake.notify_one();
    Ok(())
}

#[tauri::command]
pub fn get_config(ctx: State<'_, AppCtx>) -> Config {
    ctx.cfg.read().unwrap().clone()
}

#[tauri::command]
pub fn save_config(app: AppHandle, ctx: State<'_, AppCtx>, cfg: Config) -> Result<(), String> {
    config::save(&ctx.config_dir, &cfg).map_err(|e| e.to_string())?;
    apply_autostart(&app, cfg.autostart);
    *ctx.cfg.write().unwrap() = cfg;
    ctx.wake.notify_one();
    Ok(())
}

#[tauri::command]
pub async fn list_repos(ctx: State<'_, AppCtx>) -> Result<Vec<Repo>, String> {
    let api = ctx.api.clone();
    api.list_repos().await.map_err(|e| e.message())
}

#[tauri::command]
pub async fn get_repo(ctx: State<'_, AppCtx>, full: String) -> Result<Repo, String> {
    let api = ctx.api.clone();
    api.get_repo(full.trim()).await.map_err(|e| e.message())
}

#[tauri::command]
pub async fn list_workflows(ctx: State<'_, AppCtx>, repo: String) -> Result<Vec<Workflow>, String> {
    let api = ctx.api.clone();
    api.list_workflows(&repo).await.map_err(|e| e.message())
}

#[tauri::command]
pub fn auth_status(ctx: State<'_, AppCtx>) -> AuthStatus {
    ctx.auth.read().unwrap().clone()
}

/// Validates the token against GitHub before storing it in the keychain.
#[tauri::command]
pub async fn set_token(ctx: State<'_, AppCtx>, token: String) -> Result<AuthStatus, String> {
    let token = token.trim().to_string();
    HttpClient::new(GITHUB_API, Some(token.clone())).me().await.map_err(|e| e.message())?;
    ctx.store.set(&token)?;
    let status = ctx.refresh_auth().await;
    ctx.wake.notify_one();
    Ok(status)
}

#[tauri::command]
pub async fn clear_token(ctx: State<'_, AppCtx>) -> Result<AuthStatus, String> {
    ctx.store.clear()?;
    let status = ctx.refresh_auth().await;
    ctx.wake.notify_one();
    Ok(status)
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) {
    tray::open_settings(&app);
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}
