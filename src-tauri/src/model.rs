use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Light {
    Gray,
    Yellow,
    Green,
    Red,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NotifyPrefs {
    pub on_success: bool,
    pub on_failure: bool,
    pub on_start: bool,
}

impl Default for NotifyPrefs {
    fn default() -> Self {
        Self {
            on_success: true,
            on_failure: true,
            on_start: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WatchedWorkflow {
    /// "owner/name"
    pub repo: String,
    pub workflow_id: u64,
    pub workflow_name: String,
    /// None = any branch
    pub branch: Option<String>,
    /// None = any event
    pub event: Option<String>,
    #[serde(default)]
    pub notify: NotifyPrefs,
}

impl WatchedWorkflow {
    pub fn key(&self) -> String {
        format!(
            "{}#{}#{}#{}",
            self.repo,
            self.workflow_id,
            self.branch.as_deref().unwrap_or("*"),
            self.event.as_deref().unwrap_or("*")
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub watched: Vec<WatchedWorkflow>,
    #[serde(default = "default_true")]
    pub autostart: bool,
    #[serde(default = "default_true")]
    pub sound: bool,
    /// Set once the auto-filled branch/event filters from older versions were removed.
    #[serde(default)]
    pub filters_cleared: bool,
    /// Language code from Settings › General; None follows the system.
    #[serde(default)]
    pub language: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watched: Vec::new(),
            autostart: true,
            sound: true,
            filters_cleared: true,
            language: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RunSnapshot {
    pub id: u64,
    pub attempt: u32,
    pub status: String,
    pub conclusion: Option<String>,
    pub html_url: String,
    pub created_at: DateTime<Utc>,
    pub run_started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub head_branch: Option<String>,
    #[serde(default)]
    pub event: Option<String>,
    /// PR title or commit message GitHub shows for the run.
    #[serde(default)]
    pub title: Option<String>,
    /// Login of who triggered it.
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Card {
    pub key: String,
    pub repo: String,
    pub workflow_name: String,
    pub branch: Option<String>,
    pub light: Light,
    pub label: Option<String>,
    pub run_url: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress: Option<f32>,
    pub expected_secs: Option<u64>,
    pub step_label: Option<String>,
    /// Compact "done/total" position, e.g. "3/5".
    pub step_short: Option<String>,
    pub failed_step: Option<String>,
    pub error: Option<String>,
    /// Branch and event of the run shown, whatever the filter.
    pub run_branch: Option<String>,
    pub run_event: Option<String>,
    pub run_title: Option<String>,
    pub run_actor: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RepoGroup {
    pub repo: String,
    pub cards: Vec<Card>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TokenSource {
    Gh,
    Manual,
    Oauth,
    None,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AuthStatus {
    /// Source in use (first available in detection order).
    pub source: TokenSource,
    pub login: Option<String>,
    /// Which sources were detected, for the account panel.
    #[serde(default)]
    pub gh: bool,
    #[serde(default)]
    pub manual: bool,
    #[serde(default)]
    pub oauth: bool,
    /// Whether this build can offer "Iniciar sesión con GitHub".
    #[serde(default)]
    pub oauth_available: bool,
}

impl AuthStatus {
    pub fn none() -> Self {
        Self {
            source: TokenSource::None,
            login: None,
            gh: false,
            manual: false,
            oauth: false,
            oauth_available: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DashboardState {
    pub groups: Vec<RepoGroup>,
    pub auth: AuthStatus,
    pub banner: Option<String>,
    pub banner_url: Option<String>,
    pub last_updated: Option<DateTime<Utc>>,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            auth: AuthStatus::none(),
            banner: None,
            banner_url: None,
            last_updated: None,
        }
    }
}
