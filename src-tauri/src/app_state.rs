use crate::auth::{self, KeyringStore, SecretStore};
use crate::github::{GithubApi, HttpClient};
use crate::model::{AuthStatus, Config, DashboardState, TokenSource};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::Notify;

/// Shared state managed by Tauri (`app.manage`).
pub struct AppCtx {
    pub cfg: RwLock<Config>,
    pub api: Arc<HttpClient>,
    pub manual: KeyringStore,
    pub oauth: KeyringStore,
    pub dashboard: RwLock<DashboardState>,
    pub auth: RwLock<AuthStatus>,
    /// Wakes the poller immediately ("Actualizar ahora", config or token changed).
    pub wake: Arc<Notify>,
    pub config_dir: PathBuf,
    /// Newer release found by the updater, if any.
    pub update: RwLock<Option<crate::updates::UpdateInfo>>,
}

fn gh_token() -> Option<String> {
    let path = std::env::var("PATH").ok();
    auth::gh_token(&auth::gh_candidates(path.as_deref()))
}

impl AppCtx {
    /// Re-resolves the token (gh > manual > OAuth), pushes it into the client and looks up
    /// the login. Returns the new status.
    pub async fn refresh_auth(&self) -> AuthStatus {
        let gh = gh_token();
        let (token, source) = auth::resolve(&self.manual, &self.oauth, &|| gh.clone());
        self.api.set_token(token);
        let login = match source {
            TokenSource::None => None,
            _ => self.api.me().await.ok(),
        };
        let status = AuthStatus {
            source,
            login,
            gh: gh.is_some(),
            manual: self.manual.get().is_some(),
            oauth: self.oauth.get().is_some(),
            oauth_available: crate::oauth::client_id().is_some(),
        };
        *self.auth.write().unwrap() = status.clone();
        status
    }
}
