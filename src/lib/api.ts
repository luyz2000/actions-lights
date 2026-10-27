import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Light = "gray" | "yellow" | "green" | "red";
export type TokenSource = "manual" | "gh" | "none";

export interface Card {
  key: string;
  repo: string;
  workflow_name: string;
  branch: string | null;
  light: Light;
  label: string | null;
  run_url: string | null;
  started_at: string | null;
  completed_at: string | null;
  progress: number | null;
  expected_secs: number | null;
  step_label: string | null;
  failed_step: string | null;
  error: string | null;
}

export interface RepoGroup {
  repo: string;
  cards: Card[];
}

export interface AuthStatus {
  source: TokenSource;
  login: string | null;
}

export interface DashboardState {
  groups: RepoGroup[];
  auth: AuthStatus;
  banner: string | null;
  last_updated: string | null;
}

export interface NotifyPrefs {
  on_success: boolean;
  on_failure: boolean;
  on_start: boolean;
}

export interface WatchedWorkflow {
  repo: string;
  workflow_id: number;
  workflow_name: string;
  branch: string | null;
  event: string | null;
  notify: NotifyPrefs;
}

export interface Config {
  watched: WatchedWorkflow[];
  autostart: boolean;
}

export interface Repo {
  full_name: string;
  private: boolean;
  default_branch: string;
}

export interface Workflow {
  id: number;
  name: string;
  path: string;
  state: string;
}

export const api = {
  getDashboard: () => invoke<DashboardState>("get_dashboard"),
  refreshNow: () => invoke<void>("refresh_now"),
  getConfig: () => invoke<Config>("get_config"),
  saveConfig: (cfg: Config) => invoke<void>("save_config", { cfg }),
  listRepos: () => invoke<Repo[]>("list_repos"),
  getRepo: (full: string) => invoke<Repo>("get_repo", { full }),
  listWorkflows: (repo: string) => invoke<Workflow[]>("list_workflows", { repo }),
  authStatus: () => invoke<AuthStatus>("auth_status"),
  setToken: (token: string) => invoke<AuthStatus>("set_token", { token }),
  clearToken: () => invoke<AuthStatus>("clear_token"),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  openSettings: () => invoke<void>("open_settings"),
  quit: () => invoke<void>("quit"),
};

export function onDashboard(cb: (d: DashboardState) => void): Promise<UnlistenFn> {
  return listen<DashboardState>("dashboard", (e) => cb(e.payload));
}
