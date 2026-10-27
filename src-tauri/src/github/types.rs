use crate::model::RunSnapshot;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Repo {
    pub full_name: String,
    pub private: bool,
    pub default_branch: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Step {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub number: u32,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Job {
    pub status: String,
    pub conclusion: Option<String>,
    #[serde(default)]
    pub steps: Vec<Step>,
}

#[derive(Deserialize)]
pub(crate) struct ApiRun {
    pub id: u64,
    pub run_attempt: Option<u32>,
    pub status: Option<String>,
    pub conclusion: Option<String>,
    pub html_url: String,
    pub created_at: DateTime<Utc>,
    pub run_started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl From<ApiRun> for RunSnapshot {
    fn from(r: ApiRun) -> Self {
        RunSnapshot {
            id: r.id,
            attempt: r.run_attempt.unwrap_or(1),
            status: r.status.unwrap_or_else(|| "queued".into()),
            conclusion: r.conclusion,
            html_url: r.html_url,
            created_at: r.created_at,
            run_started_at: r.run_started_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct RunsPage {
    pub workflow_runs: Vec<ApiRun>,
}

#[derive(Deserialize)]
pub(crate) struct WorkflowsPage {
    pub workflows: Vec<Workflow>,
}

#[derive(Deserialize)]
pub(crate) struct JobsPage {
    pub jobs: Vec<Job>,
}

#[derive(Deserialize)]
pub(crate) struct User {
    pub login: String,
}
