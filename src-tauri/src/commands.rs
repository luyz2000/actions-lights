use crate::app_state::AppCtx;
use crate::auth::SecretStore;
use crate::config;
use crate::github::{GITHUB_API, GithubApi, HttpClient, Repo, Workflow};
use crate::model::{AuthStatus, Config, DashboardState};
use crate::tray;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

pub fn apply_autostart(app: &AppHandle, enabled: bool) {
    // Dev builds would register target/debug as a login item.
    if cfg!(debug_assertions) {
        return;
    }
    let launcher = app.autolaunch();
    let _ = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
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
    HttpClient::new(GITHUB_API, Some(token.clone()))
        .me()
        .await
        .map_err(|e| e.message())?;
    ctx.manual.set(&token)?;
    let status = ctx.refresh_auth().await;
    ctx.wake.notify_one();
    Ok(status)
}

#[tauri::command]
pub async fn clear_token(ctx: State<'_, AppCtx>) -> Result<AuthStatus, String> {
    ctx.manual.clear()?;
    let status = ctx.refresh_auth().await;
    ctx.wake.notify_one();
    Ok(status)
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) {
    tray::open_settings(&app);
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

/// Local checkout → its GitHub repo (via the origin remote).
#[tauri::command]
pub async fn repo_from_folder(ctx: State<'_, AppCtx>, path: String) -> Result<Repo, String> {
    let full = crate::git_remote::repo_from_folder(std::path::Path::new(&path))?;
    let api = ctx.api.clone();
    api.get_repo(&full).await.map_err(|e| e.message())
}

/// Starts "Iniciar sesión con GitHub": returns the code to show, opens the verification
/// page and polls in the background. Emits `auth-changed` on success or `oauth-failed`.
#[tauri::command]
pub async fn oauth_start(app: AppHandle) -> Result<crate::oauth::DeviceCode, String> {
    use crate::oauth::{self, GITHUB_WEB, PollResult};
    let client_id =
        oauth::client_id().ok_or("Esta versión no incluye inicio de sesión con GitHub")?;
    let code = oauth::request_code(GITHUB_WEB, client_id).await?;
    let _ = app
        .opener()
        .open_url(code.verification_uri.clone(), None::<&str>);

    let device_code = code.device_code.clone();
    let mut interval = code.interval.max(5);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(code.expires_in);
    tauri::async_runtime::spawn(async move {
        let fail = |msg: &str| {
            let _ = app.emit("oauth-failed", msg.to_string());
        };
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            if std::time::Instant::now() > deadline {
                return fail("El código caducó, vuelve a intentarlo");
            }
            match oauth::poll_once(GITHUB_WEB, client_id, &device_code).await {
                Ok(PollResult::Pending) => {}
                Ok(PollResult::SlowDown) => interval += 5,
                Ok(PollResult::Token(token)) => {
                    let ctx = app.state::<AppCtx>();
                    if let Err(e) = ctx.oauth.set(&token) {
                        return fail(&e);
                    }
                    let status = ctx.refresh_auth().await;
                    ctx.wake.notify_one();
                    let _ = app.emit("auth-changed", status);
                    return;
                }
                Ok(PollResult::Denied) => return fail("Inicio de sesión cancelado"),
                Ok(PollResult::Expired) => return fail("El código caducó, vuelve a intentarlo"),
                Err(e) => return fail(&e),
            }
        }
    });
    Ok(code)
}

#[tauri::command]
pub async fn oauth_logout(ctx: State<'_, AppCtx>) -> Result<AuthStatus, String> {
    ctx.oauth.clear()?;
    let status = ctx.refresh_auth().await;
    ctx.wake.notify_one();
    Ok(status)
}
