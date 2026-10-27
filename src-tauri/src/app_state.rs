use crate::auth::{self, KeyringStore};
use crate::github::{GithubApi, HttpClient};
use crate::model::{AuthStatus, Config, DashboardState, TokenSource};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::Notify;

/// Shared state managed by Tauri (`app.manage`).
pub struct AppCtx {
    pub cfg: RwLock<Config>,
    pub api: Arc<HttpClient>,
    pub store: KeyringStore,
    pub dashboard: RwLock<DashboardState>,
    pub auth: RwLock<AuthStatus>,
    /// Wakes the poller immediately ("Actualizar ahora", config or token changed).
    pub wake: Arc<Notify>,
    pub config_dir: PathBuf,
}

fn gh_token() -> Option<String> {
    let path = std::env::var("PATH").ok();
    auth::gh_token(&auth::gh_candidates(path.as_deref()))
}

impl AppCtx {
    /// Re-resolves the token (manual > gh > none), pushes it into the client and looks up
    /// the login. Returns the new status.
    pub async fn refresh_auth(&self) -> AuthStatus {
        let (token, source) = auth::resolve(&self.store, &gh_token);
        self.api.set_token(token);
        let login = match source {
            TokenSource::None => None,
            _ => self.api.me().await.ok(),
        };
        let status = AuthStatus { source, login };
        *self.auth.write().unwrap() = status.clone();
        status
    }
}
